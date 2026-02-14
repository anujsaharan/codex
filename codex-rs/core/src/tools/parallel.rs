use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use std::time::Instant;

use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::watch;
use tokio_util::either::Either;
use tokio_util::sync::CancellationToken;
use tokio_util::task::AbortOnDropHandle;
use tracing::Instrument;
use tracing::instrument;
use tracing::trace_span;

use crate::codex::Session;
use crate::codex::TurnContext;
use crate::error::CodexErr;
use crate::function_tool::FunctionCallError;
use crate::tools::context::SharedTurnDiffTracker;
use crate::tools::context::ToolPayload;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolRouter;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use serde_json::Map as JsonMap;
use serde_json::Value as JsonValue;

static TOOL_RESULT_CACHE_ENABLED: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("CODEX_PERF_DISABLE_TOOL_RESULT_CACHE").is_none());
static TOOL_RESULT_CACHE_MAX_ENTRIES: LazyLock<usize> = LazyLock::new(|| {
    std::env::var("CODEX_PERF_TOOL_RESULT_CACHE_MAX_ENTRIES")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(64)
});
static TOOL_RESULT_CACHE_TTL_SECS: LazyLock<u64> = LazyLock::new(|| {
    std::env::var("CODEX_PERF_TOOL_RESULT_CACHE_TTL_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(120)
});

#[derive(Clone)]
pub(crate) struct ToolCallRuntime {
    router: Arc<ToolRouter>,
    session: Arc<Session>,
    turn_context: Arc<TurnContext>,
    tracker: SharedTurnDiffTracker,
    parallel_execution: Arc<RwLock<()>>,
    turn_result_cache: Arc<Mutex<HashMap<String, ResponseInputItem>>>,
    turn_inflight_cache: Arc<Mutex<HashMap<String, watch::Receiver<Option<ResponseInputItem>>>>>,
}

impl ToolCallRuntime {
    pub(crate) fn new(
        router: Arc<ToolRouter>,
        session: Arc<Session>,
        turn_context: Arc<TurnContext>,
        tracker: SharedTurnDiffTracker,
    ) -> Self {
        Self {
            router,
            session,
            turn_context,
            tracker,
            parallel_execution: Arc::new(RwLock::new(())),
            turn_result_cache: Arc::new(Mutex::new(HashMap::new())),
            turn_inflight_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[instrument(level = "trace", skip_all, fields(call = ?call))]
    pub(crate) fn handle_tool_call(
        self,
        call: ToolCall,
        cancellation_token: CancellationToken,
    ) -> impl std::future::Future<Output = Result<ResponseInputItem, CodexErr>> {
        let supports_parallel = self.router.tool_supports_parallel(&call.tool_name);

        let router = Arc::clone(&self.router);
        let session = Arc::clone(&self.session);
        let turn = Arc::clone(&self.turn_context);
        let tracker = Arc::clone(&self.tracker);
        let lock = Arc::clone(&self.parallel_execution);
        let turn_result_cache = Arc::clone(&self.turn_result_cache);
        let turn_inflight_cache = Arc::clone(&self.turn_inflight_cache);
        let started = Instant::now();
        let cache_key = tool_call_cache_key(&call);
        let supports_turn_cache = tool_supports_turn_cache(&call.tool_name);
        let supports_session_cache = tool_supports_session_cache(&call.tool_name);
        let tool_cache_ttl = Duration::from_secs(*TOOL_RESULT_CACHE_TTL_SECS);

        let dispatch_span = trace_span!(
            "dispatch_tool_call",
            otel.name = call.tool_name.as_str(),
            tool_name = call.tool_name.as_str(),
            call_id = call.call_id.as_str(),
            aborted = false,
        );

        let handle: AbortOnDropHandle<Result<ResponseInputItem, FunctionCallError>> =
            AbortOnDropHandle::new(tokio::spawn(async move {
                let mut shared_result_sender: Option<watch::Sender<Option<ResponseInputItem>>> =
                    None;

                if supports_turn_cache
                    && let Some(cached) = turn_result_cache.lock().await.get(&cache_key).cloned()
                {
                    tracing::debug!(
                        tool_name = %call.tool_name,
                        call_id = %call.call_id,
                        "returning cached tool result from current turn"
                    );
                    return Ok(remap_response_call_id(cached, &call.call_id));
                }
                if supports_session_cache
                    && *TOOL_RESULT_CACHE_ENABLED
                    && let Some(cached) = session
                        .get_cached_tool_result(&cache_key, tool_cache_ttl)
                        .await
                {
                    tracing::debug!(
                        tool_name = %call.tool_name,
                        call_id = %call.call_id,
                        "returning cached tool result from session cache"
                    );
                    if supports_turn_cache {
                        turn_result_cache
                            .lock()
                            .await
                            .insert(cache_key.clone(), cached.clone());
                    }
                    return Ok(remap_response_call_id(cached, &call.call_id));
                }

                if supports_turn_cache {
                    let mut waiting_on_existing: Option<
                        watch::Receiver<Option<ResponseInputItem>>,
                    > = None;
                    {
                        let mut inflight = turn_inflight_cache.lock().await;
                        if let Some(existing) = inflight.get(&cache_key) {
                            waiting_on_existing = Some(existing.clone());
                        } else {
                            let (sender, receiver) = watch::channel(None::<ResponseInputItem>);
                            inflight.insert(cache_key.clone(), receiver);
                            shared_result_sender = Some(sender);
                        }
                    }

                    if let Some(mut receiver) = waiting_on_existing {
                        tracing::debug!(
                            tool_name = %call.tool_name,
                            call_id = %call.call_id,
                            "waiting for in-flight tool result"
                        );

                        tokio::select! {
                            _ = cancellation_token.cancelled() => {
                                let secs = started.elapsed().as_secs_f32().max(0.1);
                                dispatch_span.record("aborted", true);
                                return Ok(Self::aborted_response(&call, secs));
                            }
                            _ = receiver.changed() => {}
                        }

                        if let Some(cached) = receiver.borrow().clone() {
                            return Ok(remap_response_call_id(cached, &call.call_id));
                        }
                        if let Some(cached) =
                            turn_result_cache.lock().await.get(&cache_key).cloned()
                        {
                            return Ok(remap_response_call_id(cached, &call.call_id));
                        }
                    }
                }

                let mut cancelled = false;
                let result = tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        cancelled = true;
                        let secs = started.elapsed().as_secs_f32().max(0.1);
                        dispatch_span.record("aborted", true);
                        Ok(Self::aborted_response(&call, secs))
                    },
                    res = async {
                        let _guard = if supports_parallel {
                            Either::Left(lock.read().await)
                        } else {
                            Either::Right(lock.write().await)
                        };

                        let dispatched = router
                            .dispatch_tool_call(
                                Arc::clone(&session),
                                turn,
                                tracker,
                                call.clone(),
                                crate::tools::router::ToolCallSource::Direct,
                            )
                            .instrument(dispatch_span.clone())
                            .await;

                        if let Ok(response) = dispatched.as_ref()
                            && should_cache_tool_response(response)
                        {
                            if supports_turn_cache {
                                turn_result_cache
                                    .lock()
                                    .await
                                    .insert(cache_key.clone(), response.clone());
                            }
                            if supports_session_cache && *TOOL_RESULT_CACHE_ENABLED {
                                session
                                    .put_cached_tool_result(
                                        cache_key.clone(),
                                        response.clone(),
                                        *TOOL_RESULT_CACHE_MAX_ENTRIES,
                                    )
                                    .await;
                            }
                        }

                        dispatched
                    } => res
                };

                if let Some(sender) = shared_result_sender {
                    let shareable = if cancelled {
                        None
                    } else {
                        result.as_ref().ok().and_then(|response| {
                            should_cache_tool_response(response).then_some(response.clone())
                        })
                    };
                    let _ = sender.send(shareable);
                    turn_inflight_cache.lock().await.remove(&cache_key);
                }

                result
            }));

        async move {
            match handle.await {
                Ok(Ok(response)) => Ok(response),
                Ok(Err(FunctionCallError::Fatal(message))) => Err(CodexErr::Fatal(message)),
                Ok(Err(other)) => Err(CodexErr::Fatal(other.to_string())),
                Err(err) => Err(CodexErr::Fatal(format!(
                    "tool task failed to receive: {err:?}"
                ))),
            }
        }
        .in_current_span()
    }
}

impl ToolCallRuntime {
    fn aborted_response(call: &ToolCall, secs: f32) -> ResponseInputItem {
        match &call.payload {
            ToolPayload::Custom { .. } => ResponseInputItem::CustomToolCallOutput {
                call_id: call.call_id.clone(),
                output: Self::abort_message(call, secs),
            },
            ToolPayload::Mcp { .. } => ResponseInputItem::McpToolCallOutput {
                call_id: call.call_id.clone(),
                result: Err(Self::abort_message(call, secs)),
            },
            _ => ResponseInputItem::FunctionCallOutput {
                call_id: call.call_id.clone(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(Self::abort_message(call, secs)),
                    ..Default::default()
                },
            },
        }
    }

    fn abort_message(call: &ToolCall, secs: f32) -> String {
        match call.tool_name.as_str() {
            "shell" | "container.exec" | "local_shell" | "shell_command" | "unified_exec" => {
                format!("Wall time: {secs:.1} seconds\naborted by user")
            }
            _ => format!("aborted by user after {secs:.1}s"),
        }
    }
}

fn tool_supports_turn_cache(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "search_query"
            | "image_query"
            | "weather"
            | "sports"
            | "finance"
            | "time"
            | "list_mcp_resources"
            | "list_mcp_resource_templates"
            | "read_mcp_resource"
            | "search_tool_bm25"
            | "read_file"
            | "list_dir"
            | "grep_files"
            | "view_image"
    )
}

fn tool_supports_session_cache(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "search_query"
            | "image_query"
            | "weather"
            | "sports"
            | "finance"
            | "time"
            | "list_mcp_resources"
            | "list_mcp_resource_templates"
            | "read_mcp_resource"
            | "search_tool_bm25"
    )
}

fn should_cache_tool_response(response: &ResponseInputItem) -> bool {
    match response {
        ResponseInputItem::FunctionCallOutput { output, .. } => output.success.unwrap_or(true),
        ResponseInputItem::McpToolCallOutput { result, .. } => result.is_ok(),
        ResponseInputItem::CustomToolCallOutput { .. } => true,
        ResponseInputItem::Message { .. } => false,
    }
}

fn remap_response_call_id(response: ResponseInputItem, call_id: &str) -> ResponseInputItem {
    match response {
        ResponseInputItem::FunctionCallOutput { output, .. } => {
            ResponseInputItem::FunctionCallOutput {
                call_id: call_id.to_string(),
                output,
            }
        }
        ResponseInputItem::McpToolCallOutput { result, .. } => {
            ResponseInputItem::McpToolCallOutput {
                call_id: call_id.to_string(),
                result,
            }
        }
        ResponseInputItem::CustomToolCallOutput { output, .. } => {
            ResponseInputItem::CustomToolCallOutput {
                call_id: call_id.to_string(),
                output,
            }
        }
        message @ ResponseInputItem::Message { .. } => message,
    }
}

fn tool_call_cache_key(call: &ToolCall) -> String {
    match &call.payload {
        ToolPayload::Function { arguments } => {
            format!("{}|fn|{}", call.tool_name, canonical_json_or_raw(arguments))
        }
        ToolPayload::Custom { input } => {
            format!("{}|custom|{}", call.tool_name, input.trim())
        }
        ToolPayload::LocalShell { params } => format!(
            "{}|local_shell|{}|{}|{}",
            call.tool_name,
            params.command.join("\u{1f}"),
            params.workdir.as_deref().unwrap_or_default(),
            params.timeout_ms.unwrap_or_default()
        ),
        ToolPayload::Mcp {
            server,
            tool,
            raw_arguments,
        } => format!(
            "{}|mcp|{}|{}|{}",
            call.tool_name,
            server,
            tool,
            canonical_json_or_raw(raw_arguments)
        ),
    }
}

fn canonical_json_or_raw(raw: &str) -> String {
    serde_json::from_str::<JsonValue>(raw)
        .map(|value| canonicalize_json_value(&value).to_string())
        .unwrap_or_else(|_| raw.trim().to_string())
}

fn canonicalize_json_value(value: &JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(object) => {
            let mut keys: Vec<_> = object.keys().cloned().collect();
            keys.sort();
            let mut canonical = JsonMap::new();
            for key in keys {
                if let Some(child) = object.get(&key) {
                    canonical.insert(key, canonicalize_json_value(child));
                }
            }
            JsonValue::Object(canonical)
        }
        JsonValue::Array(items) => {
            JsonValue::Array(items.iter().map(canonicalize_json_value).collect())
        }
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::FunctionCallOutputBody;
    use codex_protocol::models::FunctionCallOutputPayload;
    use pretty_assertions::assert_eq;

    #[test]
    fn remap_response_call_id_rewrites_function_outputs() {
        let response = ResponseInputItem::FunctionCallOutput {
            call_id: "old".to_string(),
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::Text("ok".to_string()),
                success: Some(true),
            },
        };

        let remapped = remap_response_call_id(response, "new");
        assert_eq!(
            remapped,
            ResponseInputItem::FunctionCallOutput {
                call_id: "new".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text("ok".to_string()),
                    success: Some(true),
                },
            }
        );
    }

    #[test]
    fn tool_call_cache_key_canonicalizes_json_argument_order() {
        let call_a = ToolCall {
            tool_name: "search_query".to_string(),
            call_id: "call-a".to_string(),
            payload: ToolPayload::Function {
                arguments: r#"{"q":"rain","domains":["weather.com"],"recency":1}"#.to_string(),
            },
        };
        let call_b = ToolCall {
            tool_name: "search_query".to_string(),
            call_id: "call-b".to_string(),
            payload: ToolPayload::Function {
                arguments: r#"{"recency":1,"domains":["weather.com"],"q":"rain"}"#.to_string(),
            },
        };

        assert_eq!(tool_call_cache_key(&call_a), tool_call_cache_key(&call_b));
    }

    #[test]
    fn cache_policy_marks_weather_as_session_cacheable() {
        assert!(tool_supports_turn_cache("weather"));
        assert!(tool_supports_session_cache("weather"));
        assert!(!tool_supports_session_cache("read_file"));
    }
}

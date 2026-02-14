# PR 01: Core Tool Cache + Event Pipeline Fast Path

Branch: `upstream-pr/01-core-tool-cache-event-pipeline`  
Commit: `ada81ffdf`  
Compare: <https://github.com/openai/codex/compare/main...anujsaharan:upstream-pr/01-core-tool-cache-event-pipeline>

## What Changed

Files:

1. `codex-rs/core/src/tools/parallel.rs`
2. `codex-rs/core/src/state/session.rs`
3. `codex-rs/core/src/codex.rs`
4. `codex-rs/core/src/state/service.rs`

Behavioral deltas:

1. Added turn-local cache for safe read-only tools.
2. Added session cache for selected remote read-only tools.
3. Added in-flight single-flight dedupe for identical tool calls in one turn.
4. Added canonical tool-call cache-key generation (JSON argument order-insensitive).
5. Added session-state cache storage with TTL and max-entry eviction.
6. Reused MCP tool list for the turn so model/tool build path does not repeat `list_all_tools()`.
7. Switched `send_event_raw()` to send-to-subscribers first, then persist selected events.

## Why This Solution

The latency pain came from repeated work in three places:

1. Duplicate tool dispatches for semantically identical calls.
2. Duplicate turn-internal metadata fetches (MCP tools list).
3. Event-path overhead in the hot streaming loop.

Given existing architecture, caching at `ToolCallRuntime` is the highest leverage point because every model-driven tool call already converges there.

## System Design Impact

1. Introduces a lightweight memoization layer in core execution path.
2. Adds single-flight semantics for identical concurrent calls.
3. Adds session state as a short-lived read-through cache for tool outputs.
4. Keeps cacheability policy explicit via allowlists; no dynamic tool purity inference.

## Concrete Gains (What Resulted In What)

1. Same-turn repeated identical call now returns from map lookup instead of re-dispatch.
   Result: removes network/tool latency for repeats in the same turn.
2. Cross-turn repeated identical call within TTL now returns from session cache.
   Result: follow-up prompts that re-ask same data avoid re-fetch.
3. Identical concurrent tool calls now collapse from `N` dispatches to `1` dispatch.
   Result: lower provider/tool load and lower tail latency under parallel tool fanout.
4. MCP tool listing reused in-turn.
   Result: one less `list_all_tools()` call per turn in affected flows.
5. Cache key generation now happens only for cache-eligible tools.
   Result: avoids canonicalization overhead on non-cacheable tool calls.

## Costs / Tradeoffs

1. Cache policy is allowlist-based and must be maintained when new tools are added.
2. Session cache can serve stale data within TTL window.
3. More runtime state and locking complexity in tool dispatch path.
4. Fixed defaults after cleanup (`TTL=120s`, `capacity=64`) reduce runtime tunability.

## Interaction With Existing Code

1. Works through existing `SessionState` lock and `ToolCallRuntime` dispatch flow.
2. Does not require protocol changes.
3. Uses existing `ResponseInputItem` and remaps `call_id` per invocation so downstream consumers remain consistent.

## What May Break

1. If a tool was misclassified as cache-safe, stale/incorrect replay can happen.
2. If a caller relies on side effects from repeated identical tool calls, dedupe/cache changes that behavior.
3. Future tool schema changes may reduce cache key stability if canonicalization assumptions are violated.

## Shoe-horned vs Native Design

Shoe-horned:

1. Cache safety currently encoded as string allowlists because there is no first-class tool purity metadata.

Native:

1. State ownership in `SessionState` and dispatch centralization in `ToolCallRuntime` were already good insertion points.

## Validation Done

1. `cargo test -p codex-core --lib --quiet`
2. Focused cache tests in `session.rs` and `parallel.rs`.
3. Whole-repo follow-up tests via downstream PRs (exec/tui/sdk) to ensure no protocol regressions.

## Post-Review Cleanup Delta

1. Removed unnecessary cache-key computation for non-cacheable calls.
2. Added concise comments for single-flight/cache fast path to reduce maintainer cognitive load.
3. Kept behavior identical for cacheable flows; all core tests remained green.

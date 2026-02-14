# Performance Modernization PR Pack (Fork-Local, Upstream-Ready)

This document set is the PR metadata bundle you asked for:

- what changed
- why that solution was chosen
- system-design impact
- concrete gains and concrete costs
- what may break
- validation/ablation results

These docs are written against the code currently on this fork's `main` as of commit `e9b68b7d6`.

## Scope In Main

Included in current `main`:

1. Core runtime caching + event path improvements
2. `codex-exec` event processing + output batching improvements
3. TUI status normalization improvements
4. TypeScript SDK stream parser improvement

Explicitly removed from tracked git at your request:

1. `codex-web/*`
2. `scripts/perf_ablation_exec_stream.mjs`
3. `scripts/perf_ablation_web_bridge.mjs`

## Branches You Can Upstream Later

Each branch below is single-commit against `upstream/main` and pushed to your fork:

1. `upstream-pr/01-core-tool-cache-event-pipeline`
   Link: <https://github.com/openai/codex/compare/main...anujsaharan:upstream-pr/01-core-tool-cache-event-pipeline>
2. `upstream-pr/02-exec-jsonl-streaming`
   Link: <https://github.com/openai/codex/compare/main...anujsaharan:upstream-pr/02-exec-jsonl-streaming>
3. `upstream-pr/03-tui-status-normalization`
   Link: <https://github.com/openai/codex/compare/main...anujsaharan:upstream-pr/03-tui-status-normalization>
4. `upstream-pr/04-sdk-stream-parser`
   Link: <https://github.com/openai/codex/compare/main...anujsaharan:upstream-pr/04-sdk-stream-parser>

## PR Writeups

1. `docs/perf-upstream-pack/pr-01-core-tool-cache-event-pipeline.md`
2. `docs/perf-upstream-pack/pr-02-exec-jsonl-streaming.md`
3. `docs/perf-upstream-pack/pr-03-tui-status-normalization.md`
4. `docs/perf-upstream-pack/pr-04-sdk-stream-parser.md`
5. `docs/perf-upstream-pack/ablation-results.md`

## Quick "Resulted In What" Summary

1. Repeated identical tool calls now resolve from memory instead of re-dispatching to remote tools, within a 120s session cache window and always within-turn.
2. Concurrent duplicate tool calls are single-flight deduped (`N` callers collapse to `1` dispatch + `N-1` waiters).
3. Turn setup no longer lists MCP tools twice in the same turn (single list reused).
4. `codex-exec` now emits agent message deltas as incremental item updates and batches output writes per processed event.
5. TUI status headers now normalize machine JSON payloads into readable human labels.
6. TS SDK parser now uses chunk-buffer line parsing (no readline mode split), reducing parser overhead and improving fragmented-chunk handling.

## Concrete Costs / Constraints Introduced

1. Tool caching is allowlist-based because the codebase has no formal "tool purity" contract.
2. Session cache defaults are fixed in code (120s TTL, 64 entries) after cleanup of dev-only env toggles.
3. SDK parser behavior is now single-path (chunk parser only); no runtime parser toggle remains.
4. `codex-exec` flush behavior is now fixed to always flush after each processed aggregate block.


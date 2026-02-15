# Branch Notes: upstream-pr/01-core-tool-cache-event-pipeline

## Intent
Core runtime duplicate-work elimination and event-path tightening.

## Scope
- Adds turn + session tool result cache for safe read-only tool allowlists.\n- Adds single-flight dedupe for identical in-flight tool calls.\n- Reuses MCP tool list during turn tool-building.\n- Adds selective event persistence mode wiring.

## Why This Shape
- Latency pain centered on repeated tool dispatch + per-turn repeated MCP tool listing.\n- Existing architecture naturally centralizes this in ToolCallRuntime and SessionState.

## Concrete Gains
- Repeat tool calls can return from memory instead of re-dispatching.\n- Concurrent duplicate calls collapse to one dispatch.\n- Avoided one extra MCP list-all-tools call in app-enabled turns.

## Costs / Tradeoffs
- Bounded staleness window (session cache TTL).\n- Cache allowlist requires maintenance as new tools are added.\n- Extra runtime state + synchronization in tool dispatch path.

## Break Risk / Compatibility Risk
- Misclassified cache-safe tool could replay stale/incorrect output.\n- Callers relying on repeated side effects from identical calls may observe changed behavior.

## Validation Run
- cargo test -p codex-core --lib --quiet

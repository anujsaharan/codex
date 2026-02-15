# Branch Notes: upstream-pr/03-tui-status-normalization

## Intent
TUI status/header readability normalization for machine payloads.

## Scope
- Normalizes background machine JSON-ish status messages into concise readable labels.\n- Adds regression test for MCP status payload normalization.

## Why This Shape
- Problem was UX-level formatting confusion; best fixed at presentation boundary without protocol churn.

## Concrete Gains
- Clear status headers (e.g., MCP server readiness) instead of raw JSON strings.\n- Reduced visual noise in status bar while preserving progress signal.

## Costs / Tradeoffs
- Some raw payload detail gets summarized.\n- Heuristic mapping may require updates as payload styles evolve.

## Break Risk / Compatibility Risk
- Unexpected payload shapes may fall back to generic labels.

## Validation Run
- cargo test -p codex-tui background_event_normalizes_machine_payloads -- --nocapture

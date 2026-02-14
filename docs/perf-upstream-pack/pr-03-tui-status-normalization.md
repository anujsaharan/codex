# PR 03: TUI Status Normalization

Branch: `upstream-pr/03-tui-status-normalization`  
Commit: `c4b0d9c9c`  
Compare: <https://github.com/openai/codex/compare/main...anujsaharan:upstream-pr/03-tui-status-normalization>

## What Changed

Files:

1. `codex-rs/tui/src/chatwidget.rs`
2. `codex-rs/tui/src/chatwidget/tests.rs`

Behavioral deltas:

1. Added normalization pass for background status headers.
2. Added JSON payload summarization for common machine-format updates (`server/status`, `tool/status`, etc.).
3. Added whitespace compaction and fallback labels.
4. Added test: `background_event_normalizes_machine_payloads`.

## Why This Solution

The issue was perceived "formatting is broken" in status area because machine payloads were surfaced verbatim. This patch normalizes status text at display boundary rather than changing upstream event payload formats.

## System Design Impact

1. Presentation-layer sanitation now centralized in `ChatWidget::set_status` path.
2. No protocol contract changes, no backend behavior changes.

## Concrete Gains (What Resulted In What)

1. Raw machine payload like `{"server":"search-mcp","status":"ready"}` now renders as `MCP server search-mcp: ready`.
2. Streaming status headers remain human-readable and stable.
3. Reduced visual noise in bottom pane status updates.

## Costs / Tradeoffs

1. Some raw payload detail is intentionally hidden behind summarized labels.
2. If payload schema changes, summary heuristics may lag and fall back to generic text.

## Interaction With Existing Code

1. Touches only TUI formatting path.
2. Uses `serde_json::Value` parsing with safe fallback behavior.

## What May Break

1. Users/tools expecting exact raw status text in that field may see normalized variants.
2. Corner-case payloads can normalize to generic fallback (`System update`, `Working`).

## Shoe-horned vs Native Design

Native:

1. Correct layer (view component) for this concern.

Shoe-horned:

1. Heuristic JSON interpretation instead of a strongly typed background-status event schema.

## Validation Done

1. `cargo test -p codex-tui background_event_normalizes_machine_payloads -- --nocapture`


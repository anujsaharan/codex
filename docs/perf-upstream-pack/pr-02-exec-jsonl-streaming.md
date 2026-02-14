# PR 02: `codex-exec` JSONL Streaming and Delta Handling

Branch: `upstream-pr/02-exec-jsonl-streaming`  
Commit: `12a246f53`  
Compare: <https://github.com/openai/codex/compare/main...anujsaharan:upstream-pr/02-exec-jsonl-streaming>

## What Changed

File:

1. `codex-rs/exec/src/event_processor_with_jsonl_output.rs`

Behavioral deltas:

1. Added handling for `ItemStarted/ItemCompleted` agent-message items.
2. Added handling for `AgentMessageContentDelta` as incremental `ItemUpdated` events.
3. Added suppression logic to avoid duplicate legacy agent-message emission.
4. Batched JSONL output generation into one buffered write per processed event group.

## Why This Solution

The old path emitted lots of tiny writes and had duplicate-path message rendering risks between legacy and item-based events.

This update aligns output behavior with incremental turn-item semantics and reduces per-event stdout syscall pressure.

## System Design Impact

1. `codex-exec` now tracks running agent-message state similarly to how it tracks running command state.
2. Event reduction logic moves from consumers to producer-side normalization.
3. Output layer becomes buffered-at-batch, preserving line protocol while reducing write fragmentation.

## Concrete Gains (What Resulted In What)

1. Delta-first output means downstream UIs can render sooner and incrementally.
   Result: less perceived stall before message growth appears.
2. Duplicate legacy message suppression prevents repeated content in mixed event streams.
   Result: cleaner transcript updates and reduced render churn.
3. One buffered write per aggregate block (instead of per line in that block).
   Result: fewer write calls and better stream smoothness under rapid event bursts.

## Costs / Tradeoffs

1. Ordering assumptions between item events and legacy events must stay valid.
2. If protocol evolves, suppression logic may need updates to avoid accidental drops/dupes.

## Interaction With Existing Code

1. Purely internal to `codex-exec` event transformation.
2. No API or protocol shape change to callers.
3. Uses already-available event variants in `codex_core::protocol`.

## What May Break

1. Consumers that implicitly depended on duplicate legacy agent messages could observe fewer events.
2. Edge-case event ordering regressions could cause missing final text if protocol contracts drift.

## Shoe-horned vs Native Design

Native:

1. Built on existing event-aggregation abstraction (`collect_thread_events`).

Shoe-horned:

1. Legacy suppression flag is pragmatic and stateful; a future single canonical stream model would be cleaner.

## Validation Done

1. `cargo test -p codex-exec --quiet`
2. Full integration checks indirectly via TUI/SDK tests in later PRs.

## Post-Review Cleanup Delta

1. Removed `running_agent_messages` map as dead state (it was not used for emission decisions).
2. Removed duplicate reset calls that overlapped with `handle_task_started`.
3. Reduced line count and state surface while preserving event output behavior.

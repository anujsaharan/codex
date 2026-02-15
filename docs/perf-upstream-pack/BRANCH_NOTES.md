# Branch Notes: upstream-pr/02-exec-jsonl-streaming

## Intent
exec JSONL stream normalization + lower-overhead output emission.

## Scope
- Emits agent message progress via item started/updated/completed paths.\n- Suppresses duplicate legacy agent message emission where applicable.\n- Batches serialized JSONL writes per processed aggregate.

## Why This Shape
- Existing output stream could duplicate/fragment message rendering and do more small writes than needed.\n- This keeps protocol shape but simplifies downstream consumers.

## Concrete Gains
- Cleaner incremental UI updates from deltas.\n- Reduced duplicate transcript churn.\n- Fewer write calls under bursty output.

## Costs / Tradeoffs
- Slightly more event-state logic in processor.\n- Behavior depends on item/legacy event ordering contract.

## Break Risk / Compatibility Risk
- Protocol ordering drift could cause missing/duplicated message updates if not kept aligned.

## Validation Run
- cargo test -p codex-exec --quiet

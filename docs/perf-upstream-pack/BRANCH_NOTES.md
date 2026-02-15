# Branch Notes: pr/04-sdk-stream-parser

## Intent
Internal stacked branch for SDK stream parser modernization (includes prior stacked commits).

## Scope
- Branch intent: SDK parser simplification and chunk-boundary correctness; stacked branch may include prior dependent perf work.

## Why This Shape
- Notes clarify branch intent regardless of stacked internal history.

## Concrete Gains
- Gains: deterministic split-chunk handling and lower parser overhead.

## Costs / Tradeoffs
- Tradeoff: manual parser ownership; no runtime parser toggle.

## Break Risk / Compatibility Risk
- Risk: line parsing regressions if future edits bypass tests.

## Validation Run
- pnpm -C sdk/typescript test -- exec.test.ts\n- pnpm -C sdk/typescript build

# Branch Notes: upstream-pr/04-sdk-stream-parser

## Intent
TypeScript SDK stdout parser simplification and chunk-boundary correctness.

## Scope
- Replaces readline path with explicit chunk-buffer line parser.\n- Adds tests for split-line reconstruction across chunk boundaries.

## Why This Shape
- Direct parser has lower overhead and deterministic handling for fragmented JSONL stdout.

## Concrete Gains
- Better resilience when lines are split across chunks.\n- Less parser overhead and less dependency complexity in stream hot path.

## Costs / Tradeoffs
- Manual parser code requires deliberate maintenance and tests.\n- Runtime parser-mode toggle removed in cleaned version.

## Break Risk / Compatibility Risk
- If someone relied on readline timing quirks, emission timing may differ slightly.

## Validation Run
- pnpm -C sdk/typescript test -- exec.test.ts\n- pnpm -C sdk/typescript build

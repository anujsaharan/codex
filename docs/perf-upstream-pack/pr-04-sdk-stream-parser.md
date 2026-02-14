# PR 04: TypeScript SDK Stream Parser Modernization

Branch: `upstream-pr/04-sdk-stream-parser`  
Commit: `1eb036de5`  
Compare: <https://github.com/openai/codex/compare/main...anujsaharan:upstream-pr/04-sdk-stream-parser>

## What Changed

Files:

1. `sdk/typescript/src/exec.ts`
2. `sdk/typescript/tests/exec.test.ts`

Behavioral deltas:

1. Switched stdout parsing to chunk-buffer line parsing as the production path.
2. Correctly handles fragmented lines across arbitrary chunk boundaries.
3. Added test that verifies line reconstruction across split chunks.

## Why This Solution

`readline` introduces avoidable overhead in high-frequency line streaming and is less explicit about chunk-boundary behavior. A direct incremental parser is smaller, faster, and deterministic for JSONL-like streams.

## System Design Impact

1. Parser logic is now self-contained in `CodexExec.run()`.
2. Fewer runtime dependencies in hot path (`readline` removed from parser path).
3. One canonical parsing behavior for all SDK consumers.

## Concrete Gains (What Resulted In What)

1. Lower parser overhead in sustained streaming scenarios.
2. Better stability when child stdout chunks split JSON lines mid-token.
3. Fewer moving parts in stream path.

## Costs / Tradeoffs

1. Runtime parser mode toggle removed in cleanup; less dynamic debugging flexibility.
2. Parser code is manual and must remain carefully tested for edge cases.

## Interaction With Existing Code

1. No API shape changes to SDK consumers.
2. Existing error handling and exit semantics preserved.

## What May Break

1. If a consumer depended on quirks of readline behavior, line timing may differ slightly.
2. Very large unterminated trailing buffers are held until stream end (same fundamental behavior as line-oriented reading).

## Shoe-horned vs Native Design

Native:

1. Fits naturally inside existing async generator output model.

Shoe-horned:

1. None significant; this is a straightforward parser substitution.

## Validation Done

1. `pnpm -C sdk/typescript test -- exec.test.ts`
2. `pnpm -C sdk/typescript build`


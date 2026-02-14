# Ablation Results Snapshot (Captured Before Web/Script Removal)

Date captured: February 14, 2026  
Environment: local fork, synthetic harnesses, local binaries.

These results are included for historical context because the harness scripts were removed from tracked git per request.

## Exec Stream Harness Snapshot

Command used:

```bash
PERF_RUNS=3 PERF_DELTA_CHUNKS=60 PERF_DELTA_DELAY_MS=1 node scripts/perf_ablation_exec_stream.mjs
```

### Stream Tokens Scenario

Optimized:

1. `ttftAssistant`: `2.67ms`
2. `turnComplete`: `65.51ms`
3. `tokenLag`: `0.24ms`
4. `interEventGap`: `1.45ms`

Baseline emulated:

1. `ttftAssistant`: `2.61ms`
2. `turnComplete`: `64.29ms`
3. `tokenLag`: `0.25ms`
4. `interEventGap`: `1.44ms`

### Tool Roundtrip Scenario

Optimized:

1. `ttftAssistant`: `50.53ms`
2. `turnComplete`: `54.51ms`
3. `toolRoundtrip`: `49.26ms`
4. `tokenLag`: `0.48ms`

Baseline emulated:

1. `ttftAssistant`: `50.39ms`
2. `turnComplete`: `53.39ms`
3. `toolRoundtrip`: `49.05ms`
4. `tokenLag`: `0.57ms`

## Web Bridge Harness Snapshot (Now Removed From Tracked Git)

Command used:

```bash
PERF_WEB_RUNS=4 PERF_WEB_DELTA_CHUNKS=120 PERF_WEB_DELTA_DELAY_MS=1 PERF_WEB_TOOL_DELAY_MS=12 node scripts/perf_ablation_web_bridge.mjs
```

Optimized:

1. `ttftAssistant`: `33.45ms`
2. `firstProgress`: `21.52ms`
3. `turnComplete`: `161.90ms`
4. `assistantGap`: `1.07ms`

Baseline emulated:

1. `ttftAssistant`: `32.26ms`
2. `firstProgress`: `20.34ms`
3. `turnComplete`: `161.15ms`
4. `assistantGap`: `1.07ms`

## Interpretation (Concrete)

1. Synthetic token-stream metrics were close between variants.
2. The biggest real-world perceived win likely came from core duplicate-work elimination (cache + single-flight + MCP list reuse), which synthetic harnesses only partially stress.
3. Tradeoff accepted: bounded staleness window for repeated external read tools.
4. Post-review cleanup commits removed dead state and unnecessary key computation; these are behavior-preserving hygiene changes, not new tuning knobs.

## Validation Commands Run On Final Tracked Code

1. `cargo test -p codex-core --lib --quiet`
2. `cargo test -p codex-exec --quiet`
3. `cargo test -p codex-tui background_event_normalizes_machine_payloads -- --nocapture`
4. `pnpm -C sdk/typescript test -- exec.test.ts`
5. `pnpm -C sdk/typescript build`

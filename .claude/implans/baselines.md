# Iteration baselines

Wall-time noise floors and test counts captured at each iteration boundary. Future iterations check perf deltas against these numbers.

## I0 baseline — 2026-04-30

**Host:** Darwin 23.4.0 (macOS), Bash 3.2, cached `target/`.

**Test counts (N₀):**

- `cargo test --features udpipe`: **56 passed**, 0 failed, 0 ignored.
- `cargo test --no-default-features`: **56 lib + 2 doctests + 1 ignored doctest**, 0 failed.
- Integration tests (`cargo test --test integration -- --ignored`): not run (requires UDPipe model). To be captured at first I3/I4 publish gate.

**Wall times — `cargo test --features udpipe` (5 sequential runs, target/ pre-warmed):**

| Run | Real time |
|---|---|
| 1 (cold cache) | 7.84s |
| 2 | 0.50s |
| 3 | 0.50s |
| 4 | 0.48s |
| 5 | 0.46s |

**Steady-state statistics (runs 2–5):**

- min: 0.46s
- median: 0.49s
- max: 0.50s
- mean: 0.485s
- σ ≈ 0.015s
- **2σ noise floor: ±0.03s**

Run 1 is discarded as cold-cache compile time, not steady-state test execution.

**Wall times — `cargo test --no-default-features`:**

- Real time: 0.41s (single run, cache warm).

**Notes:**

- The `cargo test --no-default-features` gate required two pre-existing fixes to land in the I0 commit (otherwise the acceptance gate fails):
  1. `Cargo.toml`: added `[[example]] name = "basic", required-features = ["udpipe"]` so `examples/basic.rs` is skipped when the feature is off.
  2. `README.md`: changed the Rust usage doctest from `rust,no_run` to `rust,ignore` so it does not attempt to compile when `udpipe` is gated out.
  Both are scoped, minimal, and necessary for rule 6 to hold.

## Comparison protocol

When a future iteration changes performance-relevant code (notably I2 task J — `attach_sentences` parse-per-paragraph rewrite), measure as:

```
for i in 1 2 3 4 5; do
    /usr/bin/time -p cargo test --features udpipe --quiet 2>&1 | grep real
done
```

Discard run 1 (cold cache). Compare runs 2–5 median against this baseline's 0.49s.

**Acceptable delta:** within ±2σ (±0.03s, i.e. 0.46–0.52s) — within noise.
**Concern delta:** +20% over baseline (0.59s+) — investigate before merging.
**Block delta:** +50% over baseline (0.74s+) — block merge until profiled.

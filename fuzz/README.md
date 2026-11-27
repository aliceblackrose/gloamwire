# Adversarial parser testing

This standalone cargo-fuzz package exercises production decoding paths without
network access or live credentials. The root crate's hidden `fuzzing` feature
is an unstable test interface, not a supported downstream API.

| Target | Production path |
| --- | --- |
| gateway_json | JSON Gateway envelope and typed dispatch |
| gateway_etf | ETF preflight, decoding, envelope and typed dispatch |
| gateway_zlib | Fragmented zlib streaming decompression and JSON dispatch |
| gateway_zstd | Bounded zstd streaming decompression and JSON dispatch |
| voice_rtp | RTP header decoding and RTP-size encryption layout validation |
| voice_aead | Both AEAD modes, malformed packets, round trips and tampered headers |
| dave | DAVE JSON/binary typed event parsing and bounded participant updates |
| rest_error | REST error parsing, validation error flattening and diagnostic formatting |

## Local reproduction

From the repository root, use the same pinned toolchain as CI:

```sh
rustup toolchain install nightly-2026-11-20 --profile minimal --component rustfmt
cargo +nightly-2026-11-20 install cargo-fuzz --version 0.12.0 --locked
python3 fuzz/seed_corpus.py
cargo +nightly-2026-11-20 fuzz run gateway_etf -- -max_total_time=120 -timeout=10 -rss_limit_mb=4096 -max_len=65536
```

Replace the target name to run another harness. To reproduce an artifact:

```sh
cargo +nightly-2026-11-20 fuzz run gateway_etf fuzz/artifacts/gateway_etf/crash-EXAMPLE
```

Generated corpora, artifacts, and build output are intentionally ignored.
All provided seeds and AEAD keys are synthetic. Do not seed corpora with
production tokens, audio, personal data, or captured real session keys.

CI runs each target with a short PR/push budget and longer weekly/manual
budgets. Failures are not suppressed; crash artifacts are retained for 14
days. The separate fuzz dependency graph is resolved by Cargo, and the
nightly and runner must support sanitizers. These bounded runs are not a
proof of memory safety or protocol correctness.

## Deterministic and property tests

`cargo test --all-features` also checks decompression ceilings, replay-window
behavior against a reference model, participant admission atomicity, cache
invalidation under generated floods, AEAD authentication, rate-limit races,
and cancellation of queue, Identify and REST waiters. The existing CI runs
these tests with Clippy and the feature matrix. Full MLS/provider integration,
network fault injection and Loom instrumentation are outside this suite.

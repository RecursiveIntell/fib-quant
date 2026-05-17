# fib-quant

[![Crates.io](https://img.shields.io/crates/v/fib-quant.svg)](https://crates.io/crates/fib-quant)
[![Docs.rs](https://docs.rs/fib-quant/badge.svg)](https://docs.rs/fib-quant)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`fib-quant` is an experimental Rust implementation of the core radial-angular vector quantization path described in the FibQuant paper:

> Namyoon Lee and Yongjune Kim, "FibQuant: Universal Vector Quantization for Random-Access KV-Cache Compression", arXiv:2605.11478.

This crate is meant for research, conformance experiments, and integration prototypes. It does not claim production KV-cache serving readiness, paper benchmark reproduction, fused GPU kernel support, or superiority over other KV-cache quantization systems.

## What Is Implemented

- profile-validated vector normalization and fixed-rate block quantization;
- deterministic stored rotations for reproducible encode/decode receipts;
- spherical-Beta source sampling and radial quantile construction;
- Fibonacci spiral, Fibonacci sphere, and Roberts-Kronecker direction generation;
- deterministic Lloyd-Max refinement with non-worsening fallback;
- fixed-width bit packing for code indices;
- fail-closed digests and compression receipts;
- corruption, profile, schema, and payload validation tests;
- an optional, default-off `kv` feature with typed KV-cache contracts and a CPU reference encode/decode path.

## What Is Not Claimed

This release does not claim:

- production KV-cache compressor readiness;
- model-level quality validation;
- local reproduction of the FibQuant paper's perplexity, memory, or throughput numbers;
- vLLM, FlashInfer, TensorRT-LLM, Hugging Face, or CUDA integration;
- fused attention-kernel decompression;
- default-on compression in any parent project.

Benchmark results from the arXiv paper should be cited as paper results unless this repository contains local benchmark receipts with enough metadata to reproduce them.

## Install

```toml
[dependencies]
fib-quant = "0.1.0-alpha.1"
```

The KV-cache reference contracts are experimental and default-off:

```toml
[dependencies]
fib-quant = { version = "0.1.0-alpha.1", features = ["kv"] }
```

## Minimal Example

```rust
use fib_quant::{FibQuantProfileV1, FibQuantizer};

fn main() -> fib_quant::Result<()> {
    let mut profile = FibQuantProfileV1::paper_default(8, 2, 8, 7)?;
    profile.training_samples = 128;
    profile.lloyd_restarts = 1;
    profile.lloyd_iterations = 2;

    let quantizer = FibQuantizer::new(profile)?;
    let input = vec![0.25, -0.5, 0.75, 1.0, -1.25, 0.5, 0.125, -0.875];

    let code = quantizer.encode(&input)?;
    let decoded = quantizer.decode(&code)?;

    assert_eq!(decoded.len(), input.len());
    assert_eq!(code.receipt.source_vector_len, input.len());
    Ok(())
}
```

## Release Posture

`0.1.0-alpha.1` is an alpha research release. The public API is intentionally narrow and validation-heavy. Profiles reject unsupported dimensions, rates, methods, training sample counts, schema markers, norm formats, and source modes before allocation-heavy paths run.

The optional `kv` feature adds typed contracts, role-aware policy decisions, fixed-page metadata, receipts, synthetic attention-quality helpers, and CPU reference paths. It remains an experimental reference layer, not a production serving backend.

## Validation

The release gate is:

```bash
python3 scripts/publish_preflight.py
cargo fmt --all --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --examples --all-features
cargo doc --no-deps --all-features
cargo package --list
cargo publish --dry-run
python3 scripts/publish_final_assert.py
```

When working from a dirty or parent-workspace overlay, use the checklist in `RELEASE_CHECKLIST.md` and publish only from a clean Git checkout.

## Documentation

- `docs/compression/FIBQUANT_MATH_CONFORMANCE.md` records implemented math and validation boundaries.
- `docs/compression/FIBQUANT_BENCHMARK_PLAN.md` defines the receipts required before benchmark claims.
- `docs/compression/FIBQUANT_PUBLICATION_NONCLAIMS.md` lists forbidden public claims.
- `docs/kv/KV_PRODUCTION_READINESS_REPORT.md` describes the current default-off KV reference status.

## Citation

If this crate is useful in your work, cite both this implementation and the FibQuant paper. A `CITATION.cff` file is included for citation tooling.

## License

Licensed under the Apache License, Version 2.0. See `LICENSE`.

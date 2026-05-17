# Release Checklist

This crate may only be published as an experimental research implementation after every checked item has a local receipt.

- [ ] `python3 scripts/publish_preflight.py`
- [ ] `cargo fmt --all --check`
- [ ] `cargo test --all-features`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --examples --all-features`
- [ ] `cargo doc --no-deps --all-features`
- [ ] `cargo package --list`
- [ ] `cargo package`
- [ ] `cargo publish --dry-run`
- [ ] `python3 scripts/publish_final_assert.py`
- [ ] repository pushed to public GitHub remote
- [ ] GitHub CI green on `main`
- [ ] release tag created after crates.io dry-run passes from a clean checkout

## Release Posture

- Version: `0.1.0-alpha.1`.
- License: Apache-2.0.
- Publish claim: experimental paper-faithful research implementation.
- Forbidden claim: production KV-cache compressor.
- Forbidden claim: local reproduction of paper benchmark numbers without benchmark receipts.

## Integration Boundaries

- Do not integrate with `semantic-memory`.
- Do not mutate `turbo-quant`.
- Do not make FibQuant default anywhere.
- Do not hide deviations from the paper.

## crates.io Token

Actual publishing requires a crates.io API token. For GitHub Actions, store it as the repository or environment secret `CARGO_REGISTRY_TOKEN` and run the manual `Publish` workflow with the exact version from `Cargo.toml`.

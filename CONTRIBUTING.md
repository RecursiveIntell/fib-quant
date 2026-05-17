# Contributing

Contributions should keep the release posture narrow: this crate is an experimental paper-core implementation with a default-off KV reference layer.

Before opening a pull request, run:

```bash
cargo fmt --all --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --examples --all-features
cargo doc --no-deps --all-features
```

Rules for changes:

- Do not add production KV-cache claims without local benchmark and quality receipts.
- Do not make the `kv` feature default-on.
- Do not add fake GPU kernels or placeholder performance claims.
- Keep decode and digest paths fail-closed.
- Update `README.md`, `CHANGELOG.md`, and the relevant `docs/compression/**` or `docs/kv/**` files when changing public behavior.

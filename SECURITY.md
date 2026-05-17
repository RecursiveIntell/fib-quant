# Security Policy

`fib-quant` is an experimental research crate. Do not treat this release as a production KV-cache serving component.

Report security-sensitive issues privately through GitHub's private vulnerability reporting when it is enabled for the repository. If private reporting is unavailable, open a minimal public issue that does not include exploit details and ask for a maintainer contact path.

Supported security scope for `0.1.0-alpha.1`:

- malformed payload rejection;
- schema and digest validation;
- allocation-bound profile validation;
- fail-closed decode behavior.

Out of scope for this alpha:

- production serving availability guarantees;
- side-channel claims;
- GPU kernel safety;
- model-level quality guarantees.

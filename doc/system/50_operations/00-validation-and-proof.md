## 40. Validation and Proof

Validation is evidence-based and fail-closed.

### Minimum validation posture

- `cargo test` must pass
- slice verifier scripts must pass
- proof binaries must emit deterministic artifacts where repeatability is part of the contract
- tamper scenarios must reject cleanly
- failed validation paths must not publish success receipts or reports

### Current terminal verifier

The current proof chain culminates in `bash scripts/verify_slice_36.sh`, which exercises the full end-to-end chain through terminal consumer import and program capstone reporting.

### Context-assembly verifiers

Two verifiers sit outside that chain because they prove a different contract:

- `bash scripts/verify_context_assembly_continuity.sh` — the continuity profile's
  report is byte-identical across repeated emission.
- `bash scripts/verify_slice_37.sh` — the governed memory source class: its
  provenance rule, its fail-closed refusals, that provenance is bound into the
  bundle identity, that the exported schemas match the types they are generated
  from, and that no pre-existing bundle hash moved.

The second checks a recorded hash value, not merely repeatability. A verifier
that only compares two runs of the same build cannot notice a hash that moved
once and then stayed put, so the values captured before the change are asserted
literally.

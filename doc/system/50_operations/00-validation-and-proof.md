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

Three verifiers sit outside that chain because they prove a different contract:

- `bash scripts/verify_context_assembly_continuity.sh` — the continuity profile's
  report is byte-identical across repeated emission.
- `bash scripts/verify_slice_37.sh` — the governed memory source class: its
  provenance rule, its fail-closed refusals, that provenance is bound into the
  bundle identity, that the exported schemas match the types they are generated
  from, and that no pre-existing bundle hash moved.

- `bash scripts/verify_slice_38.sh` — per-class freshness limits: that an
  override for one class does not slacken another, that a policy naming a class
  twice or naming an unusable class is refused before any source is considered,
  and that the rewritten freshness-band rule agrees with the one it replaced.

The last two check recorded hash values, not merely repeatability. A verifier
that only compares two runs of the same build cannot notice a hash that moved
once and then stayed put, so the values captured before the change are asserted
literally.

Slice 38's verifier also asserts that its equivalence sweep **actually swept**.
The proof reports how many age pairs it compared, and a report claiming zero
disagreements over zero comparisons is the shape a silently-skipped check takes —
it would otherwise read as a pass.

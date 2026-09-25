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

Four verifiers sit outside that chain because they prove a different contract:

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

### Lint hygiene

`bash scripts/verify_lint.sh` runs `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings`. It proves nothing about behaviour
and everything about whether the next reformat will be a large diff that a real
change can hide inside — this repository had accumulated 83 unformatted files and
35 clippy warnings before either command was run in anger, which is what happens
when nothing checks.

It is deliberately not folded into a slice verifier: those prove contracts, and
this does not. A new warning fails the gate rather than joining a pile nobody
reads; where a lint is genuinely wrong for the code, silence it at the site with
a reason, which is a decision on the record.

- `bash scripts/verify_slice_39.sh` — the algorithm-tagged bundle identity: that
  the legacy identity is byte-identical to what it was, that both digests come
  from one canonical string, that the minted id names its algorithm, and that it
  fits the column that has to hold it.

Slice 39's is the one whose failure would be quietest. A pack is keyed by its id,
and a lookup miss falls back to re-grounding rather than erroring — so a drifted
legacy identity surfaces as cost, not as an alarm. The goldens are asserted
against `legacy_bundle_hash` for that reason.

Slice 38's verifier also asserts that its equivalence sweep **actually swept**.
The proof reports how many age pairs it compared, and a report claiming zero
disagreements over zero comparisons is the shape a silently-skipped check takes —
it would otherwise read as a pass.

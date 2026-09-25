# precomputed-context-core - Compiled System Reference

**Designation:** PCC
**Document role:** Canonical compiled technical reference for the precomputed context core contract package
**Source:** `doc/system/`
**Build command:** `bash doc/system/BUILD.sh`
**Document version:** 2.0 (2026-06-22) - canonical compliance migration
**Protocol:** BDS Documentation Protocol v2.0; BDS Repo Documentation System Canonical Compliance Standard

> **Generated artifact warning:** `doc/PCCSYSTEM.md` is assembled output. Edit
> the source modules under `doc/system/` and rebuild. Hand edits to the
> compiled artifact are overwritten by the next build.

Assembly contract:

- Command: `bash doc/system/BUILD.sh`
- Validation: `bash doc/system/validate_snapshots.sh` runs during assembly
- Primary output: `doc/PCCSYSTEM.md`

This `doc/system/` tree is the canonical source of truth for precomputed-context-core. It uses
explicit **truth classes**: canonical facts define repo role, authority
boundaries, contract behavior, runtime behavior, and verification doctrine;
snapshot facts are dated, audit-derived counts and current implementation
inventory that may drift between audits.

| Part | File | Contents |
| --- | --- | --- |
| §1 | `00-identity.md` | 00. Identity |
| §2 | `01-purpose-and-scope.md` | 01. Purpose and Scope |
| §3 | `02-architecture-overview.md` | 02. Architecture Overview |
| §4 | `00-contract-surface.md` | 10. Contract Surface |
| §5 | `00-runtime-boundary.md` | Runtime Boundary |
| §6 | `00-dependencies.md` | 30. Dependencies |
| §7 | `00-versioning-and-slice-progression.md` | 20. Versioning and Slice Progression |
| §8 | `00-validation-and-proof.md` | 40. Validation and Proof |
| §9 | `00-glossary-and-paths.md` | 99. Appendices — Glossary and Paths |

## Quick Assembly

```bash
bash doc/system/BUILD.sh
```

---

## 10. Contract Surface

The contract surface is centered on typed Rust modules and deterministic JSON artifacts.

### Primary contract families

- export package contracts
- replay and evidence contracts
- trust envelope contracts
- import authorization and policy contracts
- promotion and revocation contracts
- lineage bundle and lineage activation contracts
- consumer handoff and acknowledgment contracts
- downstream release and release readiness contracts
- release attestation and sealed release bundle contracts
- terminal consumer import receipt contracts
- context assembly contracts, including the governed memory source class, its
  provenance record, per-class freshness limits, and the algorithm-tagged bundle
  identity

### Contract rule

The library is the authority for contract shape in this repo. Proof binaries prove the contracts; they do not redefine them.

### Source admission is double-gated

A context source is admitted only if its class is phase-1 allowed **and** the
request lists it in `allowed_source_classes`. Adding a class therefore makes it
available, never supplied: a task profile that predates a class cannot receive
one by accident. This is what makes widening `SourceClass` a safe change, and it
is the reason `GovernedMemoryFact` was made phase-1 allowed on landing rather
than staged.

### Governed memory must say where it came from

`SourceClass::GovernedMemoryFact` requires `SourceProvenance` — the memory fact,
the retrieval receipt that supplied it, and the authority it was used under. The
field is optional in the type so existing callers and existing serialized
requests are unaffected, and mandatory in the rule: a memory source without it is
refused rather than admitted unlabelled.

Provenance is appended to the entry's contribution to `bundle_hash` **only when
present**. An entry without it hashes exactly as it always did, so no pre-existing
bundle identity moves; an entry with it binds that provenance into the bundle's
identity, so a bundle cannot silently change which memory it rested on while
keeping its id. Provenance recorded but unhashed would be a label rather than
evidence.

### The bundle identity says what produced it

`context_bundle_id` is `ctxb.sha256.<64 hex>`; `legacy_context_bundle_id` carries
the FNV-1a form it had before Slice 39. Both are computed for every bundle, from
one canonical string, and neither prefix is a prefix of the other — so a resolver
holding a mixed population dispatches on the tag rather than guessing from
length. `ID_PREFIX` and `LEGACY_ID_PREFIX` are exported for that.

Both, permanently. DataForge's `context_packs` keys on the id and has no
retention, so rows minted under the old scheme never age out and there is no date
after which the old form stops mattering.

### Freshness is per class, because lifetimes are

`FreshnessPolicy.max_source_age_minutes` governs every class that has no
override. It could not govern them all well: an active scene is stale in minutes
and a governed memory fact earns its value by persisting, so one number set for
scenes refused every memory fact and one set for memory admitted a stale scene.

`class_overrides` names a limit for a class. It is `Option` and skipped when
absent, so a policy that never mentions it is byte-identical on the wire to one
written before the field existed, and behaves identically. An override for one
class does not slacken another, and `StaleSource` reports the limit that actually
applied rather than the bundle-wide default — an error naming the wrong number
sends a reader looking in the wrong place.

A policy that cannot be read one way is refused before any source is considered:
a class named twice has two limits and no stated way to choose, and an override
for a class that is not phase-1 allowed is dead configuration, which is what a
typo looks like.

`freshness_band` follows the source nearest **its own** limit. Under a single
limit the oldest source was necessarily the closest to refusal; with per-class
limits it is not. The two rules agree wherever no override is present, and that
equivalence is swept rather than argued — see §10.

---

## 30. Dependencies

### Primary technical dependencies

- Rust toolchain
- Cargo test and binary execution surface
- `serde` / `serde_json` for typed artifact serialization
- `sha2` for deterministic hashing and integrity checks
- filesystem-backed proof artifact publication under `target/proof_artifacts/`

### Dependency posture

Dependencies are accepted only when they support deterministic proof emission, validation, serialization, or bounded integrity checking.

---

## 99. Appendices — Glossary and Paths

### Important paths

- Repo root: `~/Forge/ecosystem/precomputed-context-core`
- Documentation source root: `doc/system/`
- Build entry: `doc/system/BUILD.sh`
- Proposed canonical compiled artifact: `doc/PCCSYSTEM.md`
- Proof artifacts root: `target/proof_artifacts/`

### Glossary

- **Designation:** the governed 3-letter repo identity used in canonical compiled artifact naming
- **Canonical compiled artifact:** the assembled root document at `doc/{DESIGNATION}SYSTEM.md`
- **Proof slice:** a bounded implementation and verification increment
- **Fail-closed:** invalid state rejects clearly and does not publish success artifacts

---

## 00. Identity

**Document Date:** 2026-04-16  
**Document Time:** America/New_York  
**Repo:** `precomputed-context-core`  
**Repo Root:** `~/Forge/ecosystem/precomputed-context-core`  
**Proposed Designation:** `PCC`  
**Repo Class:** Library / Contract  
**Language:** Rust  
**Operational Posture:** Internal business system governance, single-operator, fail-closed proof surface

### Identity note

`PCC` is included here as the proposed 3-letter designation for this repository so the documentation system can be assembled deterministically. Final canonical compliance still depends on registry-approved designation uniqueness.

---

# Runtime Boundary

**Truth class:** canonical doctrine

`precomputed-context-core` is a library and contract package, not a resident service. Its runtime behavior is limited to deterministic context assembly, contract validation, and package-local proof commands. It does not own durable truth, deployment, or operator control surfaces.

---

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

---

## 20. Versioning and Slice Progression

This repo advances through governed proof slices rather than freeform feature drift.

### Versioning posture

- contracts are introduced in bounded slices
- proof binaries are added only when a contract family is ready for deterministic validation
- verification scripts must prove stable repeated emission where required
- failures must remain fail-closed and must not publish success artifacts on rejected paths

### Current slice position

The repo has reached capstone proof posture through Slice 36, ending at terminal consumer import validation and a program capstone report.

Slice 37 reopens the context-assembly contract to admit a governed memory source
class. It sits outside the export/import/release chain the capstone sealed, and
does not disturb it: the two bundle hashes captured before the slice are asserted
as goldens, so a context bundle assembled under the earlier slices assembles
identically under this one.

Slice 39 replaces the bundle identity with an algorithm-tagged SHA-256 while
keeping the FNV-1a one it had, because DataForge's `context_packs` keys on the id
and never removes rows — so resolution across both forms is the steady state
rather than a transitional phase. The goldens moved to `legacy_bundle_hash` and
are asserted there, which is the stronger claim: it proves a pack stored under
the old id is still findable.

Slice 38 gives `FreshnessPolicy` per-class limits, closing the gap Slice 37 named
and worked around. It carries the same goldens forward and adds one of its own
kind: the freshness-band rule was rewritten, and the rewrite is proven equivalent
to the rule it replaced by sweeping every age pair across a range of limits with
no override present, rather than by argument.

---

## 01. Purpose and Scope

`precomputed-context-core` is the governed proof-slice core crate for the BDS precomputed-context program.

Its role is to define and prove bounded contracts for:

- export packaging
- intake verification
- trust envelope validation
- policy-backed import authorization
- import rehydration
- promotion, revocation, and re-promotion controls
- lineage bundling, intake, rehydration, activation, and consumption
- bounded consumer handoff and acknowledgment
- downstream release, readiness, attestation, and sealing
- terminal consumer import validation

### In-scope posture

This repo is responsible for contract truth, proof truth, deterministic report emission, and fail-closed validation behavior for the precomputed-context proof chain.

### Out-of-scope posture

This repo does not own live production orchestration, external service runtime, UI policy surfaces, or registry authority for designation uniqueness.

---

## 02. Architecture Overview

The crate is organized as a proof-oriented library with binary proof surfaces.

### Core architecture pattern

- library modules define typed contracts and validation logic
- proof binaries exercise deterministic success and fail-closed rejection paths
- proof artifacts are emitted under `target/proof_artifacts/`
- verification scripts run the full chain end to end

### Current proof posture

The repo has been driven slice by slice through terminal consumer import validation.

The active proof chain includes:

1. package export and zip emission
2. trusted envelope and import policy enforcement
3. import intake, rehydration, and roundtrip validation
4. promotion governance and rollback controls
5. lineage continuity proof
6. bounded consumer handoff proof
7. downstream release and readiness proof
8. sealed release bundle proof
9. terminal consumer import and program capstone proof

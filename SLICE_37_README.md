# Slice 37 bundle — a governed memory source class

Adds the source class and provenance slot that let governed memory enter a
context bundle **as itself**, rather than disguised as manuscript content.

Requested by `forge-memory` (FMEM-05) and specified in
[RFC-FMEM-PCC-memory-source](https://github.com/Boswell-Digital-Solutions/forge-memory/blob/master/doc/rfcs/RFC-FMEM-PCC-memory-source.md),
filed here as [issue #5](https://github.com/Boswell-Digital-Solutions/precomputed-context-core/issues/5).
This implements **P1** and **P2**; P4 (bundle-wide freshness) is deliberately out
of scope and noted below.

Unlike Slices 15–36 this arrives as a pull request rather than a zip, so there is
no `SLICE_37_WIRING.md` — there is nothing to unzip and the `src/lib.rs` export
is already in the diff.

## The problem

`SourceClass` was a closed manuscript vocabulary and `SourceInput` had no
provenance field. A memory fact could only enter a bundle by being labelled as
something it is not — most plausibly an `AcceptedLoreRecord`, which works today
and which would give it the standing authority of accepted manuscript lore, no
record that it was memory, and no link to the receipt that supplied it. Every
question an operator might later ask of the manifest becomes unanswerable.

A slot that does not exist gets improvised. This adds the slot.

## What Slice 37 adds

- `src/context_assembly.rs`
  - `SourceClass::GovernedMemoryFact`, phase-1 allowed
  - `SourceClass::requires_provenance()`
  - `SourceProvenance { memory_ref, retrieval_receipt_ref, authority_ref }`
  - `SourceInput::provenance` and `SourceInventoryEntry::provenance`, both
    `Option<SourceProvenance>`
  - `ContextAssemblyError::MissingProvenance`
  - provenance appended to the entry's hash piece **only when present**
- `src/bin/proof_slice37_governed_memory_source.rs`
  - proves deterministic repeated emission with memory present
  - proves provenance is bound into the bundle identity, not merely carried
  - proves fail-closed rejection for:
    - a memory source with no provenance
    - a memory source whose profile does not allow the class
    - a missing-provenance error outranking an unrelated staleness failure
  - proves no publication on rejected paths
  - proves the two pre-slice bundle hashes have not moved
- `tests/slice37_governed_memory_source.rs`
- `scripts/verify_slice_37.sh`

## New expected artifacts

- `target/proof_artifacts/slice37_governed_memory_source/governed_memory_source_report.json`

## Two rules that are doing the work

**Optional in the type, mandatory in the rule.** `provenance` is an `Option`, so
every existing caller keeps compiling and every request serialized before this
slice keeps deserializing. A source whose class is `GovernedMemoryFact` and whose
provenance is `None` is refused. Optional alone would not be fail-closed;
required alone would break every manuscript caller. Neither is enough by itself.

**Provenance joins the hash only when present.** `compute_bundle_hash` builds one
piece per inventory entry; an entry without provenance hashes to the byte-identical
string it always did, so every bundle assembled before this slice keeps its
`bundle_hash` and `context_bundle_id`. When provenance *is* present it is bound
into the bundle's identity, so a bundle cannot silently change which memory fact
it rested on while keeping its id. Provenance recorded but unhashed would be a
label rather than evidence.

## What did not change

Two bundle hashes were captured from `master` at `b4868c5` **before** the enum
grew a variant and `SourceInput` grew a field, and are asserted as goldens in both
the test suite and the proof binary:

| Profile | `bundle_hash` |
| --- | --- |
| `tests/context_assembly_phase1.rs` base request | `f821a73703a5ec8d` |
| continuity profile (`proof_context_assembly_continuity`) | `81419b53ca63648c` |

Both still hold. `verify_slice_37.sh` re-checks the continuity value against the
emitted report, because that profile's own verifier only proves its report is
stable across two runs of the same build — which cannot notice a hash that moved
once and then stayed put.

Admission also remains **double-gated**: a source must be phase-1 allowed *and*
listed in the request's `allowed_source_classes`. A manuscript profile written
before this slice lists no memory class, so it cannot be handed one by accident.
Making the class phase-1 allowed makes it *available*, not *supplied*.

## Schema impact

`schemas/context_assembly_request.schema.json` and
`context_bundle_manifest.schema.json` are regenerated. The diff is **purely
additive**: `SourceClass` gains one string in its existing flat `enum` list, and
a `SourceProvenance` definition plus an optional `provenance` property appear.

The `GovernedMemoryFact` variant deliberately carries a line comment rather than
a doc comment: schemars promotes an enum whose variants carry docs from a flat
`enum` into a `oneOf` of per-variant branches, which is semantically equivalent
and structurally different. A consumer reading the exported schema should see one
more string in a list, not a changed shape.

Anyone vendoring these schemas should re-vendor.

## Out of scope: bundle-wide freshness (RFC P4)

`freshness_policy.max_source_age_minutes` governs every source in a bundle, and
exceeding it is fatal. An active scene is stale in minutes; a governed memory
fact is *supposed* to be old. One number cannot serve both.

This slice does not change that — it would alter a rule governing every existing
bundle and deserves its own evidence. The interim needs no code:
`validate_required_target_refs` only requires a target ref the request actually
names, so a **memory-only bundle** with empty `target_refs` and a
memory-appropriate policy assembles cleanly. That path is proved in the test
suite and the proof binary. Mixed manuscript-and-memory bundles wait for P4.

## Also noticed, not addressed

`bundle_hash` is FNV-1a 64-bit and `context_bundle_id` is that hash in full.
FNV collisions are constructible rather than merely improbable, and the
ecosystem's canonical digest is `forge.rfc8785-jcs-sha256.v1`. Making the bundle
identity forgeable is a different problem with different reviewers; folding it
into a source-class slice would be wrong. Recorded because an observation dropped
for being inconvenient is an observation lost.

## Verification

```bash
bash scripts/verify_slice_37.sh
```

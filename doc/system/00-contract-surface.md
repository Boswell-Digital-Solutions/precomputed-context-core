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

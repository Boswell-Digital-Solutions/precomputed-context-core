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
- context assembly contracts, including the governed memory source class and its
  provenance record

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

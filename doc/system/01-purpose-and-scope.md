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

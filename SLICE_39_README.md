# Slice 39 bundle — an algorithm-tagged bundle identity

`context_bundle_id` was FNV-1a 64, in full: sixteen hex characters behind a
`ctxb_` prefix. That was an unremarkable choice while the value was a
deterministic name.

It stopped being one. PACT's packet builders carry `context_bundle_hash` into
emitted packets, and forgeHQ's verification bridge binds a pact verdict to the
pair, describing the binding in its own docstring as proof that a verification
"was produced against the exact governed, replay-eligible context the fix was
built from". A 64-bit non-cryptographic digest is roughly 2^32 birthday work —
seconds of ordinary hardware — and does not carry that claim.

Raised as `precomputed-context-core#9` with the measurement rather than the
adjective.

## What Slice 39 adds

- `ContextBundleManifest.context_bundle_id` — now `ctxb.sha256.<64 hex>`
- `ContextBundleManifest.bundle_hash` — SHA-256
- `ContextBundleManifest.legacy_context_bundle_id` / `legacy_bundle_hash` — the
  FNV identity, unchanged
- `ID_PREFIX` / `LEGACY_ID_PREFIX`, exported so a resolver never hardcodes them
- `tests/slice39_tagged_bundle_identity.rs`
- `src/bin/proof_slice39_tagged_bundle_identity.rs`
- `scripts/verify_slice_39.sh`

## Why both identities, permanently

This is **not a migration**. DataForge's `context_packs` keys on the id and has
no retention — no `DELETE` anywhere, no TTL, no cleanup job, and the router
exposes only `POST` and `GET`. Rows minted under the old scheme never age out,
so there is no date after which the old form stops mattering.

Dual resolution is therefore the steady state, not a transitional phase, and the
design reflects that: both identities are computed for every bundle, from **one
canonical string**, forever. Two independent computations could drift apart; one
canonicalisation with two digests cannot.

## The claim that had to be proven

Not that the legacy identity is still *present* — that it is **unchanged**.

A pack stored before this slice is keyed by its legacy id. If that value drifted,
the pack becomes unfindable — and unfindable here does not raise an error. It
falls back to re-grounding, which surfaces as cost and latency rather than as a
failure anyone would page on. That is the worst way for a migration to go wrong.

So the goldens captured before the change are now asserted against
`legacy_bundle_hash`, and the verifier greps the continuity report for the legacy
value specifically. Mutating the legacy digest by one is caught by seven tests.

## What consumers must know

**New packs land under new ids.** A bundle re-assembled after this slice will not
find its pre-migration pack, because the id it now mints is different. That is a
one-time cache-warm cost, not a correctness problem — but it will look like a
token and latency regression, and it is better known in advance than diagnosed
later.

**A resolver must accept both prefixes.** They are disjoint by construction —
`ctxb.sha256.` does not start with `ctxb_`, and vice versa — so a resolver can
dispatch on the prefix without guessing from length. `ID_PREFIX` and
`LEGACY_ID_PREFIX` are exported for exactly this.

**The minted id is 76 characters**, inside the `String(128)` that
`context_packs.context_pack_id` provides. The proof asserts this rather than
assuming it, because an identity scheme that did not fit would fail at the store
rather than here.

## Not a bundle: delivered as a pull request

Like Slice 38, this is in the repository already — no unzip step, no
`SLICE_39_WIRING.md`. Running the verifier is the whole procedure:

```bash
bash scripts/verify_slice_39.sh
```

It chains Slice 38, which chains Slice 37 and the continuity verifier.

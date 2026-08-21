# Slice 38 bundle — per-class freshness limits

Closes the gap Slice 37 left open and named: `freshness_policy` was bundle-wide,
so one `max_source_age_minutes` had to serve every source class at once.

Source classes have different natural lifetimes. An active scene is stale in
minutes; a governed memory fact earns its value by persisting. A limit set for
scenes refuses every memory fact, and a limit set for memory admits a stale
scene. Slice 37 shipped a memory-only bundle with a one-year limit as the
workable interim; this slice removes the need for it, and a mixed
manuscript-plus-memory bundle becomes expressible for the first time.

## What Slice 38 adds

- `ClassFreshnessOverride { source_class, max_source_age_minutes }`
- `FreshnessPolicy.class_overrides: Option<Vec<ClassFreshnessOverride>>`
  - `#[serde(default, skip_serializing_if = "Option::is_none")]`, so a policy
    that never mentions it is byte-identical on the wire to one written before
    the field existed
  - `FreshnessPolicy::uniform(n)` — the shape every caller had before
  - `FreshnessPolicy::max_for(&class)` — the limit that applies to a class
- `ContextAssemblyError::DuplicateFreshnessOverride`
- `ContextAssemblyError::UnsupportedFreshnessOverride`
- `tests/slice38_per_class_freshness.rs`
- `src/bin/proof_slice38_per_class_freshness.rs`
- `scripts/verify_slice_38.sh`

## The band rule changed, and that is the part to check

`freshness_band` was computed from the **oldest source against the single
limit**. With per-class limits the oldest source is no longer necessarily the
one closest to being refused, so it is now computed from **any source against
its own limit**.

Those two rules agree exactly whenever no override is present — with a single
limit `L`, `max(age) * 2 >= L` holds precisely when some source satisfies
`age * 2 >= L` — and the `L == 0` case is carried by a per-source guard that
keeps a zero limit contributing `Fresh`, as the old rule did.

That equivalence is **proven rather than argued**: the test suite sweeps every
age pair across a range of limits and compares against the replaced rule
reproduced literally. The two pre-existing profiles' bundle hashes are pinned as
goldens captured on `master` at the tip of Slice 37.

## A policy that cannot be read one way is refused

- **Duplicate override** — one class with two limits, and no stated way to
  choose. Refused.
- **Override for a class that is not phase-1 allowed** — dead configuration,
  which is what a typo looks like. Refused rather than ignored.

Both are checked before any source is considered, so a request with both a bad
policy and a stale source reports the policy defect rather than whichever the
loop reached first.

## Not a bundle: delivered as a pull request

Slices 15–36 arrived as zip bundles with a `SLICE_NN_WIRING.md` telling you to
unzip and export a module. This slice is in the repository already, so there is
no wiring step and no `SLICE_38_WIRING.md`. Running the verifier is the whole
procedure:

```bash
bash scripts/verify_slice_38.sh
```

## Provenance

Raised as **P4** of `RFC-FMEM-PCC-memory-source`, filed by `forge-memory` as
[issue #5](https://github.com/Boswell-Digital-Solutions/precomputed-context-core/issues/5).
That RFC deliberately *raised* P4 without proposing it, on the grounds that it
changes a rule governing every existing bundle and deserved its own evidence.
This slice is that evidence.

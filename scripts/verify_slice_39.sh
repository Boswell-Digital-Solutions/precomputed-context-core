#!/usr/bin/env bash
set -euo pipefail

# The exported schemas must match the types they are generated from. A schema
# left unregenerated is a consumer reading a contract the code no longer
# implements.
#
# Snapshotted FIRST, before anything else runs, because `cargo test` rewrites
# schemas/ as a side effect: tests/core_contracts.rs::
# schema_validation_bundle_passes calls export_schemas against CARGO_MANIFEST_DIR.
# A drifted schema therefore heals itself during the test run, and any check
# placed after it can never fire.
schema_digest() {
  find schemas -name '*.schema.json' | sort | xargs sha256sum | sha256sum | awk '{print $1}'
}
schemas_before="$(schema_digest)"

cargo test

schemas_after="$(schema_digest)"
if [[ "$schemas_before" != "$schemas_after" ]]; then
  echo "schemas/ did not match the types they are generated from."
  echo "They have now been regenerated in place — review and commit the result."
  exit 1
fi

# Per-class limits, the refusals, and the band equivalence sweep.
cargo run --bin proof_slice39_tagged_bundle_identity

report="target/proof_artifacts/slice39_tagged_bundle_identity/tagged_bundle_identity_report.json"

hash_1="$(sha256sum "$report" | awk '{print $1}')"
cargo run --bin proof_slice39_tagged_bundle_identity >/dev/null
hash_2="$(sha256sum "$report" | awk '{print $1}')"
if [[ "$hash_1" != "$hash_2" ]]; then
  echo "slice 38 report hash changed across repeated emission"
  exit 1
fi

test -f "$report"

# The legacy identity must be preserved, not merely present. A pack stored
# before this slice is keyed by it, and a lookup miss falls back to re-grounding
# rather than erroring — so drift here is a silent cost regression, not an alarm.
assert_report() {
  if ! grep -qE "$1" "$report"; then
    echo "$2"
    exit 1
  fi
}

assert_report '"legacy_identity_preserved": true'   "a pre-existing bundle identity moved; every pack stored under it is now unfindable"
assert_report '"identities_share_one_canonical_string": true'   "the two digests were taken over different inputs and can drift apart"
assert_report '"current_identity_is_tagged": true'   "the minted id does not name the algorithm that produced it"
assert_report '"id_fits_store_column": true'   "the minted id is too long for context_packs.context_pack_id (String(128))"

# The two pre-existing profiles' replay identities must survive this slice.
# Their own verifiers only prove a report is stable across two runs of the *same*
# build, which cannot notice a hash that moved once and then stayed put — so the
# recorded values are checked here against the ones captured before the change.
bash scripts/verify_context_assembly_continuity.sh
# Slice 39 moved the FNV identity to legacy_bundle_hash and minted a tagged
# sha256 as bundle_hash. Asserting the legacy value here is the stronger claim:
# it proves a pack stored under the old id is still findable.
grep -q '"legacy_bundle_hash": "81419b53ca63648c"' \
  target/proof_artifacts/context_assembly_continuity/continuity_manifest_report.json

bash scripts/verify_slice_38.sh

echo "Slice 39 verification passed"

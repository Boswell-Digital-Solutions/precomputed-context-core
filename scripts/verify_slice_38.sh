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
cargo run --bin proof_slice38_per_class_freshness

report="target/proof_artifacts/slice38_per_class_freshness/per_class_freshness_report.json"

hash_1="$(sha256sum "$report" | awk '{print $1}')"
cargo run --bin proof_slice38_per_class_freshness >/dev/null
hash_2="$(sha256sum "$report" | awk '{print $1}')"
if [[ "$hash_1" != "$hash_2" ]]; then
  echo "slice 38 report hash changed across repeated emission"
  exit 1
fi

test -f "$report"

# The sweep must actually have swept. A report claiming zero disagreements over
# zero comparisons is the shape a silently-skipped check takes, and it would read
# as a pass. Asserted with grep rather than a JSON parser so the verifier depends
# on nothing absent from a stock Git Bash — `python3` is not there.
assert_report() {
  if ! grep -qE "$1" "$report"; then
    echo "$2"
    exit 1
  fi
}

assert_report '"disagreements": 0'   "band rule disagreed with the one it replaced"
assert_report '"age_pairs_compared": [0-9]{3,}'   "the band equivalence sweep did not run"
assert_report '"preexisting_hashes_unmoved": true'   "a pre-existing bundle hash moved"
assert_report '"band_follows_nearest_to_its_own_limit": true'   "the band did not follow the source nearest its own limit"

if grep -q '"published_manifest": true' "$report"; then
  echo "a refused request published a manifest"
  exit 1
fi

# The two pre-existing profiles' replay identities must survive this slice.
# Their own verifiers only prove a report is stable across two runs of the *same*
# build, which cannot notice a hash that moved once and then stayed put — so the
# recorded values are checked here against the ones captured before the change.
bash scripts/verify_context_assembly_continuity.sh
grep -q '"bundle_hash": "81419b53ca63648c"' \
  target/proof_artifacts/context_assembly_continuity/continuity_manifest_report.json

bash scripts/verify_slice_37.sh

echo "Slice 38 verification passed"

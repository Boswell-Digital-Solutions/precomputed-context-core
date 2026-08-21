#!/usr/bin/env bash
set -euo pipefail

# The exported schemas must match the types they are generated from. A schema
# left unregenerated is a consumer reading a contract the code no longer
# implements.
#
# This is snapshotted FIRST, before anything else runs, because `cargo test`
# rewrites schemas/ as a side effect: tests/core_contracts.rs::
# schema_validation_bundle_passes calls export_schemas against CARGO_MANIFEST_DIR.
# A drifted schema therefore heals itself during the test run, and any check
# placed after it can never fire.
#
# Compared against the working tree rather than against HEAD, deliberately: a
# HEAD diff fails every change that legitimately regenerates a schema, right up
# until the moment it is committed. In a fresh checkout the two are the same
# question, and the one worth asking is "does regenerating change anything".
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

# The new class, its provenance rule, and the fail-closed refusals.
cargo run --bin proof_slice37_governed_memory_source

report="target/proof_artifacts/slice37_governed_memory_source/governed_memory_source_report.json"

hash_1="$(sha256sum "$report" | awk '{print $1}')"
cargo run --bin proof_slice37_governed_memory_source >/dev/null
hash_2="$(sha256sum "$report" | awk '{print $1}')"
if [[ "$hash_1" != "$hash_2" ]]; then
  echo "slice 37 report hash changed across repeated emission"
  exit 1
fi

test -f "$report"

# The continuity profile's replay identity must survive this slice. Its own
# verifier only proves the report is stable across two runs of the *same* build,
# which cannot notice a hash that moved once and then stayed put — so the
# recorded value is checked here against the one captured before the change.
bash scripts/verify_context_assembly_continuity.sh
grep -q '"bundle_hash": "81419b53ca63648c"' \
  target/proof_artifacts/context_assembly_continuity/continuity_manifest_report.json

echo "Slice 37 verification passed"

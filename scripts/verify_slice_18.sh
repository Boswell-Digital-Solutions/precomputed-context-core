#!/usr/bin/env bash
set -euo pipefail

cargo test
cargo run --bin proof_check -- --export-package
cargo run --bin proof_zip
cargo run --bin proof_trust_envelope
cargo run --bin proof_intake_check
cargo run --bin proof_import_authorize
cargo run --bin proof_import_rehydrate
cargo run --bin proof_import_roundtrip

trust_hash_1="$(sha256sum target/proof_artifacts/slice14_export.zip.trust_envelope.json | awk '{print $1}')"
cargo run --bin proof_trust_envelope >/dev/null
trust_hash_2="$(sha256sum target/proof_artifacts/slice14_export.zip.trust_envelope.json | awk '{print $1}')"
if [[ "$trust_hash_1" != "$trust_hash_2" ]]; then
  echo "trust envelope hash changed across repeated emission"
  exit 1
fi

auth_hash_1="$(sha256sum target/proof_artifacts/slice19_policy/authorization_report.json | awk '{print $1}')"
cargo run --bin proof_import_authorize >/dev/null
auth_hash_2="$(sha256sum target/proof_artifacts/slice19_policy/authorization_report.json | awk '{print $1}')"
if [[ "$auth_hash_1" != "$auth_hash_2" ]]; then
  echo "authorization report hash changed across repeated emission"
  exit 1
fi

test -f target/proof_artifacts/slice19_policy/current/authorization_receipt.json

echo "Slice 18 verification passed"

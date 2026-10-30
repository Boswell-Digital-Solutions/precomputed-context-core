# Known Issues

This file records confirmed defects and open problems. Each entry states its date, what is wrong, the root cause, the fix, and an open or closed scope.

## KI-PCC-20261030-001 — `verify_slice_18.sh` failed on every run after slice 19

- **Date**: 2026-10-30. Found while adding CI, by running every slice verifier on `b0071c0`.
- **What is wrong**: `scripts/verify_slice_18.sh` read `target/proof_artifacts/slice18_import_authorization/authorization_report.json` and `.../current/authorization_receipt.json`. `proof_import_authorize` does not write either file. The script stopped at the first `sha256sum` with "No such file or directory". Slices 14 to 17 and 19 to 39 passed.
- **Root cause**: slice 19 (`c786066`) moved the binary's output to `target/proof_artifacts/slice19_policy/` (`src/bin/proof_import_authorize.rs:142`). The slice 18 verifier was not updated. Nothing ran the verifiers automatically, so the failure went unseen.
- **Fix**: the verifier now reads `target/proof_artifacts/slice19_policy/authorization_report.json` and `.../current/authorization_receipt.json`. Its checks are unchanged: the report hash stays stable across repeated emission, and the receipt exists. The new CI runs every slice verifier on each change.
- **Scope**: **closed** by the change that adds CI.

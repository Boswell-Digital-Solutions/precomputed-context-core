#!/usr/bin/env bash
set -euo pipefail

# Formatting and lint hygiene, as a gate rather than an intention.
#
# This repository accumulated 83 unformatted files and 35 clippy warnings before
# anyone ran either command in anger. That is not a criticism of the slices — it
# is what happens when nothing checks. The debt is cleared; this keeps it clear.
#
# Deliberately not folded into a slice verifier. The per-slice scripts prove a
# contract; this proves nothing about behaviour and everything about whether the
# next reformat will be a 2,000-line diff that a real change can hide inside.

cargo fmt --check

# -D warnings, so a new lint fails rather than joining a pile nobody reads. If a
# warning here is genuinely wrong for this code, silence it at the site with
# #[allow(...)] and a reason — that is a decision on the record, which a growing
# warning count is not.
cargo clippy --all-targets -- -D warnings

echo "Lint verification passed"

#!/usr/bin/env bash
set -euo pipefail

PARTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$PARTS_DIR/../.." && pwd)"
ASSEMBLED_OUTPUT="${1:-$ROOT_DIR/doc/pccSYSTEM.md}"

require_contains() {
  local file="$1"; local needle="$2"; local label="$3"
  if ! grep -Fq -- "$needle" "$file"; then echo "snapshot validation failed: $label missing in $file" >&2; echo "expected: $needle" >&2; exit 1; fi
}
require_absent() {
  local file="$1"; local needle="$2"; local label="$3"
  if grep -Fq -- "$needle" "$file"; then echo "snapshot validation failed: $label still present in $file" >&2; echo "unexpected: $needle" >&2; exit 1; fi
}

require_contains "$PARTS_DIR/_index.md" "**Designation:** pcc" "index designation"
require_contains "$PARTS_DIR/_index.md" "BDS Documentation Protocol v2.0" "index protocol"
require_contains "$PARTS_DIR/_index.md" 'Primary output: `doc/pccSYSTEM.md`' "index primary output"
require_contains "$PARTS_DIR/BUILD.sh" 'DESIGNATION="pcc"' "build designation"
require_absent "$PARTS_DIR/_index.md" 'Primary output: `doc/SYSTEM.md`' "index legacy primary output"
test -f "$ASSEMBLED_OUTPUT"
require_contains "$ASSEMBLED_OUTPUT" "Document version" "assembled document version header"
require_contains "$ASSEMBLED_OUTPUT" "**Designation:** pcc" "assembled designation"
require_contains "$ASSEMBLED_OUTPUT" 'Primary output: `doc/pccSYSTEM.md`' "assembled primary output"
require_contains "$ASSEMBLED_OUTPUT" "BDS Documentation Protocol v2.0" "assembled protocol"
require_contains "$ASSEMBLED_OUTPUT" "truth classes" "assembled truth classes"
require_absent "$ASSEMBLED_OUTPUT" 'Primary output: `doc/SYSTEM.md`' "assembled legacy primary output"
require_absent "$ASSEMBLED_OUTPUT" 'Root `SYSTEM.md` is the primary assembled reference.' "assembled legacy primary reference"
echo "snapshot validation passed: $ASSEMBLED_OUTPUT"

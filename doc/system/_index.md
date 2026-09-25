# precomputed-context-core - Compiled System Reference

**Designation:** PCC
**Document role:** Canonical compiled technical reference for the precomputed context core contract package
**Source:** `doc/system/`
**Build command:** `bash doc/system/BUILD.sh`
**Document version:** 2.0 (2026-06-22) - canonical compliance migration
**Protocol:** BDS Documentation Protocol v2.0; BDS Repo Documentation System Canonical Compliance Standard

> **Generated artifact warning:** `doc/PCCSYSTEM.md` is assembled output. Edit
> the source modules under `doc/system/` and rebuild. Hand edits to the
> compiled artifact are overwritten by the next build.

Assembly contract:

- Command: `bash doc/system/BUILD.sh`
- Validation: `bash doc/system/validate_snapshots.sh` runs during assembly
- Primary output: `doc/PCCSYSTEM.md`

This `doc/system/` tree is the canonical source of truth for precomputed-context-core. It uses
explicit **truth classes**: canonical facts define repo role, authority
boundaries, contract behavior, runtime behavior, and verification doctrine;
snapshot facts are dated, audit-derived counts and current implementation
inventory that may drift between audits.

| Part | File | Contents |
| --- | --- | --- |
| §1 | `00-identity.md` | 00. Identity |
| §2 | `01-purpose-and-scope.md` | 01. Purpose and Scope |
| §3 | `02-architecture-overview.md` | 02. Architecture Overview |
| §4 | `00-contract-surface.md` | 10. Contract Surface |
| §5 | `00-runtime-boundary.md` | Runtime Boundary |
| §6 | `00-dependencies.md` | 30. Dependencies |
| §7 | `00-versioning-and-slice-progression.md` | 20. Versioning and Slice Progression |
| §8 | `00-validation-and-proof.md` | 40. Validation and Proof |
| §9 | `00-glossary-and-paths.md` | 99. Appendices — Glossary and Paths |

## Quick Assembly

```bash
bash doc/system/BUILD.sh
```

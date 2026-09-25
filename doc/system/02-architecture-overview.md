## 02. Architecture Overview

The crate is organized as a proof-oriented library with binary proof surfaces.

### Core architecture pattern

- library modules define typed contracts and validation logic
- proof binaries exercise deterministic success and fail-closed rejection paths
- proof artifacts are emitted under `target/proof_artifacts/`
- verification scripts run the full chain end to end

### Current proof posture

The repo has been driven slice by slice through terminal consumer import validation.

The active proof chain includes:

1. package export and zip emission
2. trusted envelope and import policy enforcement
3. import intake, rehydration, and roundtrip validation
4. promotion governance and rollback controls
5. lineage continuity proof
6. bounded consumer handoff proof
7. downstream release and readiness proof
8. sealed release bundle proof
9. terminal consumer import and program capstone proof

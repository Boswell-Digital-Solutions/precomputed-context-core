## 30. Dependencies

### Primary technical dependencies

- Rust toolchain
- Cargo test and binary execution surface
- `serde` / `serde_json` for typed artifact serialization
- `sha2` for deterministic hashing and integrity checks
- filesystem-backed proof artifact publication under `target/proof_artifacts/`

### Dependency posture

Dependencies are accepted only when they support deterministic proof emission, validation, serialization, or bounded integrity checking.

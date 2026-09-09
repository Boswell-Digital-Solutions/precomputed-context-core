# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`precomputed-context-core` is the core Rust crate for the BDS Precomputed Context Program proof slice — a governed core subsystem for deterministic precomputed context construction and related identity/connectivity behavior. It exists to keep the first implementation slice contract-first, authority-first, and event-first (authority resolution, lifecycle/freshness state algebra, artifact/packet contract validation, event dedupe/coalescing, fixture bundle validation, JSON schema export) rather than letting later service integration redefine the meaning of state, admissibility, or override behavior. This is not an MVP or a generic utility package.

## Common Commands

- `cargo test` — run the test suite
- `cargo run --bin proof_check` — the unified proof entrypoint for first-wave contract proof
- `cargo run --bin <name>` — run a specific slice proof binary (see `src/bin/`, e.g. `proof_import_authorize`, `proof_release_readiness`, `proof_sealed_release_bundle`, `schema_check`, `export_schemas`, `fixture_check`)
- `bash scripts/verify_slice_<N>.sh` — run a specific slice's verification script (slices 14-39 each have one)
- `bash scripts/verify_lint.sh` — formatting and lint gate: `cargo fmt --check` then `cargo clippy --all-targets -- -D warnings`
- `bash scripts/verify_context_assembly_continuity.sh` — verify context-assembly continuity behavior

## Architecture

- `src/lib.rs` plus one file per governed concern in `src/` — e.g. `authority.rs`, `context_assembly.rs`, `events.rs`, `state_machine.rs`, `schema_validation.rs`, `evidence_bundle.rs` / `evidence_store.rs`, `lineage_*.rs`, `promotion_gate.rs` / `promotion_revocation.rs` / `re_promotion.rs`, `import_*.rs`, `trust_envelope.rs`, `sealed_release_bundle.rs`, `supersession_chain.rs`, `terminal_consumer_import.rs`
- `src/bin/` — one proof binary per slice/contract (see Common Commands); `src/proof/` and `src/fixture_support/` back these
- `schemas/`, `fixtures/` — exported JSON schemas for governed contracts and the fixtures validated against them
- `docs/plans/` — the plan set behind each slice; `SLICE*_README.md` / `SLICE*_WIRING.md` / `SLICE*_NOTES.md` at repo root document individual slices as they landed
- `doc/` — system documentation
- Architecture boundaries are deliberately kept separate: source collection, normalization/shaping, bundle construction, identity/hashing, connectivity field carriage, and downstream transport/orchestration are distinct concerns — this repo must not absorb app-specific runtime policy.

## Notes

- **Determinism is load-bearing.** Preserve stable ordering wherever it matters, preserve deterministic hash/ID derivation, and avoid hidden nondeterminism or silent inclusion of unstable/incidental context. Treat any change to output identity as high-risk and explain why.
- **Contract discipline.** Do not invent bundle fields, identity rules, hash semantics, or connectivity fields. When changing bundle/context behavior: identify the current contract first, identify affected fields explicitly, preserve current identity semantics unless the task explicitly changes them, and call out anything that affects compatibility. Key fields to never casually rename/drop/synthesize: `task_intent_id`, `context_bundle_id`, `context_bundle_hash`.
- **Bounded context by design.** Avoid "more context is always better" — prefer deliberate, selective inclusion rules over catch-all inclusion; don't casually broaden a bundle "just in case."
- **Cross-repo caution.** This repo may sit upstream of other systems — do not assume downstream systems interpret a bundle the same way unless verified, and separate bundle-shape issues from transport issues.
- Not yet in scope: storage substrate, repo discovery, ForgeCommand UI trust surfaces, packet composition runtime, invalidation worker orchestration, RBAC enforcement layer, durable audit persistence.

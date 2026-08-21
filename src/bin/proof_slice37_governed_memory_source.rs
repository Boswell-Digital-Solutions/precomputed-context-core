//! Slice 37 proof — a governed memory source, and what it may not do without.
//!
//! Emits a report proving four things about the new source class:
//!
//! 1. a memory source carrying provenance assembles, deterministically;
//! 2. one without provenance is refused, and refused for *that* reason rather
//!    than for whichever unrelated rule it happens to trip first;
//! 3. a profile that does not allow the class cannot be handed one — admission
//!    stays double-gated, which is why widening the enum was safe;
//! 4. the two bundle hashes captured before this slice have not moved.
//!
//! Nothing is published on a rejected path: the report records the refusals as
//! outcomes, and the refused requests produce no manifest.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use precomputed_context_core::{
    assemble_context, AuthorityState, ContextAssemblyError, ContextAssemblyRequest,
    ContextBundleManifest, FreshnessPolicy, OverridePosture, SourceClass, SourceInput,
    SourceProvenance, TargetRefs,
};
use serde::Serialize;

/// Captured from `master` at `b4868c5`, before this slice existed.
const GOLDEN_PHASE1_HASH: &str = "f821a73703a5ec8d";
const GOLDEN_CONTINUITY_HASH: &str = "81419b53ca63648c";

#[derive(Debug, Serialize)]
struct RefusalOutcome {
    case: String,
    refused: bool,
    error: String,
    published_manifest: bool,
}

#[derive(Debug, Serialize)]
struct Slice37Report {
    schema_version: String,
    admitted_manifest: ContextBundleManifest,
    repeated_bundle_hash: String,
    deterministic_bundle_hash: bool,
    provenance_bound_into_identity: bool,
    refusals: Vec<RefusalOutcome>,
    preexisting_hashes_unmoved: bool,
}

fn provenance() -> SourceProvenance {
    SourceProvenance {
        memory_ref: "memory_fact:f7c1a2d8-0f3b-4a91-9c22-6d0e5b8a1c34:v1".to_string(),
        retrieval_receipt_ref: "memory_retrieval_receipt:9b2e1c40-77a5-4d63-b8f1-2ac6de905331:v1"
            .to_string(),
        authority_ref: "memory_residency_policy:pol-local:v1".to_string(),
    }
}

fn memory_source(provenance: Option<SourceProvenance>) -> SourceInput {
    SourceInput {
        payload_ref: "mem://fact/amara-age".to_string(),
        source_class: SourceClass::GovernedMemoryFact,
        age_minutes: 40_320,
        authority_state: AuthorityState::Accepted,
        is_override: false,
        provenance,
    }
}

/// A memory-only bundle. `validate_required_target_refs` only requires a target
/// ref the request actually names, so one naming none assembles cleanly — the
/// interim that makes memory usable while bundle-wide freshness is still a
/// single number governing every class at once.
fn memory_request(sources: Vec<SourceInput>) -> ContextAssemblyRequest {
    ContextAssemblyRequest {
        task_intent_id: "ti_memory_0001".to_string(),
        task_family: "analysis".to_string(),
        task_version: "analyze.memory.v1".to_string(),
        target_refs: TargetRefs {
            active_scene_ref: None,
            adjacent_scene_ref: None,
            accepted_lore_record_refs: vec![],
            accepted_style_rule_refs: vec![],
        },
        allowed_source_classes: vec![SourceClass::GovernedMemoryFact],
        freshness_policy: FreshnessPolicy {
            max_source_age_minutes: 525_600,
        },
        override_posture: OverridePosture::DisallowAll,
        sources,
    }
}

fn manuscript_source(payload_ref: &str, class: SourceClass, age_minutes: u64) -> SourceInput {
    SourceInput {
        payload_ref: payload_ref.to_string(),
        source_class: class,
        age_minutes,
        authority_state: AuthorityState::Accepted,
        is_override: false,
        provenance: None,
    }
}

fn phase1_base_request() -> ContextAssemblyRequest {
    ContextAssemblyRequest {
        task_intent_id: "ti_proofread_0001".to_string(),
        task_family: "proofread".to_string(),
        task_version: "lore_safe.v1".to_string(),
        target_refs: TargetRefs {
            active_scene_ref: Some("scene://chapter-03/scene-07".to_string()),
            adjacent_scene_ref: Some("scene-summary://chapter-03/scene-06".to_string()),
            accepted_lore_record_refs: vec!["lore://canon/character/amara".to_string()],
            accepted_style_rule_refs: vec!["style://house/minimize-adverbs".to_string()],
        },
        allowed_source_classes: vec![
            SourceClass::ActiveScene,
            SourceClass::AdjacentSceneSummaryOrClippedBody,
            SourceClass::AcceptedLoreRecord,
            SourceClass::AcceptedStyleRuleRecord,
        ],
        freshness_policy: FreshnessPolicy {
            max_source_age_minutes: 120,
        },
        override_posture: OverridePosture::DisallowAll,
        sources: vec![
            manuscript_source("scene://chapter-03/scene-07", SourceClass::ActiveScene, 3),
            manuscript_source(
                "scene-summary://chapter-03/scene-06",
                SourceClass::AdjacentSceneSummaryOrClippedBody,
                15,
            ),
            manuscript_source(
                "lore://canon/character/amara",
                SourceClass::AcceptedLoreRecord,
                11,
            ),
            manuscript_source(
                "style://house/minimize-adverbs",
                SourceClass::AcceptedStyleRuleRecord,
                8,
            ),
        ],
    }
}

fn continuity_request() -> ContextAssemblyRequest {
    ContextAssemblyRequest {
        task_intent_id: "ti_analyze_continuity_0001".to_string(),
        task_family: "analysis".to_string(),
        task_version: "analyze.continuity.adjacent_scene.v1".to_string(),
        target_refs: TargetRefs {
            active_scene_ref: Some("scene://chapter-05/scene-02".to_string()),
            adjacent_scene_ref: Some("scene-summary://chapter-05/scene-01".to_string()),
            accepted_lore_record_refs: vec!["lore://canon/character/amara".to_string()],
            accepted_style_rule_refs: vec!["style://house/keep-sentences-clean".to_string()],
        },
        allowed_source_classes: vec![
            SourceClass::ActiveScene,
            SourceClass::AdjacentSceneSummaryOrClippedBody,
            SourceClass::AcceptedLoreRecord,
            SourceClass::AcceptedStyleRuleRecord,
        ],
        freshness_policy: FreshnessPolicy {
            max_source_age_minutes: 120,
        },
        override_posture: OverridePosture::DisallowAll,
        sources: vec![
            manuscript_source("scene://chapter-05/scene-02", SourceClass::ActiveScene, 4),
            manuscript_source(
                "scene-summary://chapter-05/scene-01",
                SourceClass::AdjacentSceneSummaryOrClippedBody,
                18,
            ),
            manuscript_source(
                "lore://canon/character/amara",
                SourceClass::AcceptedLoreRecord,
                9,
            ),
            manuscript_source(
                "style://house/keep-sentences-clean",
                SourceClass::AcceptedStyleRuleRecord,
                6,
            ),
        ],
    }
}

/// Record a refusal, and assert it is the refusal that was expected.
///
/// A rejected path that fails for the wrong reason still looks green from the
/// outside, so the expected error is named rather than merely counted.
fn expect_refusal(
    case: &str,
    request: &ContextAssemblyRequest,
    expected: fn(&ContextAssemblyError) -> bool,
) -> Result<RefusalOutcome, Box<dyn Error>> {
    match assemble_context(request) {
        Ok(_) => Err(format!("{case}: expected refusal, assembly succeeded").into()),
        Err(err) if expected(&err) => Ok(RefusalOutcome {
            case: case.to_string(),
            refused: true,
            error: err.to_string(),
            published_manifest: false,
        }),
        Err(err) => Err(format!("{case}: refused for the wrong reason: {err}").into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let admitted_request = memory_request(vec![memory_source(Some(provenance()))]);
    let first = assemble_context(&admitted_request)?;
    let second = assemble_context(&admitted_request)?;

    if first.manifest.bundle_hash != second.manifest.bundle_hash {
        return Err("memory bundle_hash not deterministic across repeated assembly".into());
    }

    // Provenance must be bound into identity rather than merely carried: change
    // the receipt that supplied the memory, and the bundle must stop being the
    // same bundle.
    let mut other = provenance();
    other.retrieval_receipt_ref =
        "memory_retrieval_receipt:00000000-0000-4000-8000-000000000001:v1".to_string();
    let rebound = assemble_context(&memory_request(vec![memory_source(Some(other))]))?;
    let provenance_bound_into_identity = rebound.manifest.bundle_hash != first.manifest.bundle_hash;
    if !provenance_bound_into_identity {
        return Err("provenance is recorded but not bound into the bundle identity".into());
    }

    let mut refusals = Vec::new();

    refusals.push(expect_refusal(
        "memory source without provenance",
        &memory_request(vec![memory_source(None)]),
        |e| matches!(e, ContextAssemblyError::MissingProvenance { .. }),
    )?);

    // Same defect, under a scene-sized freshness policy the source also fails.
    // The provenance error must win: an error naming the wrong rule sends a
    // reader to fix something that is not broken.
    let mut also_stale = memory_request(vec![memory_source(None)]);
    also_stale.freshness_policy = FreshnessPolicy {
        max_source_age_minutes: 120,
    };
    refusals.push(expect_refusal(
        "missing provenance outranks staleness",
        &also_stale,
        |e| matches!(e, ContextAssemblyError::MissingProvenance { .. }),
    )?);

    let mut disallowed = memory_request(vec![memory_source(Some(provenance()))]);
    disallowed.allowed_source_classes = vec![SourceClass::AcceptedLoreRecord];
    refusals.push(expect_refusal(
        "profile does not allow the memory class",
        &disallowed,
        |e| {
            matches!(
                e,
                ContextAssemblyError::UnsupportedSourceClass {
                    source_class: SourceClass::GovernedMemoryFact
                }
            )
        },
    )?);

    // Nothing that assembled before this slice assembles differently.
    let phase1 = assemble_context(&phase1_base_request())?;
    let continuity = assemble_context(&continuity_request())?;
    let preexisting_hashes_unmoved = phase1.manifest.bundle_hash == GOLDEN_PHASE1_HASH
        && continuity.manifest.bundle_hash == GOLDEN_CONTINUITY_HASH;
    if !preexisting_hashes_unmoved {
        return Err(format!(
            "a pre-existing bundle hash moved: phase1={} (expected {GOLDEN_PHASE1_HASH}), \
             continuity={} (expected {GOLDEN_CONTINUITY_HASH})",
            phase1.manifest.bundle_hash, continuity.manifest.bundle_hash
        )
        .into());
    }

    let report = Slice37Report {
        schema_version: "proof.slice37-governed-memory-source.v1".to_string(),
        admitted_manifest: first.manifest.clone(),
        repeated_bundle_hash: second.manifest.bundle_hash.clone(),
        deterministic_bundle_hash: true,
        provenance_bound_into_identity,
        refusals,
        preexisting_hashes_unmoved,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice37_governed_memory_source/governed_memory_source_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

    println!("{}", report_path.display());
    Ok(())
}

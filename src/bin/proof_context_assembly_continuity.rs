use precomputed_context_core::{
    assemble_context, AuthorityState, ContextAssemblyRequest, ContextBundleManifest, FreshnessPolicy,
    OverridePosture, SourceClass, SourceInput, TargetRefs,
};
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct ContinuityManifestReport {
    schema_version: String,
    task_family: String,
    task_version: String,
    manifest: ContextBundleManifest,
    repeated_bundle_hash: String,
    deterministic_bundle_hash: bool,
}

/// Continuity context-assembly profile request.
///
/// task_family = "analysis", task_version = "analyze.continuity.adjacent_scene.v1".
/// Minimum-viable mapping: later/active scene -> active_scene_ref,
/// earlier scene (summary/clipped) -> adjacent_scene_ref.
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
        freshness_policy: FreshnessPolicy::uniform(120),
        override_posture: OverridePosture::DisallowAll,
        sources: vec![
            SourceInput {
                payload_ref: "scene://chapter-05/scene-02".to_string(),
                source_class: SourceClass::ActiveScene,
                age_minutes: 4,
                authority_state: AuthorityState::Accepted,
                is_override: false,
                provenance: None,
            },
            SourceInput {
                payload_ref: "scene-summary://chapter-05/scene-01".to_string(),
                source_class: SourceClass::AdjacentSceneSummaryOrClippedBody,
                age_minutes: 18,
                authority_state: AuthorityState::Accepted,
                is_override: false,
                provenance: None,
            },
            SourceInput {
                payload_ref: "lore://canon/character/amara".to_string(),
                source_class: SourceClass::AcceptedLoreRecord,
                age_minutes: 9,
                authority_state: AuthorityState::Accepted,
                is_override: false,
                provenance: None,
            },
            SourceInput {
                payload_ref: "style://house/keep-sentences-clean".to_string(),
                source_class: SourceClass::AcceptedStyleRuleRecord,
                age_minutes: 6,
                authority_state: AuthorityState::Accepted,
                is_override: false,
                provenance: None,
            },
        ],
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let request = continuity_request();

    let first = assemble_context(&request)?;
    let second = assemble_context(&request)?;

    let first_hash = first.manifest.bundle_hash.clone();
    let second_hash = second.manifest.bundle_hash.clone();

    if first_hash != second_hash {
        return Err("continuity bundle_hash not deterministic across repeated assembly".into());
    }

    let report = ContinuityManifestReport {
        schema_version: "proof.context-assembly-continuity.v1".to_string(),
        task_family: request.task_family.clone(),
        task_version: request.task_version.clone(),
        manifest: first.manifest.clone(),
        repeated_bundle_hash: second_hash.clone(),
        deterministic_bundle_hash: first_hash == second_hash,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/context_assembly_continuity/continuity_manifest_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

    println!("{}", report_path.display());
    Ok(())
}

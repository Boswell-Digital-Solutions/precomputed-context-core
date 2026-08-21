use precomputed_context_core::{
    assemble_context, AuthorityState, ContextAssemblyRequest, FreshnessPolicy, OverrideDecision,
    OverridePosture, SourceClass, SourceInput, TargetRefs,
};
use precomputed_context_core::schema_bundle::schema_catalog;

fn base_request() -> ContextAssemblyRequest {
    ContextAssemblyRequest {
        task_intent_id: "ti_proofread_0002".to_string(),
        task_family: "proofread".to_string(),
        task_version: "lore_safe.v1".to_string(),
        target_refs: TargetRefs {
            active_scene_ref: Some("scene://chapter-04/scene-01".to_string()),
            adjacent_scene_ref: Some("scene-summary://chapter-03/scene-09".to_string()),
            accepted_lore_record_refs: vec!["lore://canon/location/stillwater".to_string()],
            accepted_style_rule_refs: vec!["style://house/keep-sentences-clean".to_string()],
        },
        allowed_source_classes: vec![
            SourceClass::ActiveScene,
            SourceClass::AdjacentSceneSummaryOrClippedBody,
            SourceClass::AcceptedLoreRecord,
            SourceClass::AcceptedStyleRuleRecord,
        ],
        freshness_policy: FreshnessPolicy::uniform(180),
        override_posture: OverridePosture::AllowAcceptedStyleRuleRecords,
        sources: vec![
            SourceInput {
                payload_ref: "scene://chapter-04/scene-01".to_string(),
                source_class: SourceClass::ActiveScene,
                age_minutes: 2,
                authority_state: AuthorityState::Accepted,
                is_override: false,
                provenance: None,
            },
            SourceInput {
                payload_ref: "scene-summary://chapter-03/scene-09".to_string(),
                source_class: SourceClass::AdjacentSceneSummaryOrClippedBody,
                age_minutes: 14,
                authority_state: AuthorityState::Accepted,
                is_override: false,
                provenance: None,
            },
            SourceInput {
                payload_ref: "lore://canon/location/stillwater".to_string(),
                source_class: SourceClass::AcceptedLoreRecord,
                age_minutes: 12,
                authority_state: AuthorityState::Accepted,
                is_override: false,
                provenance: None,
            },
            SourceInput {
                payload_ref: "style://house/keep-sentences-clean".to_string(),
                source_class: SourceClass::AcceptedStyleRuleRecord,
                age_minutes: 7,
                authority_state: AuthorityState::Accepted,
                is_override: true,
                provenance: None,
            },
        ],
    }
}

#[test]
fn crate_root_exports_context_assembly_surface() {
    let output = assemble_context(&base_request()).expect("crate-root exported assembly should succeed");
    assert_eq!(
        output.manifest.override_decision,
        OverrideDecision::AllowedStyleRuleOverrideUsed
    );
    assert_eq!(output.payload_refs.len(), 4);
}

#[test]
fn schema_catalog_includes_phase1_context_entries() {
    let names: Vec<&'static str> = schema_catalog().into_iter().map(|(name, _)| name).collect();

    assert!(names.contains(&"context_assembly_request.schema.json"));
    assert!(names.contains(&"context_bundle_manifest.schema.json"));
}

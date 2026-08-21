use precomputed_context_core::{
    assemble_context, AuthorityState, ContextAssemblyError, ContextAssemblyRequest,
    FreshnessPolicy, OverridePosture, SourceClass, SourceInput, TargetRefs,
};

/// Continuity context-assembly profile request builder.
///
/// task_family = "analysis", task_version = "analyze.continuity.adjacent_scene.v1".
/// Minimum-viable mapping: later/active scene -> active_scene_ref,
/// earlier scene (summary/clipped) -> adjacent_scene_ref. Both are required-enforced.
fn continuity_request() -> ContextAssemblyRequest {
    ContextAssemblyRequest {
        task_intent_id: "ti_analyze_continuity_0001".to_string(),
        task_family: "analysis".to_string(),
        task_version: "analyze.continuity.adjacent_scene.v1".to_string(),
        target_refs: TargetRefs {
            // later / active scene
            active_scene_ref: Some("scene://chapter-05/scene-02".to_string()),
            // earlier scene (summary / clipped body)
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

#[test]
fn continuity_assembly_is_deterministic_and_returns_manifest() {
    let request = continuity_request();
    let first = assemble_context(&request).expect("first continuity assembly should succeed");
    let second = assemble_context(&request).expect("second continuity assembly should succeed");

    // Deterministic across two calls: identical manifests and payload refs.
    assert_eq!(first.manifest, second.manifest);
    assert_eq!(first.payload_refs, second.payload_refs);
    assert_eq!(first.payload_refs.len(), 4);
    // context_bundle_id prefix contract.
    assert!(first.manifest.context_bundle_id.starts_with("ctxb_"));
}

#[test]
fn continuity_stale_source_is_rejected_fail_closed() {
    let mut request = continuity_request();
    // Push the active scene past the 120-minute freshness ceiling.
    request.sources[0].age_minutes = 121;

    let error = assemble_context(&request).expect_err("stale continuity source should fail closed");
    assert_eq!(
        error,
        ContextAssemblyError::StaleSource {
            payload_ref: "scene://chapter-05/scene-02".to_string(),
            age_minutes: 121,
            max_age_minutes: 120,
        }
    );
}

#[test]
fn continuity_fixture_declares_task_version() {
    let fixture = include_str!("../fixtures/context_assembly/valid_request_continuity.json");
    assert!(fixture.contains("\"task_version\": \"analyze.continuity.adjacent_scene.v1\""));
}

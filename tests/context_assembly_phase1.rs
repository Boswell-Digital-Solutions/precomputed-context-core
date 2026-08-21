#[path = "../src/context_assembly.rs"]
mod context_assembly;

use context_assembly::{
    assemble_context, AuthorityState, ContextAssemblyError, ContextAssemblyRequest, FreshnessBand,
    FreshnessPolicy, OverrideDecision, OverridePosture, SourceClass, SourceInput, TargetRefs,
};

fn base_request() -> ContextAssemblyRequest {
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
        freshness_policy: FreshnessPolicy::uniform(120),
        override_posture: OverridePosture::DisallowAll,
        sources: vec![
            SourceInput {
                payload_ref: "scene://chapter-03/scene-07".to_string(),
                source_class: SourceClass::ActiveScene,
                age_minutes: 3,
                authority_state: AuthorityState::Accepted,
                is_override: false,
                provenance: None,
            },
            SourceInput {
                payload_ref: "scene-summary://chapter-03/scene-06".to_string(),
                source_class: SourceClass::AdjacentSceneSummaryOrClippedBody,
                age_minutes: 15,
                authority_state: AuthorityState::Accepted,
                is_override: false,
                provenance: None,
            },
            SourceInput {
                payload_ref: "lore://canon/character/amara".to_string(),
                source_class: SourceClass::AcceptedLoreRecord,
                age_minutes: 11,
                authority_state: AuthorityState::Accepted,
                is_override: false,
                provenance: None,
            },
            SourceInput {
                payload_ref: "style://house/minimize-adverbs".to_string(),
                source_class: SourceClass::AcceptedStyleRuleRecord,
                age_minutes: 8,
                authority_state: AuthorityState::Accepted,
                is_override: false,
                provenance: None,
            },
        ],
    }
}

#[test]
fn valid_assembly_is_deterministic_and_returns_manifest() {
    let request = base_request();
    let first = assemble_context(&request).expect("first assembly should succeed");
    let second = assemble_context(&request).expect("second assembly should succeed");

    assert_eq!(first.manifest, second.manifest);
    assert_eq!(first.payload_refs, second.payload_refs);
    assert_eq!(
        first.manifest.override_decision,
        OverrideDecision::NoOverridePresent
    );
    assert_eq!(first.manifest.freshness_band, FreshnessBand::Fresh);
    assert!(!first.manifest.authority_conflict_flag);
    assert_eq!(first.payload_refs.len(), 4);
    // Tagged with the algorithm that produced it, and still carrying the
    // identity it would have had before Slice 39 so a pack stored under the old
    // scheme stays findable.
    assert!(first.manifest.context_bundle_id.starts_with("ctxb.sha256."));
    assert!(first.manifest.legacy_context_bundle_id.starts_with("ctxb_"));
}

#[test]
fn stale_context_is_rejected_fail_closed() {
    let mut request = base_request();
    request.sources[0].age_minutes = 121;

    let error = assemble_context(&request).expect_err("stale source should fail closed");
    assert_eq!(
        error,
        ContextAssemblyError::StaleSource {
            payload_ref: "scene://chapter-03/scene-07".to_string(),
            age_minutes: 121,
            max_age_minutes: 120,
        }
    );
}

#[test]
fn authority_conflict_is_rejected_fail_closed() {
    let mut request = base_request();
    request.sources[2].authority_state = AuthorityState::ConflictUnresolved;

    let error = assemble_context(&request).expect_err("unresolved conflict should fail closed");
    assert_eq!(
        error,
        ContextAssemblyError::AuthorityConflictUnresolved {
            payload_ref: "lore://canon/character/amara".to_string(),
        }
    );
}

#[test]
fn allowed_style_rule_override_path_is_permitted() {
    let mut request = base_request();
    request.override_posture = OverridePosture::AllowAcceptedStyleRuleRecords;
    request.sources[3].is_override = true;
    request.sources[3].authority_state = AuthorityState::ConflictResolved;

    let output = assemble_context(&request).expect("style override should be allowed");
    assert_eq!(
        output.manifest.override_decision,
        OverrideDecision::AllowedStyleRuleOverrideUsed
    );
    assert!(output.manifest.authority_conflict_flag);
    assert_eq!(output.manifest.freshness_band, FreshnessBand::Fresh);
}

#[test]
fn disallowed_source_rejection_is_fail_closed() {
    let mut request = base_request();
    request
        .allowed_source_classes
        .push(SourceClass::ExperimentalFutureSource);

    let error =
        assemble_context(&request).expect_err("unsupported source class should fail closed");
    assert_eq!(
        error,
        ContextAssemblyError::UnsupportedSourceClass {
            source_class: SourceClass::ExperimentalFutureSource,
        }
    );
}

#[test]
fn missing_required_source_is_rejected_fail_closed() {
    let mut request = base_request();
    request
        .sources
        .retain(|source| source.payload_ref != "scene-summary://chapter-03/scene-06");

    let error =
        assemble_context(&request).expect_err("missing adjacent scene summary should fail closed");
    assert_eq!(
        error,
        ContextAssemblyError::MissingRequiredSource {
            payload_ref: "scene-summary://chapter-03/scene-06".to_string(),
            source_class: SourceClass::AdjacentSceneSummaryOrClippedBody,
        }
    );
}

#[test]
fn fixture_bundle_is_present_for_next_slice_authority() {
    let valid_manifest = include_str!("../fixtures/context_assembly/valid_manifest.json");
    let stale_case = include_str!("../fixtures/context_assembly/invalid_stale_context.json");
    let conflict_case =
        include_str!("../fixtures/context_assembly/invalid_authority_conflict.json");
    let valid_request = include_str!("../fixtures/context_assembly/valid_request.json");

    assert!(valid_request.contains("\"task_intent_id\""));
    assert!(valid_manifest.contains("\"context_bundle_id\""));
    assert!(valid_manifest.contains("\"bundle_hash\""));
    assert!(stale_case.contains("\"expected_failure\": \"stale_source\""));
    assert!(conflict_case.contains("\"expected_failure\": \"authority_conflict_unresolved\""));
}

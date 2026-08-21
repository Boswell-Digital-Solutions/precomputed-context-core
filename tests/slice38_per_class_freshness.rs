//! Slice 38 — per-class freshness limits.
//!
//! One `max_source_age_minutes` could not serve both an active scene, which is
//! stale in minutes, and a governed memory fact, which earns its value by
//! persisting. Set for scenes it refused every memory fact; set for memory it
//! admitted a stale scene. Slice 37 shipped with a memory-only bundle and a
//! one-year limit as the workable interim; this slice removes the need for it.
//!
//! The claim that has to be *proven* rather than asserted is that nothing
//! already assembling changes. A policy with no overrides must behave exactly as
//! it did — same admissions, same refusals, same freshness band, same bundle
//! hash. The band computation was rewritten from "the oldest source against the
//! single limit" to "any source against its own limit", and those two are only
//! equivalent while the limits are equal, so the equivalence is checked directly
//! rather than reasoned about.

use precomputed_context_core::{
    assemble_context, AuthorityState, ClassFreshnessOverride, ContextAssemblyError,
    ContextAssemblyRequest, FreshnessBand, FreshnessPolicy, OverridePosture, SourceClass,
    SourceInput, SourceProvenance, TargetRefs,
};

/// Bundle hashes captured on `master` at the tip of Slice 37, before this slice.
const GOLDEN_PHASE1_HASH: &str = "f821a73703a5ec8d";
const GOLDEN_CONTINUITY_HASH: &str = "81419b53ca63648c";

const SCENE: &str = "scene://chapter-05/scene-02";
const ADJACENT: &str = "scene-summary://chapter-05/scene-01";
const MEMORY: &str = "memory://fact/f-contract-authority";

fn provenance() -> SourceProvenance {
    SourceProvenance {
        memory_ref: "memory_fact:f-contract-authority:v1".to_string(),
        retrieval_receipt_ref: "memory_retrieval_receipt:r-0001:v1".to_string(),
        authority_ref: "memory_residency_policy:pol-local:v1".to_string(),
    }
}

fn source(payload_ref: &str, class: SourceClass, age_minutes: u64) -> SourceInput {
    let provenance = class.requires_provenance().then(provenance);
    SourceInput {
        payload_ref: payload_ref.to_string(),
        source_class: class,
        age_minutes,
        authority_state: AuthorityState::Accepted,
        is_override: false,
        provenance,
    }
}

/// A scene plus a memory fact in one bundle — the case that could not be
/// expressed before this slice.
fn mixed_request(
    policy: FreshnessPolicy,
    scene_age: u64,
    memory_age: u64,
) -> ContextAssemblyRequest {
    ContextAssemblyRequest {
        task_intent_id: "ti_mixed_0001".to_string(),
        task_family: "analysis".to_string(),
        task_version: "analyze.mixed.v1".to_string(),
        target_refs: TargetRefs {
            active_scene_ref: Some(SCENE.to_string()),
            adjacent_scene_ref: None,
            accepted_lore_record_refs: vec![],
            accepted_style_rule_refs: vec![],
        },
        allowed_source_classes: vec![SourceClass::ActiveScene, SourceClass::GovernedMemoryFact],
        freshness_policy: policy,
        override_posture: OverridePosture::DisallowAll,
        sources: vec![
            source(SCENE, SourceClass::ActiveScene, scene_age),
            source(MEMORY, SourceClass::GovernedMemoryFact, memory_age),
        ],
    }
}

fn memory_override(minutes: u64) -> FreshnessPolicy {
    FreshnessPolicy {
        max_source_age_minutes: 120,
        class_overrides: Some(vec![ClassFreshnessOverride {
            source_class: SourceClass::GovernedMemoryFact,
            max_source_age_minutes: minutes,
        }]),
    }
}

// ── The gap this slice closes ────────────────────────────────────────────────

#[test]
fn a_scene_appropriate_limit_used_to_refuse_every_memory_fact() {
    // The problem, stated as a test so it cannot quietly stop being true.
    let request = mixed_request(FreshnessPolicy::uniform(120), 4, 10_000);
    match assemble_context(&request) {
        Err(ContextAssemblyError::StaleSource {
            payload_ref,
            max_age_minutes,
            ..
        }) => {
            assert_eq!(payload_ref, MEMORY);
            assert_eq!(max_age_minutes, 120);
        }
        other => panic!("expected the memory fact to be refused as stale, got {other:?}"),
    }
}

#[test]
fn a_per_class_limit_admits_the_memory_fact_and_still_holds_the_scene() {
    let request = mixed_request(memory_override(525_600), 4, 10_000);
    let output = assemble_context(&request).expect("a mixed bundle assembles");
    assert_eq!(output.manifest.source_inventory.len(), 2);

    // The scene is still governed by the bundle-wide limit, not the override.
    let stale_scene = mixed_request(memory_override(525_600), 121, 10_000);
    match assemble_context(&stale_scene) {
        Err(ContextAssemblyError::StaleSource {
            payload_ref,
            max_age_minutes,
            ..
        }) => {
            assert_eq!(payload_ref, SCENE);
            assert_eq!(
                max_age_minutes, 120,
                "an override for one class must not slacken another"
            );
        }
        other => panic!("expected the scene to be refused as stale, got {other:?}"),
    }
}

#[test]
fn the_error_names_the_limit_that_actually_refused_the_source() {
    // Not the bundle-wide default, which would send a reader looking at the
    // wrong number entirely.
    let request = mixed_request(memory_override(60), 4, 61);
    match assemble_context(&request) {
        Err(ContextAssemblyError::StaleSource {
            payload_ref,
            age_minutes,
            max_age_minutes,
        }) => {
            assert_eq!(payload_ref, MEMORY);
            assert_eq!(age_minutes, 61);
            assert_eq!(max_age_minutes, 60, "the override, not the default 120");
        }
        other => panic!("expected a stale memory fact, got {other:?}"),
    }
}

// ── Nothing that already assembles changes ───────────────────────────────────

#[test]
fn pre_existing_bundle_hashes_do_not_move() {
    let phase1 = assemble_context(&phase1_request()).expect("phase 1 assembles");
    assert_eq!(
        phase1.manifest.legacy_bundle_hash, GOLDEN_PHASE1_HASH,
        "the phase-1 profile's replay identity moved"
    );
    let continuity = assemble_context(&continuity_request()).expect("continuity assembles");
    assert_eq!(
        continuity.manifest.legacy_bundle_hash, GOLDEN_CONTINUITY_HASH,
        "the continuity profile's replay identity moved"
    );
}

#[test]
fn a_policy_without_overrides_serializes_as_it_always_did() {
    // The field is skipped when absent, so a policy written before this slice
    // and one written after are byte-identical on the wire.
    let rendered = serde_json::to_string(&FreshnessPolicy::uniform(120)).expect("serializable");
    assert_eq!(rendered, r#"{"max_source_age_minutes":120}"#);
}

#[test]
fn a_policy_written_before_this_slice_still_deserializes() {
    let policy: FreshnessPolicy =
        serde_json::from_str(r#"{"max_source_age_minutes":120}"#).expect("deserializable");
    assert_eq!(policy, FreshnessPolicy::uniform(120));
    assert_eq!(policy.max_for(&SourceClass::ActiveScene), 120);
    assert_eq!(policy.max_for(&SourceClass::GovernedMemoryFact), 120);
}

/// The band rule was rewritten from "the oldest source against the single limit"
/// to "any source against its own limit". Those agree only while the limits are
/// equal — so agreement is checked across the whole space rather than argued.
#[test]
fn the_band_rule_is_unchanged_wherever_no_override_is_present() {
    for limit in [0_u64, 1, 2, 3, 7, 120] {
        for a in 0..=limit {
            for b in 0..=limit {
                let request = two_scene_request(FreshnessPolicy::uniform(limit), a, b);
                let output = assemble_context(&request).expect("both sources are within the limit");

                // The rule this replaced, reproduced literally.
                let oldest = a.max(b);
                let expected = if limit == 0 || oldest * 2 < limit {
                    FreshnessBand::Fresh
                } else {
                    FreshnessBand::NearLimit
                };
                assert_eq!(
                    output.manifest.freshness_band, expected,
                    "limit={limit} ages=({a},{b})"
                );
            }
        }
    }
}

#[test]
fn the_band_follows_the_source_nearest_its_own_limit_not_the_oldest() {
    // The scene is younger in absolute minutes but far closer to its own limit.
    // Under the old rule the memory fact's age would have decided the band; that
    // is precisely what per-class limits make wrong.
    let request = mixed_request(memory_override(525_600), 60, 10_000);
    let output = assemble_context(&request).expect("assembles");
    assert_eq!(
        output.manifest.freshness_band,
        FreshnessBand::NearLimit,
        "the scene is at 50% of its own 120-minute limit"
    );

    let request = mixed_request(memory_override(525_600), 4, 10_000);
    let output = assemble_context(&request).expect("assembles");
    assert_eq!(
        output.manifest.freshness_band,
        FreshnessBand::Fresh,
        "neither source is near its own limit, though one is 10,000 minutes old"
    );
}

#[test]
fn a_zero_limit_does_not_report_near_limit() {
    // A source under a zero limit must be zero minutes old to be admitted at
    // all, and the bundle-wide rule called that Fresh. Carried forward.
    let request = two_scene_request(FreshnessPolicy::uniform(0), 0, 0);
    let output = assemble_context(&request).expect("assembles");
    assert_eq!(output.manifest.freshness_band, FreshnessBand::Fresh);

    let request = mixed_request(memory_override(0), 4, 0);
    let output = assemble_context(&request).expect("assembles");
    assert_eq!(
        output.manifest.freshness_band,
        FreshnessBand::Fresh,
        "a zero override contributes nothing to the band either"
    );
}

// ── A policy that cannot be read one way is refused ──────────────────────────

#[test]
fn one_class_may_not_carry_two_limits() {
    let mut policy = memory_override(600);
    policy
        .class_overrides
        .as_mut()
        .expect("overrides")
        .push(ClassFreshnessOverride {
            source_class: SourceClass::GovernedMemoryFact,
            max_source_age_minutes: 10,
        });

    match assemble_context(&mixed_request(policy, 4, 5)) {
        Err(ContextAssemblyError::DuplicateFreshnessOverride { source_class }) => {
            assert_eq!(source_class, SourceClass::GovernedMemoryFact);
        }
        other => panic!("expected a duplicate override to be refused, got {other:?}"),
    }
}

#[test]
fn an_override_for_an_unusable_class_is_refused_rather_than_ignored() {
    // Dead configuration is what a typo looks like.
    let policy = FreshnessPolicy {
        max_source_age_minutes: 120,
        class_overrides: Some(vec![ClassFreshnessOverride {
            source_class: SourceClass::ExperimentalFutureSource,
            max_source_age_minutes: 10,
        }]),
    };
    match assemble_context(&mixed_request(policy, 4, 5)) {
        Err(ContextAssemblyError::UnsupportedFreshnessOverride { source_class }) => {
            assert_eq!(source_class, SourceClass::ExperimentalFutureSource);
        }
        other => panic!("expected an unusable override class to be refused, got {other:?}"),
    }
}

#[test]
fn the_policy_is_validated_before_any_source_is_considered() {
    // Otherwise a request with both a bad policy and a stale source would report
    // whichever the loop reached first, which is an implementation detail rather
    // than an answer.
    let mut policy = memory_override(600);
    policy
        .class_overrides
        .as_mut()
        .expect("overrides")
        .push(ClassFreshnessOverride {
            source_class: SourceClass::GovernedMemoryFact,
            max_source_age_minutes: 10,
        });

    let request = mixed_request(policy, 9_999, 9_999);
    assert!(matches!(
        assemble_context(&request),
        Err(ContextAssemblyError::DuplicateFreshnessOverride { .. })
    ));
}

#[test]
fn an_empty_override_list_is_the_bundle_wide_policy() {
    let policy = FreshnessPolicy {
        max_source_age_minutes: 120,
        class_overrides: Some(vec![]),
    };
    assert_eq!(policy.max_for(&SourceClass::GovernedMemoryFact), 120);
    assert!(assemble_context(&mixed_request(policy, 4, 121)).is_err());
}

// ── Fixtures for the pre-existing profiles, matching their live definitions ──

fn scene_source(payload_ref: &str, class: SourceClass, age: u64) -> SourceInput {
    source(payload_ref, class, age)
}

fn two_scene_request(
    policy: FreshnessPolicy,
    active_age: u64,
    adjacent_age: u64,
) -> ContextAssemblyRequest {
    ContextAssemblyRequest {
        task_intent_id: "ti_band_0001".to_string(),
        task_family: "analysis".to_string(),
        task_version: "analyze.band.v1".to_string(),
        target_refs: TargetRefs {
            active_scene_ref: Some(SCENE.to_string()),
            adjacent_scene_ref: Some(ADJACENT.to_string()),
            accepted_lore_record_refs: vec![],
            accepted_style_rule_refs: vec![],
        },
        allowed_source_classes: vec![
            SourceClass::ActiveScene,
            SourceClass::AdjacentSceneSummaryOrClippedBody,
        ],
        freshness_policy: policy,
        override_posture: OverridePosture::DisallowAll,
        sources: vec![
            scene_source(SCENE, SourceClass::ActiveScene, active_age),
            scene_source(
                ADJACENT,
                SourceClass::AdjacentSceneSummaryOrClippedBody,
                adjacent_age,
            ),
        ],
    }
}

/// Mirrors `tests/context_assembly_phase1.rs::base_request`.
fn phase1_request() -> ContextAssemblyRequest {
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
            source("scene://chapter-03/scene-07", SourceClass::ActiveScene, 3),
            source(
                "scene-summary://chapter-03/scene-06",
                SourceClass::AdjacentSceneSummaryOrClippedBody,
                15,
            ),
            source(
                "lore://canon/character/amara",
                SourceClass::AcceptedLoreRecord,
                11,
            ),
            source(
                "style://house/minimize-adverbs",
                SourceClass::AcceptedStyleRuleRecord,
                8,
            ),
        ],
    }
}

/// Mirrors `src/bin/proof_context_assembly_continuity.rs::continuity_request`.
fn continuity_request() -> ContextAssemblyRequest {
    ContextAssemblyRequest {
        task_intent_id: "ti_analyze_continuity_0001".to_string(),
        task_family: "analysis".to_string(),
        task_version: "analyze.continuity.adjacent_scene.v1".to_string(),
        target_refs: TargetRefs {
            active_scene_ref: Some(SCENE.to_string()),
            adjacent_scene_ref: Some(ADJACENT.to_string()),
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
            source(SCENE, SourceClass::ActiveScene, 4),
            source(ADJACENT, SourceClass::AdjacentSceneSummaryOrClippedBody, 18),
            source(
                "lore://canon/character/amara",
                SourceClass::AcceptedLoreRecord,
                9,
            ),
            source(
                "style://house/keep-sentences-clean",
                SourceClass::AcceptedStyleRuleRecord,
                6,
            ),
        ],
    }
}

//! Slice 37 — a governed memory source class, and the provenance it must carry.
//!
//! Two things are being proved, and they pull in opposite directions.
//!
//! The **new** behavior: memory can enter a context bundle as itself, labelled
//! as memory, carrying the fact it came from, the receipt that supplied it, and
//! the authority it was used under — and a memory source that cannot say those
//! things is refused rather than admitted unlabelled.
//!
//! The **unchanged** behavior: nothing that assembled before this slice
//! assembles differently. That is not a claim to be taken on trust, so the two
//! bundle hashes captured from `master` at `b4868c5` — before the enum grew a
//! variant and before `SourceInput` grew a field — are asserted here as
//! goldens. If a future change moves them, this file says so.

use precomputed_context_core::{
    assemble_context, AuthorityState, ContextAssemblyError, ContextAssemblyRequest,
    FreshnessPolicy, OverridePosture, SourceClass, SourceInput, SourceProvenance, TargetRefs,
};

/// Captured from `master` at `b4868c5`, before this slice.
const GOLDEN_PHASE1_HASH: &str = "f821a73703a5ec8d";
/// Captured from the same revision by `proof_context_assembly_continuity`.
const GOLDEN_CONTINUITY_HASH: &str = "81419b53ca63648c";

fn provenance() -> SourceProvenance {
    SourceProvenance {
        memory_ref: "memory_fact:f7c1a2d8-0f3b-4a91-9c22-6d0e5b8a1c34:v1".to_string(),
        retrieval_receipt_ref: "memory_retrieval_receipt:9b2e1c40-77a5-4d63-b8f1-2ac6de905331:v1"
            .to_string(),
        authority_ref: "memory_residency_policy:pol-local:v1".to_string(),
    }
}

fn memory_source(payload_ref: &str, provenance: Option<SourceProvenance>) -> SourceInput {
    SourceInput {
        payload_ref: payload_ref.to_string(),
        source_class: SourceClass::GovernedMemoryFact,
        age_minutes: 40_320, // four weeks: memory earns its value by persisting
        authority_state: AuthorityState::Accepted,
        is_override: false,
        provenance,
    }
}

/// A memory-only bundle: no manuscript targets, and a freshness policy sized for
/// something that is supposed to be old.
///
/// This is the interim the RFC describes for P4. `validate_required_target_refs`
/// only requires a target ref the request actually names, so a request naming
/// none assembles cleanly — which is what makes memory usable before bundle-wide
/// freshness is revisited.
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
        // One year: the memory-only-bundle interim Slice 37 shipped with,
        // before per-class limits made it unnecessary (Slice 38).
        freshness_policy: FreshnessPolicy::uniform(525_600),
        override_posture: OverridePosture::DisallowAll,
        sources,
    }
}

// ── The class exists and is usable ───────────────────────────────────────────

#[test]
fn a_governed_memory_fact_is_phase1_allowed() {
    assert!(SourceClass::GovernedMemoryFact.is_phase1_allowed());
    assert_eq!(
        SourceClass::GovernedMemoryFact.as_str(),
        "governed_memory_fact"
    );
}

#[test]
fn a_memory_source_with_provenance_assembles() {
    let request = memory_request(vec![memory_source(
        "mem://fact/amara-age",
        Some(provenance()),
    )]);
    let out = assemble_context(&request).expect("a labelled memory source is admissible");

    assert_eq!(out.payload_refs, vec!["mem://fact/amara-age".to_string()]);
    let entry = &out.manifest.source_inventory[0];
    assert_eq!(entry.source_class, SourceClass::GovernedMemoryFact);
    assert_eq!(
        entry.provenance.as_ref().map(|p| p.memory_ref.as_str()),
        Some("memory_fact:f7c1a2d8-0f3b-4a91-9c22-6d0e5b8a1c34:v1"),
        "the manifest records which fact the bundle rested on"
    );
}

#[test]
fn assembly_stays_deterministic_with_memory_present() {
    let request = memory_request(vec![memory_source(
        "mem://fact/amara-age",
        Some(provenance()),
    )]);
    let first = assemble_context(&request).expect("first");
    let second = assemble_context(&request).expect("second");
    assert_eq!(first.manifest, second.manifest);
}

// ── And cannot be used unlabelled ────────────────────────────────────────────

#[test]
fn a_memory_source_without_provenance_is_refused() {
    let request = memory_request(vec![memory_source("mem://fact/amara-age", None)]);
    match assemble_context(&request) {
        Err(ContextAssemblyError::MissingProvenance {
            payload_ref,
            source_class,
        }) => {
            assert_eq!(payload_ref, "mem://fact/amara-age");
            assert_eq!(source_class, SourceClass::GovernedMemoryFact);
        }
        other => panic!("expected MissingProvenance, got {other:?}"),
    }
}

#[test]
fn provenance_is_refused_for_the_reason_that_applies() {
    // The source is also stale under a scene-sized policy. The missing
    // provenance is the real defect, and the error must name it rather than
    // whichever unrelated rule the source happens to trip first.
    let mut request = memory_request(vec![memory_source("mem://fact/amara-age", None)]);
    request.freshness_policy = FreshnessPolicy::uniform(120);
    assert!(matches!(
        assemble_context(&request),
        Err(ContextAssemblyError::MissingProvenance { .. })
    ));
}

#[test]
fn manuscript_classes_still_need_no_provenance() {
    assert!(!SourceClass::ActiveScene.requires_provenance());
    assert!(!SourceClass::AcceptedLoreRecord.requires_provenance());
    assert!(!SourceClass::AcceptedStyleRuleRecord.requires_provenance());
    assert!(SourceClass::GovernedMemoryFact.requires_provenance());
}

// ── Admission is still double-gated ──────────────────────────────────────────

#[test]
fn a_profile_that_does_not_allow_memory_cannot_receive_it() {
    // The reason widening the enum is safe: phase-1 permission makes the class
    // *available*, and the request's allowlist decides whether it is *supplied*.
    // A manuscript profile written before this slice lists no memory class, so
    // it cannot be handed one by accident.
    let mut request = memory_request(vec![memory_source(
        "mem://fact/amara-age",
        Some(provenance()),
    )]);
    request.allowed_source_classes = vec![SourceClass::AcceptedLoreRecord];

    assert!(matches!(
        assemble_context(&request),
        Err(ContextAssemblyError::UnsupportedSourceClass {
            source_class: SourceClass::GovernedMemoryFact
        })
    ));
}

#[test]
fn the_not_yet_placeholder_is_still_not_yet() {
    // Adding a real class beside ExperimentalFutureSource must not have
    // promoted the placeholder along with it.
    assert!(!SourceClass::ExperimentalFutureSource.is_phase1_allowed());
}

// ── Provenance is bound into identity, not merely recorded ───────────────────

#[test]
fn changing_the_provenance_changes_the_bundle() {
    let first = assemble_context(&memory_request(vec![memory_source(
        "mem://fact/amara-age",
        Some(provenance()),
    )]))
    .expect("first");

    let mut other = provenance();
    other.retrieval_receipt_ref =
        "memory_retrieval_receipt:00000000-0000-4000-8000-000000000001:v1".to_string();
    let second = assemble_context(&memory_request(vec![memory_source(
        "mem://fact/amara-age",
        Some(other),
    )]))
    .expect("second");

    assert_ne!(
        first.manifest.bundle_hash, second.manifest.bundle_hash,
        "a bundle must not keep its id while the receipt that supplied its memory changes; \
         provenance recorded but unhashed would be a label, not evidence"
    );
    assert_ne!(
        first.manifest.context_bundle_id,
        second.manifest.context_bundle_id
    );
}

// ── And nothing that assembled before assembles differently ──────────────────

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

/// Byte-for-byte the phase-1 request from `tests/context_assembly_phase1.rs`.
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
        freshness_policy: FreshnessPolicy::uniform(120),
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

/// Byte-for-byte the continuity request from
/// `src/bin/proof_context_assembly_continuity.rs`.
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

#[test]
fn no_existing_bundle_hash_moved() {
    let phase1 = assemble_context(&phase1_base_request()).expect("phase-1 assembly");
    assert_eq!(
        phase1.manifest.legacy_bundle_hash, GOLDEN_PHASE1_HASH,
        "a manuscript bundle assembled before this slice must hash identically after it"
    );
    assert_eq!(
        phase1.manifest.legacy_context_bundle_id,
        format!("ctxb_{GOLDEN_PHASE1_HASH}")
    );

    let continuity = assemble_context(&continuity_request()).expect("continuity assembly");
    assert_eq!(
        continuity.manifest.legacy_bundle_hash, GOLDEN_CONTINUITY_HASH,
        "the continuity profile's replay identity must survive this slice"
    );
    assert_eq!(
        continuity.manifest.legacy_context_bundle_id,
        format!("ctxb_{GOLDEN_CONTINUITY_HASH}")
    );
}

#[test]
fn a_manuscript_entry_serializes_exactly_as_it_did() {
    // skip_serializing_if keeps the absent field absent rather than emitting an
    // explicit null, so a manifest with no memory in it is byte-identical to one
    // produced before the field existed.
    let out = assemble_context(&phase1_base_request()).expect("assembly");
    let rendered = serde_json::to_string(&out.manifest).expect("serializable");
    assert!(
        !rendered.contains("provenance"),
        "a manuscript-only manifest must not gain a provenance key: {rendered}"
    );
}

#[test]
fn a_request_without_the_field_still_deserializes() {
    // Every ContextAssemblyRequest serialized before this slice omits
    // `provenance` entirely. serde(default) is what keeps those readable.
    let json = r#"{
        "task_intent_id": "ti_legacy",
        "task_family": "proofread",
        "task_version": "lore_safe.v1",
        "target_refs": {
            "active_scene_ref": "scene://chapter-01/scene-01",
            "adjacent_scene_ref": null,
            "accepted_lore_record_refs": [],
            "accepted_style_rule_refs": []
        },
        "allowed_source_classes": ["ActiveScene"],
        "freshness_policy": { "max_source_age_minutes": 120 },
        "override_posture": "DisallowAll",
        "sources": [{
            "payload_ref": "scene://chapter-01/scene-01",
            "source_class": "ActiveScene",
            "age_minutes": 3,
            "authority_state": "Accepted",
            "is_override": false
        }]
    }"#;
    let request: ContextAssemblyRequest =
        serde_json::from_str(json).expect("a request written before this slice still parses");
    assert!(request.sources[0].provenance.is_none());
    assemble_context(&request).expect("and still assembles");
}

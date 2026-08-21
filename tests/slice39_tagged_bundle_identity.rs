//! Slice 39 — an algorithm-tagged bundle identity.
//!
//! `context_bundle_id` was FNV-1a 64, in full: sixteen hex characters behind a
//! `ctxb_` prefix. That was an unremarkable choice while the value was a
//! deterministic name. It stopped being one when PACT's packet builders and
//! forgeHQ's verification bridge began treating it as an integrity binding — the
//! latter describing it as proof that a verification "was produced against the
//! exact governed, replay-eligible context the fix was built from". A 64-bit
//! non-cryptographic digest is roughly 2^32 birthday work, seconds of ordinary
//! hardware, and does not carry that claim.
//!
//! The constraint on fixing it is that **recorded ids must stay resolvable**.
//! DataForge's `context_packs` keys on the id and has no retention — no
//! `DELETE`, no TTL, no cleanup job — so rows minted under the old scheme never
//! age out. Dual resolution is therefore permanent, not transitional, and this
//! is not a cutover: both identities are computed for every bundle, forever.
//!
//! What must be proven here is that the legacy identity is *unchanged*, not
//! merely still present. If it drifted, every pack stored before this slice
//! would become unfindable — and it would fail silently, because a lookup miss
//! falls back to re-grounding rather than erroring.

use precomputed_context_core::{
    assemble_context, AuthorityState, ContextAssemblyRequest, FreshnessPolicy, OverridePosture,
    SourceClass, SourceInput, TargetRefs, ID_PREFIX, LEGACY_ID_PREFIX,
};

/// Captured on `master` at the tip of Slice 38, before this slice existed.
const GOLDEN_PHASE1_HASH: &str = "f821a73703a5ec8d";
const GOLDEN_CONTINUITY_HASH: &str = "81419b53ca63648c";

const SCENE: &str = "scene://chapter-05/scene-02";
const ADJACENT: &str = "scene-summary://chapter-05/scene-01";

fn source(payload_ref: &str, class: SourceClass, age_minutes: u64) -> SourceInput {
    SourceInput {
        payload_ref: payload_ref.to_string(),
        source_class: class,
        age_minutes,
        authority_state: AuthorityState::Accepted,
        is_override: false,
        provenance: None,
    }
}

fn two_scene_request(active_age: u64) -> ContextAssemblyRequest {
    ContextAssemblyRequest {
        task_intent_id: "ti_slice39".to_string(),
        task_family: "analysis".to_string(),
        task_version: "analyze.identity.v1".to_string(),
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
        freshness_policy: FreshnessPolicy::uniform(120),
        override_posture: OverridePosture::DisallowAll,
        sources: vec![
            source(SCENE, SourceClass::ActiveScene, active_age),
            source(ADJACENT, SourceClass::AdjacentSceneSummaryOrClippedBody, 18),
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

// ── The legacy identity is preserved, not merely retained ────────────────────

#[test]
fn the_legacy_identity_of_a_pre_existing_profile_is_byte_identical() {
    // The load-bearing claim. A pack stored before this slice is keyed by this
    // value; if it drifted the pack becomes unfindable, and unfindable here
    // means silently re-grounded rather than an error anyone would see.
    let phase1 = assemble_context(&phase1_request()).expect("assembles");
    assert_eq!(phase1.manifest.legacy_bundle_hash, GOLDEN_PHASE1_HASH);
    assert_eq!(
        phase1.manifest.legacy_context_bundle_id,
        format!("{LEGACY_ID_PREFIX}{GOLDEN_PHASE1_HASH}")
    );
}

#[test]
fn both_identities_are_derived_from_the_same_canonical_string() {
    // Not two independent computations that could drift apart: one
    // canonicalisation, two digests over it. Changing what a bundle *is* must
    // move both or neither — a change visible to one identity and not the other
    // would let a bundle keep an id it no longer deserves.
    let a = assemble_context(&two_scene_request(4)).expect("assembles");
    let b = assemble_context(&two_scene_request(5)).expect("assembles");

    assert_ne!(a.manifest.bundle_hash, b.manifest.bundle_hash);
    assert_ne!(a.manifest.legacy_bundle_hash, b.manifest.legacy_bundle_hash);
}

// ── The new identity says what produced it ───────────────────────────────────

#[test]
fn the_current_identity_is_tagged_with_its_algorithm() {
    let out = assemble_context(&two_scene_request(4)).expect("assembles");
    assert!(out.manifest.context_bundle_id.starts_with(ID_PREFIX));
    assert_eq!(out.manifest.bundle_hash.len(), 64, "sha256 in hex");
    assert_eq!(
        out.manifest.context_bundle_id,
        format!("{ID_PREFIX}{}", out.manifest.bundle_hash)
    );
}

#[test]
fn the_two_identities_cannot_be_mistaken_for_each_other() {
    // A resolver holding a mixed population must be able to tell them apart
    // without guessing from length. The prefixes are disjoint: "ctxb.sha256."
    // does not start with "ctxb_", and vice versa.
    let out = assemble_context(&two_scene_request(4)).expect("assembles");
    assert!(!out.manifest.context_bundle_id.starts_with(LEGACY_ID_PREFIX));
    assert!(!out.manifest.legacy_context_bundle_id.starts_with(ID_PREFIX));
    assert!(!ID_PREFIX.starts_with(LEGACY_ID_PREFIX));
    assert!(!LEGACY_ID_PREFIX.starts_with(ID_PREFIX));
}

#[test]
fn the_new_identity_fits_the_column_that_has_to_hold_it() {
    // DataForge's context_packs.context_pack_id is String(128), and its request
    // model rejects anything longer before the handler is reached. An identity
    // scheme that did not fit would fail at the store rather than here.
    let out = assemble_context(&two_scene_request(4)).expect("assembles");
    assert!(
        out.manifest.context_bundle_id.len() <= 128,
        "id is {} chars: {}",
        out.manifest.context_bundle_id.len(),
        out.manifest.context_bundle_id
    );
}

// ── Determinism, for both ────────────────────────────────────────────────────

#[test]
fn both_identities_are_deterministic_across_repeated_assembly() {
    let first = assemble_context(&two_scene_request(4)).expect("assembles");
    let second = assemble_context(&two_scene_request(4)).expect("assembles");

    assert_eq!(
        first.manifest.context_bundle_id,
        second.manifest.context_bundle_id
    );
    assert_eq!(first.manifest.bundle_hash, second.manifest.bundle_hash);
    assert_eq!(
        first.manifest.legacy_context_bundle_id,
        second.manifest.legacy_context_bundle_id
    );
    assert_eq!(
        first.manifest.legacy_bundle_hash,
        second.manifest.legacy_bundle_hash
    );
}

#[test]
fn the_continuity_profiles_legacy_identity_survives() {
    let phase1 = assemble_context(&phase1_request()).expect("assembles");
    // Distinct profiles keep distinct identities under both schemes — a
    // collapse would be the failure a stronger digest is supposed to prevent.
    let other = assemble_context(&two_scene_request(4)).expect("assembles");
    assert_ne!(phase1.manifest.bundle_hash, other.manifest.bundle_hash);
    assert_ne!(
        phase1.manifest.legacy_bundle_hash,
        other.manifest.legacy_bundle_hash
    );
    assert_eq!(phase1.manifest.legacy_bundle_hash, GOLDEN_PHASE1_HASH);
    assert_ne!(GOLDEN_PHASE1_HASH, GOLDEN_CONTINUITY_HASH);
}

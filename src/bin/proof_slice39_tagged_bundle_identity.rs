//! Slice 39 proof — an algorithm-tagged identity that does not strand the old one.
//!
//! Emits a report proving four things:
//!
//! 1. the legacy identity of a pre-existing profile is byte-identical to the
//!    value recorded before this slice — the load-bearing claim, because a pack
//!    stored under it is keyed by it, and a lookup miss falls back to
//!    re-grounding rather than erroring;
//! 2. both identities are derived from one canonical string, so a change to what
//!    a bundle *is* moves both or neither;
//! 3. the minted identity names the algorithm that produced it, so a resolver
//!    holding a mixed population never has to guess;
//! 4. it fits the column that has to hold it — `context_packs.context_pack_id`
//!    is `String(128)`, and DataForge's request model rejects anything longer
//!    before its handler is reached.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use precomputed_context_core::{
    assemble_context, AuthorityState, ContextAssemblyRequest, ContextBundleManifest,
    FreshnessPolicy, OverridePosture, SourceClass, SourceInput, TargetRefs, ID_PREFIX,
    LEGACY_ID_PREFIX,
};
use serde::Serialize;

/// Captured on `master` at the tip of Slice 38, before this slice existed.
const GOLDEN_PHASE1_HASH: &str = "f821a73703a5ec8d";
const GOLDEN_CONTINUITY_HASH: &str = "81419b53ca63648c";

/// `context_packs.context_pack_id` in DataForge-Local and DataForge Cloud.
const STORE_ID_COLUMN_WIDTH: usize = 128;

const SCENE: &str = "scene://chapter-05/scene-02";
const ADJACENT: &str = "scene-summary://chapter-05/scene-01";

#[derive(Debug, Serialize)]
struct Slice39Report {
    schema_version: String,
    manifest: ContextBundleManifest,
    repeated_context_bundle_id: String,
    deterministic_identity: bool,
    legacy_identity_preserved: bool,
    identities_share_one_canonical_string: bool,
    current_identity_is_tagged: bool,
    id_fits_store_column: bool,
    minted_id_length: usize,
    store_id_column_width: usize,
}

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

fn main() -> Result<(), Box<dyn Error>> {
    let request = two_scene_request(4);
    let first = assemble_context(&request)?;
    let second = assemble_context(&request)?;

    let deterministic_identity = first.manifest.context_bundle_id
        == second.manifest.context_bundle_id
        && first.manifest.legacy_context_bundle_id == second.manifest.legacy_context_bundle_id;
    if !deterministic_identity {
        return Err("bundle identity is not deterministic across repeated assembly".into());
    }

    // 1. The legacy identity of both pre-existing profiles is unchanged.
    let phase1 = assemble_context(&phase1_request())?;
    let continuity = assemble_context(&continuity_request())?;
    let legacy_identity_preserved = phase1.manifest.legacy_bundle_hash == GOLDEN_PHASE1_HASH
        && continuity.manifest.legacy_bundle_hash == GOLDEN_CONTINUITY_HASH
        && phase1.manifest.legacy_context_bundle_id
            == format!("{LEGACY_ID_PREFIX}{GOLDEN_PHASE1_HASH}")
        && continuity.manifest.legacy_context_bundle_id
            == format!("{LEGACY_ID_PREFIX}{GOLDEN_CONTINUITY_HASH}");
    if !legacy_identity_preserved {
        return Err(format!(
            "a pre-existing bundle identity moved, so every pack stored under it is now \
             unfindable: phase1 legacy={} (expected {GOLDEN_PHASE1_HASH}), continuity legacy={} \
             (expected {GOLDEN_CONTINUITY_HASH})",
            phase1.manifest.legacy_bundle_hash, continuity.manifest.legacy_bundle_hash
        )
        .into());
    }

    // 2. One canonical string behind both digests: a change to what the bundle
    //    is must move both, never one.
    let altered = assemble_context(&two_scene_request(5))?;
    let identities_share_one_canonical_string = altered.manifest.bundle_hash
        != first.manifest.bundle_hash
        && altered.manifest.legacy_bundle_hash != first.manifest.legacy_bundle_hash;
    if !identities_share_one_canonical_string {
        return Err(
            "a change moved one identity but not the other; the digests are taken \
                    over different inputs and can drift apart"
                .into(),
        );
    }

    // 3. The minted identity names its algorithm, and cannot be mistaken for the
    //    legacy form.
    let current_identity_is_tagged = first.manifest.context_bundle_id.starts_with(ID_PREFIX)
        && first.manifest.bundle_hash.len() == 64
        && !first
            .manifest
            .context_bundle_id
            .starts_with(LEGACY_ID_PREFIX);
    if !current_identity_is_tagged {
        return Err("the minted id does not name the algorithm that produced it".into());
    }

    // 4. It fits the column that has to hold it.
    let minted_id_length = first.manifest.context_bundle_id.len();
    let id_fits_store_column = minted_id_length <= STORE_ID_COLUMN_WIDTH;
    if !id_fits_store_column {
        return Err(format!(
            "the minted id is {minted_id_length} characters; \
             context_packs.context_pack_id holds {STORE_ID_COLUMN_WIDTH}"
        )
        .into());
    }

    let report = Slice39Report {
        schema_version: "proof.slice39-tagged-bundle-identity.v1".to_string(),
        manifest: first.manifest.clone(),
        repeated_context_bundle_id: second.manifest.context_bundle_id.clone(),
        deterministic_identity,
        legacy_identity_preserved,
        identities_share_one_canonical_string,
        current_identity_is_tagged,
        id_fits_store_column,
        minted_id_length,
        store_id_column_width: STORE_ID_COLUMN_WIDTH,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice39_tagged_bundle_identity/tagged_bundle_identity_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

    println!("{}", report_path.display());
    Ok(())
}

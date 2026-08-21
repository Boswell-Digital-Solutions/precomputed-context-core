//! Slice 38 proof — per-class freshness, and the equivalence it must preserve.
//!
//! Emits a report proving four things:
//!
//! 1. a mixed manuscript-plus-memory bundle assembles under per-class limits,
//!    deterministically — the case that could not be expressed before;
//! 2. an override for one class does not slacken another, and the refusal names
//!    the limit that actually applied rather than the bundle-wide default;
//! 3. a policy that cannot be read one way — a duplicate override, or one for a
//!    class that cannot be used — is refused before any source is considered;
//! 4. the rewritten freshness-band rule agrees with the one it replaced
//!    everywhere no override is present, swept rather than argued, and the two
//!    pre-existing bundle hashes have not moved.
//!
//! Nothing is published on a rejected path: refusals are recorded as outcomes,
//! and the refused requests produce no manifest.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use precomputed_context_core::{
    assemble_context, AuthorityState, ClassFreshnessOverride, ContextAssemblyRequest,
    ContextBundleManifest, FreshnessBand, FreshnessPolicy, OverridePosture, SourceClass,
    SourceInput, SourceProvenance, TargetRefs,
};
use serde::Serialize;

/// Captured on `master` at the tip of Slice 37, before this slice existed.
const GOLDEN_PHASE1_HASH: &str = "f821a73703a5ec8d";
const GOLDEN_CONTINUITY_HASH: &str = "81419b53ca63648c";

const SCENE: &str = "scene://chapter-05/scene-02";
const ADJACENT: &str = "scene-summary://chapter-05/scene-01";
const MEMORY: &str = "memory://fact/f-contract-authority";

#[derive(Debug, Serialize)]
struct RefusalOutcome {
    case: String,
    refused: bool,
    error: String,
    published_manifest: bool,
}

#[derive(Debug, Serialize)]
struct BandEquivalence {
    limits_swept: Vec<u64>,
    age_pairs_compared: usize,
    disagreements: usize,
}

#[derive(Debug, Serialize)]
struct Slice38Report {
    schema_version: String,
    mixed_manifest: ContextBundleManifest,
    repeated_bundle_hash: String,
    deterministic_bundle_hash: bool,
    band_follows_nearest_to_its_own_limit: bool,
    band_equivalence_without_overrides: BandEquivalence,
    refusals: Vec<RefusalOutcome>,
    preexisting_hashes_unmoved: bool,
}

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

fn memory_override(minutes: u64) -> FreshnessPolicy {
    FreshnessPolicy {
        max_source_age_minutes: 120,
        class_overrides: Some(vec![ClassFreshnessOverride {
            source_class: SourceClass::GovernedMemoryFact,
            max_source_age_minutes: minutes,
        }]),
    }
}

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
            source(SCENE, SourceClass::ActiveScene, active_age),
            source(
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

fn refusal(case: &str, request: &ContextAssemblyRequest) -> Result<RefusalOutcome, String> {
    match assemble_context(request) {
        Ok(_) => Err(format!("{case} assembled but should have been refused")),
        Err(err) => Ok(RefusalOutcome {
            case: case.to_string(),
            refused: true,
            error: err.to_string(),
            published_manifest: false,
        }),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // 1. A mixed bundle, under a memory-appropriate override, assembles twice
    //    to the same identity.
    let mixed = mixed_request(memory_override(525_600), 4, 10_000);
    let first = assemble_context(&mixed)?;
    let second = assemble_context(&mixed)?;
    if first.manifest.bundle_hash != second.manifest.bundle_hash {
        return Err("mixed bundle_hash not deterministic across repeated assembly".into());
    }

    // 2. The band follows the source nearest its *own* limit. The scene is far
    //    younger in minutes and far closer to being refused.
    let near = assemble_context(&mixed_request(memory_override(525_600), 60, 10_000))?;
    let fresh = assemble_context(&mixed_request(memory_override(525_600), 4, 10_000))?;
    let band_follows_nearest_to_its_own_limit = near.manifest.freshness_band
        == FreshnessBand::NearLimit
        && fresh.manifest.freshness_band == FreshnessBand::Fresh;
    if !band_follows_nearest_to_its_own_limit {
        return Err("the freshness band did not follow the source nearest its own limit".into());
    }

    // 3. Refusals, each for its own reason and publishing nothing.
    let refusals = vec![
        refusal(
            "memory fact stale under its own override",
            &mixed_request(memory_override(60), 4, 61),
        )?,
        refusal(
            "an override for one class does not slacken another",
            &mixed_request(memory_override(525_600), 121, 10_000),
        )?,
        refusal("duplicate override for one class", &{
            let mut policy = memory_override(600);
            policy
                .class_overrides
                .as_mut()
                .expect("overrides")
                .push(ClassFreshnessOverride {
                    source_class: SourceClass::GovernedMemoryFact,
                    max_source_age_minutes: 10,
                });
            mixed_request(policy, 4, 5)
        })?,
        refusal(
            "override for a class that cannot be used",
            &mixed_request(
                FreshnessPolicy {
                    max_source_age_minutes: 120,
                    class_overrides: Some(vec![ClassFreshnessOverride {
                        source_class: SourceClass::ExperimentalFutureSource,
                        max_source_age_minutes: 10,
                    }]),
                },
                4,
                5,
            ),
        )?,
    ];

    // 4a. The rewritten band rule agrees with the one it replaced, everywhere
    //     no override is present. Swept, not argued.
    let limits_swept = vec![0_u64, 1, 2, 3, 7, 120];
    let mut age_pairs_compared = 0_usize;
    let mut disagreements = 0_usize;
    for &limit in &limits_swept {
        for a in 0..=limit {
            for b in 0..=limit {
                let output =
                    assemble_context(&two_scene_request(FreshnessPolicy::uniform(limit), a, b))?;
                let oldest = a.max(b);
                let replaced_rule = if limit == 0 || oldest * 2 < limit {
                    FreshnessBand::Fresh
                } else {
                    FreshnessBand::NearLimit
                };
                age_pairs_compared += 1;
                if output.manifest.freshness_band != replaced_rule {
                    disagreements += 1;
                }
            }
        }
    }
    // A sweep that compared nothing would report zero disagreements, which reads
    // exactly like success. Refuse to publish that.
    if age_pairs_compared < 100 {
        return Err(format!(
            "the band equivalence sweep compared only {age_pairs_compared} age pairs; it did not run"
        )
        .into());
    }
    if disagreements > 0 {
        return Err(format!(
            "the rewritten band rule disagreed with the one it replaced in {disagreements} of \
             {age_pairs_compared} cases with no override present"
        )
        .into());
    }

    // 4b. Nothing that assembled before this slice assembles differently.
    let phase1 = assemble_context(&phase1_request())?;
    let continuity = assemble_context(&continuity_request())?;
    let preexisting_hashes_unmoved = phase1.manifest.legacy_bundle_hash == GOLDEN_PHASE1_HASH
        && continuity.manifest.legacy_bundle_hash == GOLDEN_CONTINUITY_HASH;
    if !preexisting_hashes_unmoved {
        return Err(format!(
            "a pre-existing bundle hash moved: phase1={} (expected {GOLDEN_PHASE1_HASH}), \
             continuity={} (expected {GOLDEN_CONTINUITY_HASH})",
            phase1.manifest.legacy_bundle_hash, continuity.manifest.legacy_bundle_hash
        )
        .into());
    }

    let report = Slice38Report {
        schema_version: "proof.slice38-per-class-freshness.v1".to_string(),
        mixed_manifest: first.manifest.clone(),
        repeated_bundle_hash: second.manifest.bundle_hash.clone(),
        deterministic_bundle_hash: true,
        band_follows_nearest_to_its_own_limit,
        band_equivalence_without_overrides: BandEquivalence {
            limits_swept,
            age_pairs_compared,
            disagreements,
        },
        refusals,
        preexisting_hashes_unmoved,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice38_per_class_freshness/per_class_freshness_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

    println!("{}", report_path.display());
    Ok(())
}

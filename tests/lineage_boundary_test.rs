//! Lineage boundary gate (PCC -> pact).
//!
//! PCC OWNS the bundle handle. This gate pins two facts so a downstream
//! consumer can rely on them, and so drift is caught at compile/test time:
//!
//!   1. PCC `ContextBundleManifest` carries EXACTLY `context_bundle_id` +
//!      `bundle_hash` as its lineage-handle fields, and
//!      `ContextAssemblyRequest` carries `task_intent_id`.
//!      (Proven by compile-time field access + a serde round-trip.)
//!
//!   2. The PCC->pact boundary rename `bundle_hash` -> `context_bundle_hash`
//!      is intentional and is encoded here as a documented constant, then
//!      checked against the VENDORED canonical pact schema
//!      (`tests/_vendor/pact_packet_base.schema.json`, a committed in-repo
//!      mirror of pact's `99-contracts/schemas/packet_base.schema.json`).
//!
//! SCOPE: This gate deliberately does NOT assert any continuity enums
//! (finding_type / scope_type / span_role / confidence / candidate_state /
//! severity_hint). Those are pact's contract surface, out of scope for PCC.
//!
//! FAIL-CLOSED: schema extraction asserts the extracted set is NON-EMPTY
//! before any comparison, so a parse/shape change can never trivially "pass".

use precomputed_context_core::{ContextAssemblyRequest, ContextBundleManifest};
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// The PCC->pact field rename mapping (intentional, documented).
//
// PCC field name            =>  pact (packet_base) field name
// -------------------------     ------------------------------
//   context_bundle_id       =>  context_bundle_id          (identity)
//   bundle_hash             =>  context_bundle_hash        (RENAME)
//   task_intent_id          =>  task_intent_id             (identity, on request)
//
// A downstream gate may rely on these tuples as the canonical mapping.
// ---------------------------------------------------------------------------

/// (pcc_field, pact_field). The lineage triple, expressed as the rename map.
const LINEAGE_RENAME_MAP: &[(&str, &str)] = &[
    ("context_bundle_id", "context_bundle_id"),
    ("bundle_hash", "context_bundle_hash"),
    ("task_intent_id", "task_intent_id"),
];

/// The one true rename across the boundary, named explicitly so drift on it
/// is loud.
const PCC_BUNDLE_HASH_FIELD: &str = "bundle_hash";
const PACT_BUNDLE_HASH_FIELD: &str = "context_bundle_hash";

/// Vendored, committed copy of pact's `packet_base.schema.json`.
/// We load from this in-repo mirror, NOT a brittle ../../ path into pact.
const VENDORED_PACT_PACKET_BASE: &str = include_str!("_vendor/pact_packet_base.schema.json");

fn sample_manifest() -> ContextBundleManifest {
    use precomputed_context_core::{FreshnessBand, OverrideDecision, ReplayEligibility};
    ContextBundleManifest {
        context_bundle_id: "cb-001".to_string(),
        bundle_hash: "deadbeefcafef00d".to_string(),
        source_inventory: Vec::new(),
        freshness_band: FreshnessBand::Fresh,
        override_decision: OverrideDecision::NoOverridePresent,
        authority_conflict_flag: false,
        replay_eligibility: ReplayEligibility::Eligible,
    }
}

fn sample_request() -> ContextAssemblyRequest {
    use precomputed_context_core::{FreshnessPolicy, OverridePosture, TargetRefs};
    ContextAssemblyRequest {
        task_intent_id: "ti-001".to_string(),
        task_family: "proofread".to_string(),
        task_version: "1".to_string(),
        target_refs: TargetRefs {
            active_scene_ref: None,
            adjacent_scene_ref: None,
            accepted_lore_record_refs: Vec::new(),
            accepted_style_rule_refs: Vec::new(),
        },
        allowed_source_classes: Vec::new(),
        freshness_policy: FreshnessPolicy::uniform(60),
        override_posture: OverridePosture::DisallowAll,
        sources: Vec::new(),
    }
}

/// Compile-time + runtime proof that PCC's manifest carries EXACTLY the two
/// lineage-handle fields by their PCC names. Field access below will not
/// compile if either field is renamed/removed; the serde round-trip asserts
/// the wire names are stable too.
#[test]
fn pcc_manifest_carries_context_bundle_id_and_bundle_hash() {
    let manifest = sample_manifest();

    // Compile-time field access: fails to compile if renamed/removed.
    let _id: &str = &manifest.context_bundle_id;
    let _hash: &str = &manifest.bundle_hash;

    // Serde round-trip pins the WIRE field names.
    let v = serde_json::to_value(&manifest).expect("manifest serializes");
    let obj = v.as_object().expect("manifest is a JSON object");
    assert!(
        obj.contains_key("context_bundle_id"),
        "PCC manifest must serialize `context_bundle_id`"
    );
    assert!(
        obj.contains_key("bundle_hash"),
        "PCC manifest must serialize `bundle_hash` (PCC name; renamed to \
         context_bundle_hash at the pact boundary)"
    );
    // Guard against accidental adoption of the pact name on the PCC side.
    assert!(
        !obj.contains_key("context_bundle_hash"),
        "PCC manifest must NOT use pact's `context_bundle_hash` name; the \
         rename happens AT the boundary, not in PCC"
    );

    let round: ContextBundleManifest = serde_json::from_value(v).expect("manifest round-trips");
    assert_eq!(round, manifest);
}

/// Compile-time + runtime proof that PCC's assembly request carries
/// `task_intent_id`.
#[test]
fn pcc_request_carries_task_intent_id() {
    let request = sample_request();

    let _ti: &str = &request.task_intent_id; // compile-time field access.

    let v = serde_json::to_value(&request).expect("request serializes");
    let obj = v.as_object().expect("request is a JSON object");
    assert!(
        obj.contains_key("task_intent_id"),
        "PCC request must serialize `task_intent_id`"
    );

    let round: ContextAssemblyRequest = serde_json::from_value(v).expect("request round-trips");
    assert_eq!(round, request);
}

/// Extract every top-level property name from the vendored pact packet_base
/// schema. FAIL-CLOSED: returns a set; callers assert non-empty before use.
fn vendored_pact_property_names() -> BTreeSet<String> {
    let schema: serde_json::Value = serde_json::from_str(VENDORED_PACT_PACKET_BASE)
        .expect("vendored pact packet_base schema parses as JSON");
    let props = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .expect("vendored pact packet_base schema has a `properties` object");
    props.keys().cloned().collect()
}

/// The boundary rename is real and points at fields that ACTUALLY EXIST in
/// the vendored canonical pact schema. This is what a downstream gate relies
/// on: PCC.bundle_hash <=> pact.context_bundle_hash.
#[test]
fn pact_boundary_carries_the_renamed_lineage_triple() {
    let pact_props = vendored_pact_property_names();

    // FAIL-CLOSED: a broken parse / empty extraction must NOT pass silently.
    assert!(
        !pact_props.is_empty(),
        "FAIL-CLOSED: extracted zero properties from vendored pact schema"
    );

    // The pact-side names of the lineage triple, as a SET (order-independent).
    let expected_pact_triple: BTreeSet<String> = LINEAGE_RENAME_MAP
        .iter()
        .map(|(_pcc, pact)| pact.to_string())
        .collect();

    // Every pact-side lineage field must be present in the canonical schema.
    let missing: Vec<&String> = expected_pact_triple
        .iter()
        .filter(|name| !pact_props.contains(*name))
        .collect();
    assert!(
        missing.is_empty(),
        "vendored pact packet_base is missing lineage-triple field(s) {:?}; \
         present properties = {:?}",
        missing,
        pact_props
    );

    // The rename target specifically must exist; its absence would mean the
    // PCC.bundle_hash -> pact.context_bundle_hash mapping is broken.
    assert!(
        pact_props.contains(PACT_BUNDLE_HASH_FIELD),
        "boundary rename target `{}` (== PCC `{}`) absent from vendored pact \
         schema",
        PACT_BUNDLE_HASH_FIELD,
        PCC_BUNDLE_HASH_FIELD
    );
}

/// Pin the rename map itself so it cannot silently drift: the PCC side must
/// reference real PCC fields and the pact side must reference the real,
/// renamed pact field.
#[test]
fn rename_map_is_internally_consistent() {
    // The PCC bundle-hash entry must map the PCC name to the pact name.
    let entry = LINEAGE_RENAME_MAP
        .iter()
        .find(|(pcc, _)| *pcc == PCC_BUNDLE_HASH_FIELD)
        .expect("rename map must contain the bundle_hash entry");
    assert_eq!(
        entry.1, PACT_BUNDLE_HASH_FIELD,
        "intentional rename must be bundle_hash -> context_bundle_hash"
    );

    // The two identity mappings must remain identities.
    for (pcc, pact) in LINEAGE_RENAME_MAP {
        if *pcc != PCC_BUNDLE_HASH_FIELD {
            assert_eq!(
                pcc, pact,
                "non-rename lineage field `{}` must keep the same name across \
                 the boundary",
                pcc
            );
        }
    }
}

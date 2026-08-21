mod common;
mod governed_flow_support;

use std::path::Path;

use common::{base_artifact, base_packet};
use governed_flow_support::assert_governed_flow_report;

#[cfg(feature = "test-support")]
use precomputed_context_core::fixture_support::{
    authority_record_changed_event, source_deleted_event,
};

#[cfg(not(feature = "test-support"))]
use precomputed_context_core::proof::{
    authority_record_changed_event_for_test as authority_record_changed_event, source_deleted_event,
};

use precomputed_context_core::{
    apply_artifact_invalidation, apply_packet_constituent_change, run_governed_flow_proof,
    validate_artifact_state, validate_packet_state, AdmissibilityState,
    ArtifactInvalidationDecision, FreshnessState,
};

#[test]
fn governed_flow_proof_passes_and_preserves_affected_boundaries() {
    let report = run_governed_flow_proof(Path::new("."));
    assert_governed_flow_report(&report);
}

#[test]
fn governed_flow_mutation_detects_broken_artifact_and_packet_state() {
    let report = run_governed_flow_proof(Path::new("."));
    assert_governed_flow_report(&report);

    let mut artifact = base_artifact();
    artifact.freshness_state = FreshnessState::Invalidated;
    artifact.admissibility_state = AdmissibilityState::Admissible;

    let artifact_err = validate_artifact_state(&artifact).unwrap_err();
    assert!(
        artifact_err.contains("invalidated") || artifact_err.contains("not_admissible"),
        "unexpected artifact validation error: {}",
        artifact_err
    );

    let mut packet = base_packet();
    packet.reevaluation_required = true;

    let packet_err = validate_packet_state(&packet).unwrap_err();
    assert!(
        packet_err.contains("reevaluation") || packet_err.contains("not_admissible"),
        "unexpected packet validation error: {}",
        packet_err
    );
}

#[test]
fn source_deleted_trigger_invalidates_artifact_and_downgrades_packet() {
    let artifact = base_artifact();
    let packet = base_packet();
    let event = source_deleted_event();

    let artifact_outcome = apply_artifact_invalidation(&event, &artifact);

    assert!(artifact_outcome.overlapped);
    assert_eq!(
        artifact_outcome.decision,
        ArtifactInvalidationDecision::Invalidated
    );
    assert_eq!(
        artifact_outcome.artifact.freshness_state,
        FreshnessState::Invalidated
    );
    assert_eq!(
        artifact_outcome.artifact.admissibility_state,
        AdmissibilityState::NotAdmissible
    );
    assert!(artifact_outcome.remediation_required);

    let packet_outcome =
        apply_packet_constituent_change(&packet, &[artifact_outcome.artifact.clone()]);

    assert_eq!(packet_outcome.affected_artifact_ids, vec!["art-001"]);
    assert!(packet_outcome.reevaluation_required);
    assert!(packet_outcome.remediation_required);
    assert!(packet_outcome.packet.reevaluation_required);
    assert_eq!(
        packet_outcome.packet.admissibility_state,
        AdmissibilityState::NotAdmissible
    );
}

#[test]
fn authority_record_changed_invalidates_artifact_and_downgrades_packet() {
    let artifact = base_artifact();
    let packet = base_packet();
    let event = authority_record_changed_event();

    let artifact_outcome = apply_artifact_invalidation(&event, &artifact);

    assert!(artifact_outcome.overlapped);
    assert_ne!(
        artifact_outcome.artifact.freshness_state,
        FreshnessState::Fresh
    );
    assert_ne!(
        artifact_outcome.artifact.admissibility_state,
        AdmissibilityState::Admissible
    );
    assert!(artifact_outcome.remediation_required);

    let packet_outcome =
        apply_packet_constituent_change(&packet, &[artifact_outcome.artifact.clone()]);

    assert_eq!(packet_outcome.affected_artifact_ids, vec!["art-001"]);
    assert!(packet_outcome.reevaluation_required);
    assert!(packet_outcome.remediation_required);
    assert!(packet_outcome.packet.reevaluation_required);
    assert_ne!(
        packet_outcome.packet.admissibility_state,
        AdmissibilityState::Admissible
    );
}

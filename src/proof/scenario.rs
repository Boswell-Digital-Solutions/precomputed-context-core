use std::path::Path;

use crate::{
    apply_artifact_invalidation, apply_packet_constituent_change, plan_artifact_remediation,
    plan_packet_remediation, validate_artifact_state, validate_packet_state, AdmissibilityState,
    ArtifactClass, ArtifactInvalidationDecision, EventLedger, EventProcessingDecision,
    FreshnessState, RemediationTrigger, SourceFamily,
};

use super::{
    authority_record_changed_event, base_artifact_record, base_authority_record,
    base_packet_record, source_changed_event, GovernedFlowReport, GovernedFlowStep,
};

pub fn run_governed_flow_proof(_root: &Path) -> GovernedFlowReport {
    let mut steps = Vec::new();

    let authority_record = base_authority_record();
    let authority_ok = authority_record.validate().is_ok()
        && authority_record
            .validate_derivation_request(
                ArtifactClass::RepoNavigationMap,
                &[SourceFamily::CodeRuntime, SourceFamily::RepoTruthDoc],
            )
            .is_ok();

    steps.push(if authority_ok {
        GovernedFlowStep::pass(
            "authority_record",
            "authority resolution record validated and approved derivation request passed",
        )
    } else {
        GovernedFlowStep::fail(
            "authority_record",
            "authority resolution record failed validation or derivation gating",
        )
    });

    let affected_artifact = base_artifact_record("art-001", "src-tauri/src/lib.rs");
    let unaffected_artifact = base_artifact_record("art-002", "README.md");

    let affected_packet = base_packet_record("pkt-001", "art-001");
    let unaffected_packet = base_packet_record("pkt-002", "art-002");

    let initial_state_ok = validate_artifact_state(&affected_artifact).is_ok()
        && validate_artifact_state(&unaffected_artifact).is_ok()
        && validate_packet_state(&affected_packet).is_ok()
        && validate_packet_state(&unaffected_packet).is_ok();

    steps.push(if initial_state_ok {
        GovernedFlowStep::pass(
            "initial_state_validation",
            "artifacts and packets start in a valid approved/admissible posture",
        )
    } else {
        GovernedFlowStep::fail(
            "initial_state_validation",
            "initial governed objects are not starting from a valid posture",
        )
    });

    let source_event = source_changed_event("evt-001", "idem-001", None);
    let duplicate_event = source_changed_event("evt-duplicate", "idem-001", Some("evt-001"));
    let authority_event =
        authority_record_changed_event("evt-authority-001", "idem-authority-001", Some("evt-001"));

    let mut ledger = EventLedger::default();

    let first_decision = ledger.accept(source_event.clone());
    let duplicate_decision = ledger.accept(duplicate_event);
    let authority_decision = ledger.accept(authority_event.clone());

    let dedupe_ok = matches!(first_decision, Ok(EventProcessingDecision::Accepted))
        && matches!(
            duplicate_decision,
            Ok(EventProcessingDecision::DuplicateIgnored)
        )
        && matches!(authority_decision, Ok(EventProcessingDecision::Accepted));

    steps.push(if dedupe_ok {
        GovernedFlowStep::pass(
            "event_dedupe",
            "duplicate source event ignored by idempotency key while source and authority triggers were accepted",
        )
    } else {
        GovernedFlowStep::fail(
            "event_dedupe",
            "event ledger did not enforce idempotency across source and authority triggers as expected",
        )
    });

    let batches = ledger.coalesce_pending();
    let coalescing_ok = batches.len() == 2
        && batches
            .iter()
            .map(|batch| batch.events.len())
            .sum::<usize>()
            == 2;

    steps.push(if coalescing_ok {
        GovernedFlowStep::pass(
            "event_coalescing",
            "source and authority triggers coalesced into two dependency-distinct batches",
        )
    } else {
        GovernedFlowStep::fail(
            "event_coalescing",
            format!(
                "expected 2 coalesced batches containing 2 accepted events, got {} batch(es)",
                batches.len()
            ),
        )
    });

    let triggering_event_ids: Vec<String> = batches
        .iter()
        .flat_map(|batch| batch.events.iter().map(|event| event.event_id.clone()))
        .collect();

    let source_outcome = apply_artifact_invalidation(&source_event, &affected_artifact);
    let source_invalidation_ok = source_outcome.overlapped
        && source_outcome.decision == ArtifactInvalidationDecision::ReviewDue
        && source_outcome.artifact.freshness_state == FreshnessState::ReviewDue
        && validate_artifact_state(&source_outcome.artifact).is_ok()
        && unaffected_artifact.freshness_state == FreshnessState::Fresh
        && unaffected_artifact.admissibility_state == AdmissibilityState::Admissible;

    steps.push(if source_invalidation_ok {
        GovernedFlowStep::pass(
            "source_invalidation",
            "source-driven degradation downgraded the affected artifact to review_due without touching unaffected boundaries",
        )
    } else {
        GovernedFlowStep::fail(
            "source_invalidation",
            "source-driven degradation did not produce the expected review_due posture",
        )
    });

    let authority_outcome = apply_artifact_invalidation(&authority_event, &source_outcome.artifact);
    let authority_invalidation_ok = authority_outcome.overlapped
        && authority_outcome.decision == ArtifactInvalidationDecision::Invalidated
        && authority_outcome.artifact.freshness_state == FreshnessState::Invalidated
        && authority_outcome.artifact.admissibility_state == AdmissibilityState::NotAdmissible
        && authority_outcome.remediation_required
        && validate_artifact_state(&authority_outcome.artifact).is_ok()
        && unaffected_artifact.freshness_state == FreshnessState::Fresh
        && unaffected_artifact.admissibility_state == AdmissibilityState::Admissible;

    steps.push(if authority_invalidation_ok {
        GovernedFlowStep::pass(
            "authority_invalidation",
            "authority-driven degradation invalidated the affected artifact while unaffected boundaries remained unchanged",
        )
    } else {
        GovernedFlowStep::fail(
            "authority_invalidation",
            "authority-driven degradation did not invalidate the affected artifact as expected",
        )
    });

    let affected_packet_outcome =
        apply_packet_constituent_change(&affected_packet, &[authority_outcome.artifact.clone()]);
    let unaffected_packet_outcome =
        apply_packet_constituent_change(&unaffected_packet, &[unaffected_artifact.clone()]);

    let packet_gate_ok = affected_packet_outcome.affected_artifact_ids == vec!["art-001"]
        && affected_packet_outcome.reevaluation_required
        && affected_packet_outcome.remediation_required
        && affected_packet_outcome.packet.reevaluation_required
        && affected_packet_outcome.packet.admissibility_state == AdmissibilityState::NotAdmissible
        && !unaffected_packet_outcome.reevaluation_required
        && !unaffected_packet_outcome.remediation_required
        && unaffected_packet_outcome.packet.admissibility_state == AdmissibilityState::Admissible;

    steps.push(if packet_gate_ok {
        GovernedFlowStep::pass(
            "packet_reevaluation",
            "affected packet downgraded to not_admissible while unaffected packet remained admissible",
        )
    } else {
        GovernedFlowStep::fail(
            "packet_reevaluation",
            "packet reevaluation did not reflect constituent degradation correctly",
        )
    });

    let artifact_plan = plan_artifact_remediation(&authority_outcome.artifact);
    let packet_plan = plan_packet_remediation(
        &affected_packet_outcome.packet,
        &[authority_outcome.artifact.freshness_state],
        &[authority_outcome.artifact.lifecycle_state],
    );

    let remediation_ok = matches!(
        artifact_plan.as_ref().map(|plan| plan.trigger.clone()),
        Some(RemediationTrigger::ArtifactInvalidated)
    ) && matches!(
        packet_plan.as_ref().map(|plan| plan.trigger.clone()),
        Some(
            RemediationTrigger::PacketReevaluationRequired
                | RemediationTrigger::ConstituentInvalidated
        )
    );

    let remediation_count = artifact_plan.iter().count() + packet_plan.iter().count();

    steps.push(if remediation_ok {
        GovernedFlowStep::pass(
            "remediation_planning",
            format!(
                "artifact and packet remediation paths were generated after source and authority degradation; remediation_count={}",
                remediation_count
            ),
        )
    } else {
        GovernedFlowStep::fail(
            "remediation_planning",
            "expected both artifact and packet remediation plans to be generated",
        )
    });

    GovernedFlowReport {
        scenario_id: "governed_flow_v1",
        steps,
        triggering_event_ids,
        affected_artifact_ids: vec!["art-001".into()],
        unaffected_artifact_ids: vec!["art-002".into()],
        affected_packet_ids: vec!["pkt-001".into()],
        unaffected_packet_ids: vec!["pkt-002".into()],
        initial_artifact_freshness: "fresh".into(),
        final_artifact_freshness: "invalidated".into(),
        initial_packet_admissibility: "admissible".into(),
        final_packet_admissibility: "not_admissible".into(),
        deduped_events: 1,
        coalesced_batches: batches.len(),
        remediation_required: remediation_ok,
        remediation_count,
        schema_paths: Vec::new(),
    }
}

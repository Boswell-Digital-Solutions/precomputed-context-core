use precomputed_context_core::GovernedFlowReport;

pub fn assert_governed_flow_report(report: &GovernedFlowReport) {
    let failed_steps: Vec<String> = report
        .steps
        .iter()
        .filter(|step| !step.passed)
        .map(|step| format!("{} :: {}", step.step, step.detail))
        .collect();

    assert!(
        failed_steps.is_empty(),
        "governed flow proof failures: {:?}",
        failed_steps
    );

    assert_eq!(report.scenario_id, "governed_flow_v1");
    assert_eq!(report.deduped_events, 1);
    assert_eq!(report.coalesced_batches, 2);

    assert_eq!(report.affected_artifact_ids, vec!["art-001"]);
    assert_eq!(report.unaffected_artifact_ids, vec!["art-002"]);
    assert_eq!(report.affected_packet_ids, vec!["pkt-001"]);
    assert_eq!(report.unaffected_packet_ids, vec!["pkt-002"]);

    assert_eq!(report.initial_artifact_freshness, "fresh");
    assert_eq!(report.final_artifact_freshness, "invalidated");
    assert_eq!(report.initial_packet_admissibility, "admissible");
    assert_eq!(report.final_packet_admissibility, "not_admissible");

    assert!(
        report
            .steps
            .iter()
            .any(|step| step.step == "source_invalidation" && step.passed),
        "missing passing source_invalidation step"
    );
    assert!(
        report
            .steps
            .iter()
            .any(|step| step.step == "authority_invalidation" && step.passed),
        "missing passing authority_invalidation step"
    );
    assert!(
        report
            .steps
            .iter()
            .any(|step| step.step == "packet_reevaluation" && step.passed),
        "missing passing packet_reevaluation step"
    );
    assert!(
        report
            .steps
            .iter()
            .any(|step| step.step == "remediation_planning" && step.passed),
        "missing passing remediation_planning step"
    );

    assert!(report.remediation_required);
    assert_eq!(report.remediation_count, 2);
    assert_eq!(report.triggering_event_ids.len(), 2);
    assert!(report.passed());
}

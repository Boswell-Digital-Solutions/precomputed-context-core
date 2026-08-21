use crate::{
    AdmissibilityState, ArtifactClass, ArtifactRecord, AuthorityLevel, AuthorityResolutionRecord,
    CriticStatus, EventRecord, EventType, FreshnessState, LifecycleState, PacketLifecycleState,
    PacketRecord, PacketRole, RepoArchetype, SensitivityClassification, SourceFamily,
};

pub fn base_authority_record() -> AuthorityResolutionRecord {
    AuthorityResolutionRecord {
        schema_version: "1.0".into(),
        repo_id: "forge-command".into(),
        repo_name: "ForgeCommand".into(),
        repo_archetype: RepoArchetype::DesktopAppRepo,
        authority_order: vec![
            SourceFamily::ContractSchema,
            SourceFamily::CodeRuntime,
            SourceFamily::TestVerification,
            SourceFamily::RepoTruthDoc,
            SourceFamily::ProtocolDoc,
        ],
        approved_source_families: vec![
            SourceFamily::CodeRuntime,
            SourceFamily::ContractSchema,
            SourceFamily::TestVerification,
            SourceFamily::RepoTruthDoc,
            SourceFamily::ProtocolDoc,
        ],
        disallowed_source_families: vec![SourceFamily::GeneratedOutput, SourceFamily::AdvisoryNote],
        ambiguity_rules: vec![
            "code_vs_repo_truth_doc_conflict_blocks".into(),
            "protocol_vs_runtime_conflict_requires_review".into(),
        ],
        escalation_required_conditions: vec!["authority_conflict_on_required_truth".into()],
        approved_derivation_scope: vec![
            ArtifactClass::RepoNavigationMap,
            ArtifactClass::KeyFilePacket,
            ArtifactClass::ValidationCommandPacket,
        ],
        operator_review_required_conditions: vec!["authority_ambiguity".into()],
        notes_on_known_authority_gaps: Some("Initial governed-flow fixture record.".into()),
        created_at: "2026-04-09T00:00:00-04:00".into(),
        last_reviewed_at: "2026-04-09T00:00:00-04:00".into(),
    }
}

pub fn base_artifact_record(artifact_id: &str, source_ref: &str) -> ArtifactRecord {
    ArtifactRecord {
        schema_version: "1.0".into(),
        artifact_id: artifact_id.into(),
        artifact_class: ArtifactClass::RepoNavigationMap,
        repo_id: "forge-command".into(),
        title: format!("Artifact {}", artifact_id),
        operational_purpose: "Provide first-wave repo structure".into(),
        summary_block: "Bounded navigational summary".into(),
        source_refs: vec![source_ref.into(), "doc/fcSYSTEM.md".into()],
        source_ref_hashes: vec!["hash-1".into(), "hash-2".into()],
        authority_level: AuthorityLevel::Canonical,
        lifecycle_state: LifecycleState::Approved,
        freshness_state: FreshnessState::Fresh,
        critic_status: CriticStatus::Passed,
        admissibility_state: AdmissibilityState::Admissible,
        related_artifact_refs: vec![],
        supersedes_artifact_id: None,
        protocol_refs: vec!["BDS_BACKEND_ENGINEERING_PROTOCOL".into()],
        created_at: "2026-04-09T00:00:00-04:00".into(),
        last_validated_at: "2026-04-09T00:00:00-04:00".into(),
        producer_identity: "governed-flow-proof".into(),
        sensitivity_classification: SensitivityClassification::InternalGeneral,
    }
}

pub fn base_packet_record(packet_id: &str, artifact_id: &str) -> PacketRecord {
    PacketRecord {
        schema_version: "1.0".into(),
        packet_id: packet_id.into(),
        packet_role: PacketRole::RepoNavigationAssist,
        repo_id: "forge-command".into(),
        included_artifact_ids: vec![artifact_id.into()],
        included_artifact_hashes: vec![format!("hash-{}", artifact_id)],
        packet_constraints: vec!["bounded".into()],
        packet_budget_band: "small".into(),
        lane_compatibility: vec!["neuroforge".into(), "neuronforge".into()],
        lifecycle_state: PacketLifecycleState::Approved,
        admissibility_state: AdmissibilityState::Admissible,
        created_at: "2026-04-09T00:00:00-04:00".into(),
        last_evaluated_at: "2026-04-09T00:00:00-04:00".into(),
        required_constituents_present: true,
        reevaluation_required: false,
        sensitivity_classification: SensitivityClassification::InternalGeneral,
    }
}

pub fn source_changed_event(
    event_id: &str,
    idempotency_key: &str,
    causation_id: Option<&str>,
) -> EventRecord {
    EventRecord {
        event_id: event_id.into(),
        event_type: EventType::SourceChanged,
        schema_version: "1.0".into(),
        emitted_at: "2026-04-09T00:00:00-04:00".into(),
        emitter_service: "repo-watcher".into(),
        repo_id: "forge-command".into(),
        related_artifact_ids: vec!["art-001".into()],
        related_packet_ids: vec!["pkt-001".into()],
        source_refs: vec!["src-tauri/src/lib.rs".into()],
        causation_id: causation_id.map(str::to_string),
        correlation_id: "corr-governed-flow-001".into(),
        idempotency_key: idempotency_key.into(),
        event_payload: "source change affecting governed proof path".into(),
    }
}

pub fn authority_record_changed_event(
    event_id: &str,
    idempotency_key: &str,
    causation_id: Option<&str>,
) -> EventRecord {
    EventRecord {
        event_id: event_id.into(),
        event_type: EventType::AuthorityRecordChanged,
        schema_version: "1.0".into(),
        emitted_at: "2026-04-09T00:00:10-04:00".into(),
        emitter_service: "authority-governance".into(),
        repo_id: "forge-command".into(),
        related_artifact_ids: vec!["art-001".into()],
        related_packet_ids: vec!["pkt-001".into()],
        source_refs: vec!["doc/fcSYSTEM.md".into()],
        causation_id: causation_id.map(str::to_string),
        correlation_id: "corr-governed-flow-001".into(),
        idempotency_key: idempotency_key.into(),
        event_payload: "authority record changed for governed proof path".into(),
    }
}

pub fn source_deleted_event() -> EventRecord {
    EventRecord {
        event_id: "evt-source-deleted-001".into(),
        event_type: EventType::SourceDeleted,
        schema_version: "1.0".into(),
        emitted_at: "2026-04-09T00:00:05-04:00".into(),
        emitter_service: "repo-watcher".into(),
        repo_id: "forge-command".into(),
        related_artifact_ids: vec!["art-001".into()],
        related_packet_ids: vec!["pkt-001".into()],
        source_refs: vec!["src-tauri/src".into()],
        causation_id: None,
        correlation_id: "corr-source-deleted-001".into(),
        idempotency_key: "idem-source-deleted-001".into(),
        event_payload: "source deleted for governed flow coverage".into(),
    }
}

pub fn authority_record_changed_event_for_test() -> EventRecord {
    EventRecord {
        event_id: "evt-authority-record-changed-001".into(),
        event_type: EventType::AuthorityRecordChanged,
        schema_version: "1.0".into(),
        emitted_at: "2026-04-10T01:00:05-04:00".into(),
        emitter_service: "authority-governance".into(),
        repo_id: "forge-command".into(),
        related_artifact_ids: vec!["art-001".into()],
        related_packet_ids: vec!["pkt-001".into()],
        source_refs: vec!["doc/system/fcSYSTEM.md".into()],
        causation_id: None,
        correlation_id: "corr-authority-record-changed-001".into(),
        idempotency_key: "idem-authority-record-changed-001".into(),
        event_payload: "authority precedence changed for governed flow coverage".into(),
    }
}

use crate::{
    AdmissibilityState, AffectedObjectType, ArtifactRecord, CriticStatus, FreshnessState,
    LifecycleState, PacketLifecycleState, PacketRecord,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemediationTrigger {
    ArtifactBlocked,
    ArtifactStale,
    ArtifactInvalidated,
    CriticFailed,
    PacketMissingRequiredConstituents,
    PacketReevaluationRequired,
    ConstituentBlocked,
    ConstituentStale,
    ConstituentInvalidated,
    ConstituentSuperseded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemediationPlan {
    pub repo_id: String,
    pub affected_object_type: AffectedObjectType,
    pub affected_object_ids: Vec<String>,
    pub trigger: RemediationTrigger,
    pub blocking: bool,
    pub summary: String,
    pub recommended_action: String,
}

pub fn plan_artifact_remediation(artifact: &ArtifactRecord) -> Option<RemediationPlan> {
    if artifact.critic_status == CriticStatus::Failed {
        return Some(RemediationPlan {
            repo_id: artifact.repo_id.clone(),
            affected_object_type: AffectedObjectType::Artifact,
            affected_object_ids: vec![artifact.artifact_id.clone()],
            trigger: RemediationTrigger::CriticFailed,
            blocking: true,
            summary: format!(
                "artifact {} failed critic validation",
                artifact.artifact_id
            ),
            recommended_action:
                "repair the artifact, rerun critic validation, and reapprove only after the failure is cleared"
                    .into(),
        });
    }

    match artifact.lifecycle_state {
        LifecycleState::Blocked => Some(RemediationPlan {
            repo_id: artifact.repo_id.clone(),
            affected_object_type: AffectedObjectType::Artifact,
            affected_object_ids: vec![artifact.artifact_id.clone()],
            trigger: RemediationTrigger::ArtifactBlocked,
            blocking: true,
            summary: format!("artifact {} is blocked", artifact.artifact_id),
            recommended_action:
                "resolve the blocking condition, regenerate if needed, and revalidate before reuse"
                    .into(),
        }),
        _ => match artifact.freshness_state {
            FreshnessState::Stale => Some(RemediationPlan {
                repo_id: artifact.repo_id.clone(),
                affected_object_type: AffectedObjectType::Artifact,
                affected_object_ids: vec![artifact.artifact_id.clone()],
                trigger: RemediationTrigger::ArtifactStale,
                blocking: true,
                summary: format!("artifact {} is stale", artifact.artifact_id),
                recommended_action:
                    "revalidate against current source refs and refresh source hashes before normal admission"
                        .into(),
            }),
            FreshnessState::Invalidated => Some(RemediationPlan {
                repo_id: artifact.repo_id.clone(),
                affected_object_type: AffectedObjectType::Artifact,
                affected_object_ids: vec![artifact.artifact_id.clone()],
                trigger: RemediationTrigger::ArtifactInvalidated,
                blocking: true,
                summary: format!("artifact {} is invalidated", artifact.artifact_id),
                recommended_action:
                    "repair or regenerate the artifact from canonical sources, then rerun validation and approval"
                        .into(),
            }),
            _ => None,
        },
    }
}

pub fn plan_packet_remediation(
    packet: &PacketRecord,
    constituent_freshness: &[FreshnessState],
    constituent_lifecycle: &[LifecycleState],
) -> Option<RemediationPlan> {
    if packet.lifecycle_state != PacketLifecycleState::Approved {
        return None;
    }

    if !packet.required_constituents_present {
        return Some(RemediationPlan {
            repo_id: packet.repo_id.clone(),
            affected_object_type: AffectedObjectType::Packet,
            affected_object_ids: vec![packet.packet_id.clone()],
            trigger: RemediationTrigger::PacketMissingRequiredConstituents,
            blocking: true,
            summary: format!(
                "packet {} is missing required constituents",
                packet.packet_id
            ),
            recommended_action:
                "restore the missing required artifact set and recompute the packet before admission"
                    .into(),
        });
    }

    if packet.reevaluation_required {
        return Some(RemediationPlan {
            repo_id: packet.repo_id.clone(),
            affected_object_type: AffectedObjectType::Packet,
            affected_object_ids: vec![packet.packet_id.clone()],
            trigger: RemediationTrigger::PacketReevaluationRequired,
            blocking: true,
            summary: format!("packet {} requires reevaluation", packet.packet_id),
            recommended_action:
                "reevaluate all required constituents and recompute packet admissibility before use"
                    .into(),
        });
    }

    if constituent_lifecycle.contains(&LifecycleState::Blocked) {
        return Some(RemediationPlan {
            repo_id: packet.repo_id.clone(),
            affected_object_type: AffectedObjectType::Packet,
            affected_object_ids: vec![packet.packet_id.clone()],
            trigger: RemediationTrigger::ConstituentBlocked,
            blocking: true,
            summary: format!(
                "packet {} depends on at least one blocked constituent artifact",
                packet.packet_id
            ),
            recommended_action:
                "repair or replace the blocked constituent artifact, then recompute the packet"
                    .into(),
        });
    }

    if constituent_lifecycle.contains(&LifecycleState::Superseded) {
        return Some(RemediationPlan {
            repo_id: packet.repo_id.clone(),
            affected_object_type: AffectedObjectType::Packet,
            affected_object_ids: vec![packet.packet_id.clone()],
            trigger: RemediationTrigger::ConstituentSuperseded,
            blocking: true,
            summary: format!(
                "packet {} depends on at least one superseded constituent artifact",
                packet.packet_id
            ),
            recommended_action:
                "replace superseded constituent artifacts with current approved versions and recompute the packet"
                    .into(),
        });
    }

    if constituent_freshness.contains(&FreshnessState::Invalidated) {
        return Some(RemediationPlan {
            repo_id: packet.repo_id.clone(),
            affected_object_type: AffectedObjectType::Packet,
            affected_object_ids: vec![packet.packet_id.clone()],
            trigger: RemediationTrigger::ConstituentInvalidated,
            blocking: true,
            summary: format!(
                "packet {} depends on at least one invalidated constituent artifact",
                packet.packet_id
            ),
            recommended_action:
                "repair or regenerate the invalidated constituent artifact, then reevaluate packet admission"
                    .into(),
        });
    }

    if constituent_freshness.contains(&FreshnessState::Stale) {
        return Some(RemediationPlan {
            repo_id: packet.repo_id.clone(),
            affected_object_type: AffectedObjectType::Packet,
            affected_object_ids: vec![packet.packet_id.clone()],
            trigger: RemediationTrigger::ConstituentStale,
            blocking: true,
            summary: format!(
                "packet {} depends on at least one stale constituent artifact",
                packet.packet_id
            ),
            recommended_action:
                "refresh the stale constituent artifact and recompute packet admissibility".into(),
        });
    }

    if packet.admissibility_state != AdmissibilityState::Admissible {
        return Some(RemediationPlan {
            repo_id: packet.repo_id.clone(),
            affected_object_type: AffectedObjectType::Packet,
            affected_object_ids: vec![packet.packet_id.clone()],
            trigger: RemediationTrigger::PacketReevaluationRequired,
            blocking: true,
            summary: format!("packet {} is not currently admissible", packet.packet_id),
            recommended_action:
                "inspect constituent posture and rerun packet evaluation before consumer use".into(),
        });
    }

    None
}

pub fn remediation_required_for_packet(
    packet: &PacketRecord,
    constituent_freshness: &[FreshnessState],
    constituent_lifecycle: &[LifecycleState],
) -> bool {
    plan_packet_remediation(packet, constituent_freshness, constituent_lifecycle).is_some()
}

#[cfg(test)]
mod tests {
    use crate::{
        AdmissibilityState, ArtifactClass, ArtifactRecord, AuthorityLevel, CriticStatus,
        FreshnessState, LifecycleState, PacketLifecycleState, PacketRecord, PacketRole,
        SensitivityClassification,
    };

    use super::{
        plan_artifact_remediation, plan_packet_remediation, remediation_required_for_packet,
        RemediationTrigger,
    };

    fn base_artifact() -> ArtifactRecord {
        ArtifactRecord {
            schema_version: "1.0".into(),
            artifact_id: "art-001".into(),
            artifact_class: ArtifactClass::RepoNavigationMap,
            repo_id: "forge-command".into(),
            title: "ForgeCommand Repo Navigation".into(),
            operational_purpose: "Provide first-wave repo structure".into(),
            summary_block: "Bounded navigational summary".into(),
            source_refs: vec!["src-tauri/src".into(), "doc/fcSYSTEM.md".into()],
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
            producer_identity: "proof-slice-core".into(),
            sensitivity_classification: SensitivityClassification::InternalGeneral,
        }
    }

    fn base_packet() -> PacketRecord {
        PacketRecord {
            schema_version: "1.0".into(),
            packet_id: "pkt-001".into(),
            packet_role: PacketRole::RepoNavigationAssist,
            repo_id: "forge-command".into(),
            included_artifact_ids: vec!["art-001".into()],
            included_artifact_hashes: vec!["hash-art-001".into()],
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

    #[test]
    fn invalidated_artifact_generates_plan() {
        let mut artifact = base_artifact();
        artifact.freshness_state = FreshnessState::Invalidated;
        artifact.admissibility_state = AdmissibilityState::NotAdmissible;

        let plan = plan_artifact_remediation(&artifact).unwrap();
        assert_eq!(plan.trigger, RemediationTrigger::ArtifactInvalidated);
        assert!(plan.blocking);
    }

    #[test]
    fn healthy_artifact_does_not_generate_plan() {
        let artifact = base_artifact();
        assert!(plan_artifact_remediation(&artifact).is_none());
    }

    #[test]
    fn reevaluation_required_packet_generates_plan() {
        let mut packet = base_packet();
        packet.reevaluation_required = true;
        packet.admissibility_state = AdmissibilityState::NotAdmissible;

        let plan = plan_packet_remediation(
            &packet,
            &[FreshnessState::Fresh],
            &[LifecycleState::Approved],
        )
        .unwrap();

        assert_eq!(plan.trigger, RemediationTrigger::PacketReevaluationRequired);
        assert!(plan.blocking);
    }

    #[test]
    fn invalidated_constituent_generates_packet_plan() {
        let packet = base_packet();

        let plan = plan_packet_remediation(
            &packet,
            &[FreshnessState::Invalidated],
            &[LifecycleState::Approved],
        )
        .unwrap();

        assert_eq!(plan.trigger, RemediationTrigger::ConstituentInvalidated);
        assert!(plan.blocking);
    }

    #[test]
    fn healthy_packet_does_not_require_remediation() {
        let packet = base_packet();

        assert!(!remediation_required_for_packet(
            &packet,
            &[FreshnessState::Fresh],
            &[LifecycleState::Approved],
        ));
    }
}

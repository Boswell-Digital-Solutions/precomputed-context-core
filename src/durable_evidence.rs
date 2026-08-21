use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::enums::{AdmissibilityState, EventType, FreshnessState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceAdmissionResult {
    Accepted,
    Duplicate,
    Poisoned,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceBatchOutcome {
    Accepted,
    PartiallyAccepted,
    Poisoned,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTargetKind {
    Artifact,
    Packet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EventReceiptRecord {
    pub receipt_id: String,
    pub event_id: String,
    pub idempotency_key: String,
    pub event_kind: EventType,
    pub repo_id: String,
    pub correlated_scope: String,
    pub received_at: String,
    pub admission_result: EvidenceAdmissionResult,
    pub admission_reason: String,
    pub coalesced_batch_id: Option<String>,
    pub source_digest: String,
}

impl EventReceiptRecord {
    pub fn validate(&self) -> Result<(), String> {
        validate_nonempty("receipt_id", &self.receipt_id)?;
        validate_nonempty("event_id", &self.event_id)?;
        validate_nonempty("idempotency_key", &self.idempotency_key)?;
        validate_nonempty("repo_id", &self.repo_id)?;
        validate_nonempty("correlated_scope", &self.correlated_scope)?;
        validate_nonempty("received_at", &self.received_at)?;
        validate_nonempty("admission_reason", &self.admission_reason)?;
        validate_nonempty("source_digest", &self.source_digest)?;

        if let Some(batch_id) = &self.coalesced_batch_id {
            validate_nonempty("coalesced_batch_id", batch_id)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CoalescedBatchRecord {
    pub batch_id: String,
    pub repo_id: String,
    pub correlated_scope: String,
    pub member_event_ids: Vec<String>,
    pub opened_at: String,
    pub closed_at: String,
    pub batch_outcome: EvidenceBatchOutcome,
    pub poisoned: bool,
}

impl CoalescedBatchRecord {
    pub fn validate(&self) -> Result<(), String> {
        validate_nonempty("batch_id", &self.batch_id)?;
        validate_nonempty("repo_id", &self.repo_id)?;
        validate_nonempty("correlated_scope", &self.correlated_scope)?;
        validate_nonempty("opened_at", &self.opened_at)?;
        validate_nonempty("closed_at", &self.closed_at)?;
        validate_distinct_nonempty_vec("member_event_ids", &self.member_event_ids)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactInvalidationEvidenceRecord {
    pub invalidation_record_id: String,
    pub artifact_id: String,
    pub prior_freshness: FreshnessState,
    pub next_freshness: FreshnessState,
    pub prior_admissibility: AdmissibilityState,
    pub next_admissibility: AdmissibilityState,
    pub cause_event_ids: Vec<String>,
    pub cause_summary: String,
    pub changed_at: String,
}

impl ArtifactInvalidationEvidenceRecord {
    pub fn validate(&self) -> Result<(), String> {
        validate_nonempty("invalidation_record_id", &self.invalidation_record_id)?;
        validate_nonempty("artifact_id", &self.artifact_id)?;
        validate_distinct_nonempty_vec("cause_event_ids", &self.cause_event_ids)?;
        validate_nonempty("cause_summary", &self.cause_summary)?;
        validate_nonempty("changed_at", &self.changed_at)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PacketReevaluationEvidenceRecord {
    pub packet_reevaluation_id: String,
    pub packet_id: String,
    pub prior_admissibility: AdmissibilityState,
    pub next_admissibility: AdmissibilityState,
    pub constituent_artifact_ids: Vec<String>,
    pub trigger_invalidation_record_ids: Vec<String>,
    pub changed_at: String,
    pub change_summary: String,
}

impl PacketReevaluationEvidenceRecord {
    pub fn validate(&self) -> Result<(), String> {
        validate_nonempty("packet_reevaluation_id", &self.packet_reevaluation_id)?;
        validate_nonempty("packet_id", &self.packet_id)?;
        validate_distinct_nonempty_vec("constituent_artifact_ids", &self.constituent_artifact_ids)?;
        validate_distinct_nonempty_vec(
            "trigger_invalidation_record_ids",
            &self.trigger_invalidation_record_ids,
        )?;
        validate_nonempty("changed_at", &self.changed_at)?;
        validate_nonempty("change_summary", &self.change_summary)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemediationEvidenceRecord {
    pub remediation_record_id: String,
    pub target_kind: EvidenceTargetKind,
    pub target_id: String,
    pub blocking: bool,
    pub trigger_summary: String,
    pub recommended_actions: Vec<String>,
    pub generated_at: String,
}

impl RemediationEvidenceRecord {
    pub fn validate(&self) -> Result<(), String> {
        validate_nonempty("remediation_record_id", &self.remediation_record_id)?;
        validate_nonempty("target_id", &self.target_id)?;
        validate_nonempty("trigger_summary", &self.trigger_summary)?;
        validate_nonempty("generated_at", &self.generated_at)?;
        validate_optional_nonempty_vec("recommended_actions", &self.recommended_actions)?;

        if self.blocking && self.recommended_actions.is_empty() {
            return Err("recommended_actions must not be empty when blocking is true".into());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayBundleManifest {
    pub replay_bundle_id: String,
    pub created_at: String,
    pub repo_id: String,
    pub event_receipt_ids: Vec<String>,
    pub artifact_invalidation_record_ids: Vec<String>,
    pub packet_reevaluation_record_ids: Vec<String>,
    pub remediation_record_ids: Vec<String>,
    pub final_summary: String,
    pub proof_digest: String,
}

impl ReplayBundleManifest {
    pub fn validate(&self) -> Result<(), String> {
        validate_nonempty("replay_bundle_id", &self.replay_bundle_id)?;
        validate_nonempty("created_at", &self.created_at)?;
        validate_nonempty("repo_id", &self.repo_id)?;
        validate_distinct_nonempty_vec("event_receipt_ids", &self.event_receipt_ids)?;
        validate_optional_nonempty_vec(
            "artifact_invalidation_record_ids",
            &self.artifact_invalidation_record_ids,
        )?;
        validate_optional_nonempty_vec(
            "packet_reevaluation_record_ids",
            &self.packet_reevaluation_record_ids,
        )?;
        validate_optional_nonempty_vec("remediation_record_ids", &self.remediation_record_ids)?;
        validate_nonempty("final_summary", &self.final_summary)?;
        validate_nonempty("proof_digest", &self.proof_digest)?;
        Ok(())
    }
}

fn validate_nonempty(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }

    Ok(())
}

fn validate_distinct_nonempty_vec(label: &str, values: &[String]) -> Result<(), String> {
    if values.is_empty() {
        return Err(format!("{label} must not be empty"));
    }

    let mut seen = BTreeSet::new();

    for value in values {
        if value.trim().is_empty() {
            return Err(format!("{label} contains an empty entry"));
        }

        if !seen.insert(value.as_str()) {
            return Err(format!("{label} contains duplicate entry '{}'", value));
        }
    }

    Ok(())
}

fn validate_optional_nonempty_vec(label: &str, values: &[String]) -> Result<(), String> {
    let mut seen = BTreeSet::new();

    for value in values {
        if value.trim().is_empty() {
            return Err(format!("{label} contains an empty entry"));
        }

        if !seen.insert(value.as_str()) {
            return Err(format!("{label} contains duplicate entry '{}'", value));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::{AdmissibilityState, EventType, FreshnessState};

    #[test]
    fn event_receipt_record_validates() {
        let record = EventReceiptRecord {
            receipt_id: "receipt_001".into(),
            event_id: "event_001".into(),
            idempotency_key: "idem_001".into(),
            event_kind: EventType::SourceChanged,
            repo_id: "forgecommand".into(),
            correlated_scope: "src/routes".into(),
            received_at: "2026-04-15T20:05:00Z".into(),
            admission_result: EvidenceAdmissionResult::Accepted,
            admission_reason: "event accepted".into(),
            coalesced_batch_id: Some("batch_001".into()),
            source_digest: "sha256:abc123".into(),
        };

        assert!(record.validate().is_ok());
    }

    #[test]
    fn packet_reevaluation_requires_trigger_records() {
        let record = PacketReevaluationEvidenceRecord {
            packet_reevaluation_id: "pkt_reval_001".into(),
            packet_id: "packet_001".into(),
            prior_admissibility: AdmissibilityState::Admissible,
            next_admissibility: AdmissibilityState::Restricted,
            constituent_artifact_ids: vec!["artifact_001".into()],
            trigger_invalidation_record_ids: vec![],
            changed_at: "2026-04-15T20:05:00Z".into(),
            change_summary: "constituent degraded".into(),
        };

        let err = record.validate().expect_err("expected validation failure");
        assert!(err.contains("trigger_invalidation_record_ids must not be empty"));
    }

    #[test]
    fn blocking_remediation_requires_actions() {
        let record = RemediationEvidenceRecord {
            remediation_record_id: "rem_001".into(),
            target_kind: EvidenceTargetKind::Packet,
            target_id: "packet_001".into(),
            blocking: true,
            trigger_summary: "packet blocked by invalidated constituent".into(),
            recommended_actions: vec![],
            generated_at: "2026-04-15T20:05:00Z".into(),
        };

        let err = record.validate().expect_err("expected validation failure");
        assert!(err.contains("recommended_actions must not be empty when blocking is true"));
    }

    #[test]
    fn replay_bundle_requires_event_receipts() {
        let manifest = ReplayBundleManifest {
            replay_bundle_id: "bundle_001".into(),
            created_at: "2026-04-15T20:05:00Z".into(),
            repo_id: "forgecommand".into(),
            event_receipt_ids: vec![],
            artifact_invalidation_record_ids: vec![],
            packet_reevaluation_record_ids: vec![],
            remediation_record_ids: vec![],
            final_summary: "final posture recorded".into(),
            proof_digest: "sha256:def456".into(),
        };

        let err = manifest
            .validate()
            .expect_err("expected validation failure");
        assert!(err.contains("event_receipt_ids must not be empty"));
    }

    #[test]
    fn artifact_invalidation_record_validates() {
        let record = ArtifactInvalidationEvidenceRecord {
            invalidation_record_id: "inv_001".into(),
            artifact_id: "artifact_001".into(),
            prior_freshness: FreshnessState::Fresh,
            next_freshness: FreshnessState::Invalidated,
            prior_admissibility: AdmissibilityState::Admissible,
            next_admissibility: AdmissibilityState::NotAdmissible,
            cause_event_ids: vec!["event_001".into(), "event_002".into()],
            cause_summary: "source deleted and authority changed".into(),
            changed_at: "2026-04-15T20:05:00Z".into(),
        };

        assert!(record.validate().is_ok());
    }
}

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use crate::durable_evidence::{
    ArtifactInvalidationEvidenceRecord, EventReceiptRecord, EvidenceAdmissionResult,
    EvidenceTargetKind, PacketReevaluationEvidenceRecord, RemediationEvidenceRecord,
};
use crate::enums::{AdmissibilityState, EventType, FreshnessState};
use crate::evidence_bundle::{build_and_write_replay_bundle, EvidenceBundleError};
use crate::evidence_store::{EvidenceStore, EvidenceStoreError};
use crate::replay::{replay_bundle_by_id, ReplayError};

#[derive(Debug)]
pub enum ReplayScenarioError {
    Io(std::io::Error),
    Store(EvidenceStoreError),
    Bundle(EvidenceBundleError),
    Replay(ReplayError),
}

impl Display for ReplayScenarioError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Store(err) => write!(f, "evidence store error: {err}"),
            Self::Bundle(err) => write!(f, "evidence bundle error: {err}"),
            Self::Replay(err) => write!(f, "replay error: {err}"),
        }
    }
}

impl Error for ReplayScenarioError {}

impl From<std::io::Error> for ReplayScenarioError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<EvidenceStoreError> for ReplayScenarioError {
    fn from(value: EvidenceStoreError) -> Self {
        Self::Store(value)
    }
}

impl From<EvidenceBundleError> for ReplayScenarioError {
    fn from(value: EvidenceBundleError) -> Self {
        Self::Bundle(value)
    }
}

impl From<ReplayError> for ReplayScenarioError {
    fn from(value: ReplayError) -> Self {
        Self::Replay(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayScenarioReport {
    pub artifact_invalidation_count: usize,
    pub event_receipt_count: usize,
    pub mismatch_count: usize,
    pub packet_reevaluation_count: usize,
    pub proof_root: PathBuf,
    pub remediation_count: usize,
    pub replay_bundle_id: String,
    pub replay_ok: bool,
}

pub fn run_replay_scenario_proof(
    repo_root: &Path,
) -> Result<ReplayScenarioReport, ReplayScenarioError> {
    let proof_root = repo_root.join("target/proof_artifacts/slice12_smoke");

    if proof_root.exists() {
        fs::remove_dir_all(&proof_root)?;
    }

    let store = EvidenceStore::new(&proof_root);

    let event_receipt = EventReceiptRecord {
        receipt_id: "receipt_001".into(),
        event_id: "event_001".into(),
        idempotency_key: "idem_001".into(),
        event_kind: EventType::SourceChanged,
        repo_id: "precomputed-context-core".into(),
        correlated_scope: "src".into(),
        received_at: "2026-04-15T20:20:00Z".into(),
        admission_result: EvidenceAdmissionResult::Accepted,
        admission_reason: "event accepted".into(),
        coalesced_batch_id: Some("batch_001".into()),
        source_digest: "sha256:abc123".into(),
    };

    let artifact_record = ArtifactInvalidationEvidenceRecord {
        invalidation_record_id: "inv_001".into(),
        artifact_id: "artifact_001".into(),
        prior_freshness: FreshnessState::Fresh,
        next_freshness: FreshnessState::Invalidated,
        prior_admissibility: AdmissibilityState::Admissible,
        next_admissibility: AdmissibilityState::NotAdmissible,
        cause_event_ids: vec!["event_001".into()],
        cause_summary: "source deleted".into(),
        changed_at: "2026-04-15T20:21:00Z".into(),
    };

    let packet_record = PacketReevaluationEvidenceRecord {
        packet_reevaluation_id: "pkt_reval_001".into(),
        packet_id: "packet_001".into(),
        prior_admissibility: AdmissibilityState::Admissible,
        next_admissibility: AdmissibilityState::Restricted,
        constituent_artifact_ids: vec!["artifact_001".into()],
        trigger_invalidation_record_ids: vec!["inv_001".into()],
        changed_at: "2026-04-15T20:22:00Z".into(),
        change_summary: "constituent degraded".into(),
    };

    let remediation_record = RemediationEvidenceRecord {
        remediation_record_id: "rem_001".into(),
        target_kind: EvidenceTargetKind::Packet,
        target_id: "packet_001".into(),
        blocking: true,
        trigger_summary: "packet blocked by invalidated constituent".into(),
        recommended_actions: vec!["regenerate packet".into()],
        generated_at: "2026-04-15T20:23:00Z".into(),
    };

    store.write_event_receipt(&event_receipt)?;
    store.write_artifact_invalidation(&artifact_record)?;
    store.write_packet_reevaluation(&packet_record)?;
    store.write_remediation(&remediation_record)?;

    let manifest = build_and_write_replay_bundle(
        &store,
        "2026-04-15T20:24:00Z",
        "precomputed-context-core",
        &[event_receipt],
        &[artifact_record],
        &[packet_record],
        &[remediation_record],
    )?;

    let report = replay_bundle_by_id(&proof_root, &manifest.replay_bundle_id)?;

    Ok(ReplayScenarioReport {
        artifact_invalidation_count: report.artifact_invalidation_count,
        event_receipt_count: report.event_receipt_count,
        mismatch_count: report.mismatches.len(),
        packet_reevaluation_count: report.packet_reevaluation_count,
        proof_root,
        remediation_count: report.remediation_count,
        replay_bundle_id: report.replay_bundle_id,
        replay_ok: report.replay_ok,
    })
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn replay_scenario_proof_passes() {
        let root = unique_test_root("replay_scenario_proof_passes");

        let report = run_replay_scenario_proof(&root).expect("replay scenario should succeed");

        assert!(report.replay_ok);
        assert_eq!(report.event_receipt_count, 1);
        assert_eq!(report.artifact_invalidation_count, 1);
        assert_eq!(report.packet_reevaluation_count, 1);
        assert_eq!(report.remediation_count, 1);
        assert_eq!(report.mismatch_count, 0);

        cleanup_root(&root);
    }

    fn unique_test_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();

        env::temp_dir().join(format!("precomputed_context_core_{label}_{nanos}"))
    }

    fn cleanup_root(root: &Path) {
        if root.exists() {
            fs::remove_dir_all(root).expect("test cleanup should remove temp directory");
        }
    }
}

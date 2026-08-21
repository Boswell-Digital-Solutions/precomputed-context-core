use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

use crate::durable_evidence::ReplayBundleManifest;
use crate::evidence_bundle::{
    build_replay_bundle_manifest, load_evidence_bundle, EvidenceBundleData, EvidenceBundleError,
};

#[derive(Debug)]
pub enum ReplayError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    Bundle(EvidenceBundleError),
    Validation(String),
}

impl Display for ReplayError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Serde(err) => write!(f, "serialization error: {err}"),
            Self::Bundle(err) => write!(f, "evidence bundle error: {err}"),
            Self::Validation(message) => write!(f, "validation error: {message}"),
        }
    }
}

impl Error for ReplayError {}

impl From<std::io::Error> for ReplayError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ReplayError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<EvidenceBundleError> for ReplayError {
    fn from(value: EvidenceBundleError) -> Self {
        Self::Bundle(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayReport {
    pub replay_bundle_id: String,
    pub replay_ok: bool,
    pub expected_proof_digest: String,
    pub recomputed_proof_digest: String,
    pub expected_final_summary: String,
    pub recomputed_final_summary: String,
    pub event_receipt_count: usize,
    pub artifact_invalidation_count: usize,
    pub packet_reevaluation_count: usize,
    pub remediation_count: usize,
    pub mismatches: Vec<String>,
}

pub fn load_replay_bundle_manifest(
    root: &Path,
    replay_bundle_id: &str,
) -> Result<ReplayBundleManifest, ReplayError> {
    validate_nonempty("replay_bundle_id", replay_bundle_id)?;

    let path = root.join("replay_bundles").join(format!(
        "{}.json",
        sanitize_file_component(replay_bundle_id)
    ));

    if !path.exists() {
        return Err(ReplayError::Validation(format!(
            "replay bundle manifest not found: {}",
            path.display()
        )));
    }

    let bytes = fs::read(&path)?;
    let manifest = serde_json::from_slice::<ReplayBundleManifest>(&bytes)?;
    manifest.validate().map_err(ReplayError::Validation)?;

    Ok(manifest)
}

pub fn replay_bundle_by_id(
    root: &Path,
    replay_bundle_id: &str,
) -> Result<ReplayReport, ReplayError> {
    let manifest = load_replay_bundle_manifest(root, replay_bundle_id)?;
    replay_bundle(root, &manifest)
}

pub fn replay_bundle(
    root: &Path,
    manifest: &ReplayBundleManifest,
) -> Result<ReplayReport, ReplayError> {
    manifest.validate().map_err(ReplayError::Validation)?;

    let loaded = load_evidence_bundle(root, manifest)?;
    build_replay_report(manifest, &loaded)
}

fn build_replay_report(
    manifest: &ReplayBundleManifest,
    loaded: &EvidenceBundleData,
) -> Result<ReplayReport, ReplayError> {
    let recomputed = build_replay_bundle_manifest(
        &manifest.created_at,
        &manifest.repo_id,
        &loaded.event_receipts,
        &loaded.artifact_invalidation_records,
        &loaded.packet_reevaluation_records,
        &loaded.remediation_records,
    )
    .map_err(|err| ReplayError::Validation(err.to_string()))?;

    let mut mismatches = Vec::new();

    for receipt in &loaded.event_receipts {
        if receipt.repo_id != manifest.repo_id {
            mismatches.push(format!(
                "event receipt '{}' repo_id '{}' does not match manifest repo_id '{}'",
                receipt.receipt_id, receipt.repo_id, manifest.repo_id
            ));
        }
    }

    compare_vec(
        "event_receipt_ids",
        &manifest.event_receipt_ids,
        &recomputed.event_receipt_ids,
        &mut mismatches,
    );
    compare_vec(
        "artifact_invalidation_record_ids",
        &manifest.artifact_invalidation_record_ids,
        &recomputed.artifact_invalidation_record_ids,
        &mut mismatches,
    );
    compare_vec(
        "packet_reevaluation_record_ids",
        &manifest.packet_reevaluation_record_ids,
        &recomputed.packet_reevaluation_record_ids,
        &mut mismatches,
    );
    compare_vec(
        "remediation_record_ids",
        &manifest.remediation_record_ids,
        &recomputed.remediation_record_ids,
        &mut mismatches,
    );

    if manifest.final_summary != recomputed.final_summary {
        mismatches.push(format!(
            "final_summary mismatch: expected '{}' but recomputed '{}'",
            manifest.final_summary, recomputed.final_summary
        ));
    }

    if manifest.proof_digest != recomputed.proof_digest {
        mismatches.push(format!(
            "proof_digest mismatch: expected '{}' but recomputed '{}'",
            manifest.proof_digest, recomputed.proof_digest
        ));
    }

    if manifest.replay_bundle_id != recomputed.replay_bundle_id {
        mismatches.push(format!(
            "replay_bundle_id mismatch: expected '{}' but recomputed '{}'",
            manifest.replay_bundle_id, recomputed.replay_bundle_id
        ));
    }

    Ok(ReplayReport {
        replay_bundle_id: manifest.replay_bundle_id.clone(),
        replay_ok: mismatches.is_empty(),
        expected_proof_digest: manifest.proof_digest.clone(),
        recomputed_proof_digest: recomputed.proof_digest,
        expected_final_summary: manifest.final_summary.clone(),
        recomputed_final_summary: recomputed.final_summary,
        event_receipt_count: loaded.event_receipts.len(),
        artifact_invalidation_count: loaded.artifact_invalidation_records.len(),
        packet_reevaluation_count: loaded.packet_reevaluation_records.len(),
        remediation_count: loaded.remediation_records.len(),
        mismatches,
    })
}

fn compare_vec(label: &str, expected: &[String], actual: &[String], mismatches: &mut Vec<String>) {
    if expected != actual {
        mismatches.push(format!(
            "{label} mismatch: expected [{}] but recomputed [{}]",
            expected.join(", "),
            actual.join(", ")
        ));
    }
}

fn sanitize_file_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect();

    if sanitized.trim_matches('_').is_empty() {
        "record".to_string()
    } else {
        sanitized
    }
}

fn validate_nonempty(label: &str, value: &str) -> Result<(), ReplayError> {
    if value.trim().is_empty() {
        return Err(ReplayError::Validation(format!(
            "{label} must not be empty"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::durable_evidence::{
        ArtifactInvalidationEvidenceRecord, EventReceiptRecord, EvidenceAdmissionResult,
        EvidenceTargetKind, PacketReevaluationEvidenceRecord, RemediationEvidenceRecord,
    };
    use crate::enums::{AdmissibilityState, EventType, FreshnessState};
    use crate::evidence_bundle::build_and_write_replay_bundle;
    use crate::evidence_store::EvidenceStore;

    use super::*;

    #[test]
    fn replay_bundle_by_id_round_trip_passes() {
        let root = unique_test_root("replay_bundle_by_id_round_trip_passes");
        let store = EvidenceStore::new(&root);

        let event_receipt = sample_event_receipt();
        let artifact_record = sample_artifact_invalidation_record();
        let packet_record = sample_packet_reevaluation_record();
        let remediation_record = sample_remediation_record();

        store
            .write_event_receipt(&event_receipt)
            .expect("event receipt should write");
        store
            .write_artifact_invalidation(&artifact_record)
            .expect("artifact invalidation should write");
        store
            .write_packet_reevaluation(&packet_record)
            .expect("packet reevaluation should write");
        store
            .write_remediation(&remediation_record)
            .expect("remediation should write");

        let manifest = build_and_write_replay_bundle(
            &store,
            "2026-04-15T20:15:00Z",
            "forgecommand",
            &[event_receipt],
            &[artifact_record],
            &[packet_record],
            &[remediation_record],
        )
        .expect("bundle build should succeed");

        let report = replay_bundle_by_id(&root, &manifest.replay_bundle_id)
            .expect("replay by id should succeed");

        assert!(report.replay_ok);
        assert_eq!(report.event_receipt_count, 1);
        assert_eq!(report.artifact_invalidation_count, 1);
        assert_eq!(report.packet_reevaluation_count, 1);
        assert_eq!(report.remediation_count, 1);
        assert!(report.mismatches.is_empty());

        cleanup_root(&root);
    }

    #[test]
    fn replay_bundle_detects_manifest_digest_mutation() {
        let root = unique_test_root("replay_bundle_detects_manifest_digest_mutation");
        let store = EvidenceStore::new(&root);

        let event_receipt = sample_event_receipt();
        let artifact_record = sample_artifact_invalidation_record();
        let packet_record = sample_packet_reevaluation_record();
        let remediation_record = sample_remediation_record();

        store
            .write_event_receipt(&event_receipt)
            .expect("event receipt should write");
        store
            .write_artifact_invalidation(&artifact_record)
            .expect("artifact invalidation should write");
        store
            .write_packet_reevaluation(&packet_record)
            .expect("packet reevaluation should write");
        store
            .write_remediation(&remediation_record)
            .expect("remediation should write");

        let mut manifest = build_and_write_replay_bundle(
            &store,
            "2026-04-15T20:15:00Z",
            "forgecommand",
            &[event_receipt],
            &[artifact_record],
            &[packet_record],
            &[remediation_record],
        )
        .expect("bundle build should succeed");

        manifest.proof_digest = "fnv1a64:tampered000000000".into();

        let report = replay_bundle(&root, &manifest).expect("replay should still return report");

        assert!(!report.replay_ok);
        assert!(report
            .mismatches
            .iter()
            .any(|entry| entry.contains("proof_digest mismatch")));

        cleanup_root(&root);
    }

    fn sample_event_receipt() -> EventReceiptRecord {
        EventReceiptRecord {
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
        }
    }

    fn sample_artifact_invalidation_record() -> ArtifactInvalidationEvidenceRecord {
        ArtifactInvalidationEvidenceRecord {
            invalidation_record_id: "inv_001".into(),
            artifact_id: "artifact_001".into(),
            prior_freshness: FreshnessState::Fresh,
            next_freshness: FreshnessState::Invalidated,
            prior_admissibility: AdmissibilityState::Admissible,
            next_admissibility: AdmissibilityState::NotAdmissible,
            cause_event_ids: vec!["event_001".into()],
            cause_summary: "source deleted".into(),
            changed_at: "2026-04-15T20:06:00Z".into(),
        }
    }

    fn sample_packet_reevaluation_record() -> PacketReevaluationEvidenceRecord {
        PacketReevaluationEvidenceRecord {
            packet_reevaluation_id: "pkt_reval_001".into(),
            packet_id: "packet_001".into(),
            prior_admissibility: AdmissibilityState::Admissible,
            next_admissibility: AdmissibilityState::Restricted,
            constituent_artifact_ids: vec!["artifact_001".into()],
            trigger_invalidation_record_ids: vec!["inv_001".into()],
            changed_at: "2026-04-15T20:07:00Z".into(),
            change_summary: "constituent degraded".into(),
        }
    }

    fn sample_remediation_record() -> RemediationEvidenceRecord {
        RemediationEvidenceRecord {
            remediation_record_id: "rem_001".into(),
            target_kind: EvidenceTargetKind::Packet,
            target_id: "packet_001".into(),
            blocking: true,
            trigger_summary: "packet blocked by invalidated constituent".into(),
            recommended_actions: vec!["regenerate packet".into()],
            generated_at: "2026-04-15T20:08:00Z".into(),
        }
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

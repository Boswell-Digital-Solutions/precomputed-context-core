use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::durable_evidence::{
    ArtifactInvalidationEvidenceRecord, EventReceiptRecord, PacketReevaluationEvidenceRecord,
    RemediationEvidenceRecord, ReplayBundleManifest,
};
use crate::evidence_store::{EvidenceStore, EvidenceStoreError};

#[derive(Debug)]
pub enum EvidenceBundleError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    Validation(String),
    Store(EvidenceStoreError),
    MissingRecord(PathBuf),
}

impl Display for EvidenceBundleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Serde(err) => write!(f, "serialization error: {err}"),
            Self::Validation(message) => write!(f, "validation error: {message}"),
            Self::Store(err) => write!(f, "evidence store error: {err}"),
            Self::MissingRecord(path) => write!(f, "missing evidence record: {}", path.display()),
        }
    }
}

impl Error for EvidenceBundleError {}

impl From<std::io::Error> for EvidenceBundleError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for EvidenceBundleError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<EvidenceStoreError> for EvidenceBundleError {
    fn from(value: EvidenceStoreError) -> Self {
        Self::Store(value)
    }
}

#[derive(Debug, Clone)]
pub struct EvidenceBundleData {
    pub manifest: ReplayBundleManifest,
    pub event_receipts: Vec<EventReceiptRecord>,
    pub artifact_invalidation_records: Vec<ArtifactInvalidationEvidenceRecord>,
    pub packet_reevaluation_records: Vec<PacketReevaluationEvidenceRecord>,
    pub remediation_records: Vec<RemediationEvidenceRecord>,
}

pub fn build_replay_bundle_manifest(
    created_at: &str,
    repo_id: &str,
    event_receipts: &[EventReceiptRecord],
    artifact_invalidation_records: &[ArtifactInvalidationEvidenceRecord],
    packet_reevaluation_records: &[PacketReevaluationEvidenceRecord],
    remediation_records: &[RemediationEvidenceRecord],
) -> Result<ReplayBundleManifest, EvidenceBundleError> {
    validate_nonempty("created_at", created_at)?;
    validate_nonempty("repo_id", repo_id)?;

    if event_receipts.is_empty() {
        return Err(EvidenceBundleError::Validation(
            "event_receipts must not be empty".into(),
        ));
    }

    let mut ordered_event_receipts = event_receipts.to_vec();
    ordered_event_receipts.sort_by(|left, right| {
        left.received_at
            .cmp(&right.received_at)
            .then_with(|| left.receipt_id.cmp(&right.receipt_id))
    });

    let mut ordered_artifact_records = artifact_invalidation_records.to_vec();
    ordered_artifact_records.sort_by(|left, right| {
        left.changed_at.cmp(&right.changed_at).then_with(|| {
            left.invalidation_record_id
                .cmp(&right.invalidation_record_id)
        })
    });

    let mut ordered_packet_records = packet_reevaluation_records.to_vec();
    ordered_packet_records.sort_by(|left, right| {
        left.changed_at.cmp(&right.changed_at).then_with(|| {
            left.packet_reevaluation_id
                .cmp(&right.packet_reevaluation_id)
        })
    });

    let mut ordered_remediation_records = remediation_records.to_vec();
    ordered_remediation_records.sort_by(|left, right| {
        left.generated_at
            .cmp(&right.generated_at)
            .then_with(|| left.remediation_record_id.cmp(&right.remediation_record_id))
    });

    let event_receipt_ids: Vec<String> = ordered_event_receipts
        .iter()
        .map(|record| record.receipt_id.clone())
        .collect();

    let artifact_invalidation_record_ids: Vec<String> = ordered_artifact_records
        .iter()
        .map(|record| record.invalidation_record_id.clone())
        .collect();

    let packet_reevaluation_record_ids: Vec<String> = ordered_packet_records
        .iter()
        .map(|record| record.packet_reevaluation_id.clone())
        .collect();

    let remediation_record_ids: Vec<String> = ordered_remediation_records
        .iter()
        .map(|record| record.remediation_record_id.clone())
        .collect();

    let final_summary = build_final_summary(
        &event_receipt_ids,
        &artifact_invalidation_record_ids,
        &packet_reevaluation_record_ids,
        &remediation_record_ids,
    );

    let digest_material = build_digest_material(
        created_at,
        repo_id,
        &event_receipt_ids,
        &artifact_invalidation_record_ids,
        &packet_reevaluation_record_ids,
        &remediation_record_ids,
        &final_summary,
    );
    let digest_hex = stable_digest_hex(digest_material.as_bytes());

    let manifest = ReplayBundleManifest {
        replay_bundle_id: format!("bundle_{digest_hex}"),
        created_at: created_at.into(),
        repo_id: repo_id.into(),
        event_receipt_ids,
        artifact_invalidation_record_ids,
        packet_reevaluation_record_ids,
        remediation_record_ids,
        final_summary,
        proof_digest: format!("fnv1a64:{digest_hex}"),
    };

    manifest
        .validate()
        .map_err(EvidenceBundleError::Validation)?;

    Ok(manifest)
}

pub fn build_and_write_replay_bundle(
    store: &EvidenceStore,
    created_at: &str,
    repo_id: &str,
    event_receipts: &[EventReceiptRecord],
    artifact_invalidation_records: &[ArtifactInvalidationEvidenceRecord],
    packet_reevaluation_records: &[PacketReevaluationEvidenceRecord],
    remediation_records: &[RemediationEvidenceRecord],
) -> Result<ReplayBundleManifest, EvidenceBundleError> {
    let manifest = build_replay_bundle_manifest(
        created_at,
        repo_id,
        event_receipts,
        artifact_invalidation_records,
        packet_reevaluation_records,
        remediation_records,
    )?;

    store.write_replay_bundle(&manifest)?;
    Ok(manifest)
}

pub fn load_evidence_bundle(
    root: &Path,
    manifest: &ReplayBundleManifest,
) -> Result<EvidenceBundleData, EvidenceBundleError> {
    manifest
        .validate()
        .map_err(EvidenceBundleError::Validation)?;

    let event_receipts =
        load_record_set::<EventReceiptRecord>(root, "event_receipts", &manifest.event_receipt_ids)?;

    let artifact_invalidation_records = load_record_set::<ArtifactInvalidationEvidenceRecord>(
        root,
        "artifact_invalidation_records",
        &manifest.artifact_invalidation_record_ids,
    )?;

    let packet_reevaluation_records = load_record_set::<PacketReevaluationEvidenceRecord>(
        root,
        "packet_reevaluation_records",
        &manifest.packet_reevaluation_record_ids,
    )?;

    let remediation_records = load_record_set::<RemediationEvidenceRecord>(
        root,
        "remediation_records",
        &manifest.remediation_record_ids,
    )?;

    Ok(EvidenceBundleData {
        manifest: manifest.clone(),
        event_receipts,
        artifact_invalidation_records,
        packet_reevaluation_records,
        remediation_records,
    })
}

fn load_record_set<T: DeserializeOwned>(
    root: &Path,
    subdir: &str,
    ids: &[String],
) -> Result<Vec<T>, EvidenceBundleError> {
    let mut records = Vec::with_capacity(ids.len());

    for id in ids {
        let path = root
            .join(subdir)
            .join(format!("{}.json", sanitize_file_component(id)));

        if !path.exists() {
            return Err(EvidenceBundleError::MissingRecord(path));
        }

        let bytes = fs::read(&path)?;
        let record = serde_json::from_slice::<T>(&bytes)?;
        records.push(record);
    }

    Ok(records)
}

fn build_final_summary(
    event_receipt_ids: &[String],
    artifact_invalidation_record_ids: &[String],
    packet_reevaluation_record_ids: &[String],
    remediation_record_ids: &[String],
) -> String {
    format!(
        "event_receipts={} artifact_invalidations={} packet_reevaluations={} remediations={}",
        event_receipt_ids.len(),
        artifact_invalidation_record_ids.len(),
        packet_reevaluation_record_ids.len(),
        remediation_record_ids.len()
    )
}

fn build_digest_material(
    created_at: &str,
    repo_id: &str,
    event_receipt_ids: &[String],
    artifact_invalidation_record_ids: &[String],
    packet_reevaluation_record_ids: &[String],
    remediation_record_ids: &[String],
    final_summary: &str,
) -> String {
    let mut material = String::new();
    material.push_str("created_at=");
    material.push_str(created_at);
    material.push('|');
    material.push_str("repo_id=");
    material.push_str(repo_id);
    material.push('|');
    material.push_str("event_receipt_ids=");
    material.push_str(&event_receipt_ids.join(","));
    material.push('|');
    material.push_str("artifact_invalidation_record_ids=");
    material.push_str(&artifact_invalidation_record_ids.join(","));
    material.push('|');
    material.push_str("packet_reevaluation_record_ids=");
    material.push_str(&packet_reevaluation_record_ids.join(","));
    material.push('|');
    material.push_str("remediation_record_ids=");
    material.push_str(&remediation_record_ids.join(","));
    material.push('|');
    material.push_str("final_summary=");
    material.push_str(final_summary);
    material
}

fn stable_digest_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;

    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("{hash:016x}")
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

fn validate_nonempty(label: &str, value: &str) -> Result<(), EvidenceBundleError> {
    if value.trim().is_empty() {
        return Err(EvidenceBundleError::Validation(format!(
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

    use super::*;

    #[test]
    fn build_and_write_replay_bundle_persists_manifest() {
        let root = unique_test_root("build_and_write_replay_bundle_persists_manifest");
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
            "2026-04-15T20:10:00Z",
            "forgecommand",
            &[event_receipt],
            &[artifact_record],
            &[packet_record],
            &[remediation_record],
        )
        .expect("bundle build should succeed");

        let manifest_path = root
            .join("replay_bundles")
            .join(format!("{}.json", manifest.replay_bundle_id));

        assert!(manifest_path.exists());

        let contents = fs::read_to_string(&manifest_path).expect("manifest should be readable");
        assert!(contents.contains("\"repo_id\": \"forgecommand\""));
        assert!(contents.contains("\"proof_digest\": \"fnv1a64:"));

        cleanup_root(&root);
    }

    #[test]
    fn load_evidence_bundle_reads_referenced_records() {
        let root = unique_test_root("load_evidence_bundle_reads_referenced_records");
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

        let manifest = build_replay_bundle_manifest(
            "2026-04-15T20:10:00Z",
            "forgecommand",
            std::slice::from_ref(&event_receipt),
            std::slice::from_ref(&artifact_record),
            std::slice::from_ref(&packet_record),
            std::slice::from_ref(&remediation_record),
        )
        .expect("manifest build should succeed");

        let loaded = load_evidence_bundle(&root, &manifest).expect("bundle should load");

        assert_eq!(loaded.manifest.replay_bundle_id, manifest.replay_bundle_id);
        assert_eq!(loaded.event_receipts.len(), 1);
        assert_eq!(loaded.artifact_invalidation_records.len(), 1);
        assert_eq!(loaded.packet_reevaluation_records.len(), 1);
        assert_eq!(loaded.remediation_records.len(), 1);
        assert_eq!(loaded.event_receipts[0].receipt_id, "receipt_001");

        cleanup_root(&root);
    }

    #[test]
    fn load_evidence_bundle_fails_closed_on_missing_record() {
        let root = unique_test_root("load_evidence_bundle_fails_closed_on_missing_record");

        let manifest = ReplayBundleManifest {
            replay_bundle_id: "bundle_missing_case".into(),
            created_at: "2026-04-15T20:10:00Z".into(),
            repo_id: "forgecommand".into(),
            event_receipt_ids: vec!["receipt_missing".into()],
            artifact_invalidation_record_ids: vec![],
            packet_reevaluation_record_ids: vec![],
            remediation_record_ids: vec![],
            final_summary:
                "event_receipts=1 artifact_invalidations=0 packet_reevaluations=0 remediations=0"
                    .into(),
            proof_digest: "fnv1a64:deadbeefdeadbeef".into(),
        };

        let err = load_evidence_bundle(&root, &manifest).expect_err("load should fail");

        match err {
            EvidenceBundleError::MissingRecord(path) => {
                assert!(path.ends_with("event_receipts/receipt_missing.json"));
            }
            other => panic!("expected MissingRecord error, got {other}"),
        }

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

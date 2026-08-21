use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::durable_evidence::{
    ArtifactInvalidationEvidenceRecord, CoalescedBatchRecord, EventReceiptRecord,
    PacketReevaluationEvidenceRecord, RemediationEvidenceRecord, ReplayBundleManifest,
};

#[derive(Debug)]
pub enum EvidenceStoreError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    Validation(String),
    AlreadyExists(PathBuf),
}

impl EvidenceStoreError {
    fn validation(message: String) -> Self {
        Self::Validation(message)
    }
}

impl Display for EvidenceStoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Serde(err) => write!(f, "serialization error: {err}"),
            Self::Validation(message) => write!(f, "validation error: {message}"),
            Self::AlreadyExists(path) => {
                write!(
                    f,
                    "append-only write refused because file already exists: {}",
                    path.display()
                )
            }
        }
    }
}

impl Error for EvidenceStoreError {}

impl From<std::io::Error> for EvidenceStoreError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for EvidenceStoreError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

#[derive(Debug, Clone)]
pub struct EvidenceStore {
    root: PathBuf,
}

impl EvidenceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn init(&self) -> Result<(), EvidenceStoreError> {
        for dir in [
            "event_receipts",
            "coalesced_batches",
            "artifact_invalidation_records",
            "packet_reevaluation_records",
            "remediation_records",
            "replay_bundles",
        ] {
            fs::create_dir_all(self.root.join(dir))?;
        }

        Ok(())
    }

    pub fn write_event_receipt(
        &self,
        record: &EventReceiptRecord,
    ) -> Result<PathBuf, EvidenceStoreError> {
        record.validate().map_err(EvidenceStoreError::validation)?;
        self.write_record("event_receipts", &record.receipt_id, record)
    }

    pub fn write_coalesced_batch(
        &self,
        record: &CoalescedBatchRecord,
    ) -> Result<PathBuf, EvidenceStoreError> {
        record.validate().map_err(EvidenceStoreError::validation)?;
        self.write_record("coalesced_batches", &record.batch_id, record)
    }

    pub fn write_artifact_invalidation(
        &self,
        record: &ArtifactInvalidationEvidenceRecord,
    ) -> Result<PathBuf, EvidenceStoreError> {
        record.validate().map_err(EvidenceStoreError::validation)?;
        self.write_record(
            "artifact_invalidation_records",
            &record.invalidation_record_id,
            record,
        )
    }

    pub fn write_packet_reevaluation(
        &self,
        record: &PacketReevaluationEvidenceRecord,
    ) -> Result<PathBuf, EvidenceStoreError> {
        record.validate().map_err(EvidenceStoreError::validation)?;
        self.write_record(
            "packet_reevaluation_records",
            &record.packet_reevaluation_id,
            record,
        )
    }

    pub fn write_remediation(
        &self,
        record: &RemediationEvidenceRecord,
    ) -> Result<PathBuf, EvidenceStoreError> {
        record.validate().map_err(EvidenceStoreError::validation)?;
        self.write_record("remediation_records", &record.remediation_record_id, record)
    }

    pub fn write_replay_bundle(
        &self,
        manifest: &ReplayBundleManifest,
    ) -> Result<PathBuf, EvidenceStoreError> {
        manifest
            .validate()
            .map_err(EvidenceStoreError::validation)?;
        self.write_record("replay_bundles", &manifest.replay_bundle_id, manifest)
    }

    fn write_record<T: Serialize>(
        &self,
        subdir: &str,
        record_id: &str,
        value: &T,
    ) -> Result<PathBuf, EvidenceStoreError> {
        self.init()?;

        let filename = format!("{}.json", sanitize_file_component(record_id));
        let path = self.root.join(subdir).join(filename);

        if path.exists() {
            return Err(EvidenceStoreError::AlreadyExists(path));
        }

        let bytes = serde_json::to_vec_pretty(value)?;
        write_new_file(&path, &bytes)?;
        Ok(path)
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

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), EvidenceStoreError> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;

    file.write_all(bytes)?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::durable_evidence::{
        EventReceiptRecord, EvidenceAdmissionResult, ReplayBundleManifest,
    };
    use crate::enums::EventType;

    use super::*;

    #[test]
    fn writes_event_receipt_record_to_expected_location() {
        let root = unique_test_root("writes_event_receipt_record_to_expected_location");
        let store = EvidenceStore::new(&root);

        let record = sample_event_receipt();

        let written_path = store
            .write_event_receipt(&record)
            .expect("event receipt write should succeed");

        assert!(written_path.exists());

        let contents = fs::read_to_string(&written_path).expect("expected written json file");
        assert!(contents.contains("\"receipt_id\": \"receipt_001\""));
        assert!(contents.contains("\"admission_result\": \"accepted\""));

        cleanup_root(&root);
    }

    #[test]
    fn refuses_duplicate_append_for_same_record_id() {
        let root = unique_test_root("refuses_duplicate_append_for_same_record_id");
        let store = EvidenceStore::new(&root);

        let record = sample_event_receipt();

        store
            .write_event_receipt(&record)
            .expect("first write should succeed");

        let err = store
            .write_event_receipt(&record)
            .expect_err("second write should fail");

        match err {
            EvidenceStoreError::AlreadyExists(path) => {
                assert!(path.ends_with("event_receipts/receipt_001.json"));
            }
            other => panic!("expected AlreadyExists error, got {other}"),
        }

        cleanup_root(&root);
    }

    #[test]
    fn writes_replay_bundle_manifest() {
        let root = unique_test_root("writes_replay_bundle_manifest");
        let store = EvidenceStore::new(&root);

        let manifest = ReplayBundleManifest {
            replay_bundle_id: "bundle_001".into(),
            created_at: "2026-04-15T20:05:00Z".into(),
            repo_id: "forgecommand".into(),
            event_receipt_ids: vec!["receipt_001".into()],
            artifact_invalidation_record_ids: vec!["inv_001".into()],
            packet_reevaluation_record_ids: vec!["pkt_reval_001".into()],
            remediation_record_ids: vec!["rem_001".into()],
            final_summary: "final posture recorded".into(),
            proof_digest: "sha256:def456".into(),
        };

        let written_path = store
            .write_replay_bundle(&manifest)
            .expect("replay bundle write should succeed");

        assert!(written_path.exists());

        let contents = fs::read_to_string(&written_path).expect("expected written json file");
        assert!(contents.contains("\"replay_bundle_id\": \"bundle_001\""));
        assert!(contents.contains("\"proof_digest\": \"sha256:def456\""));

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

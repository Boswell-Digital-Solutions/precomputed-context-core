use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::proof::replay_scenario::{
    run_replay_scenario_proof, ReplayScenarioError, ReplayScenarioReport,
};
use crate::proof::report::GovernedFlowReport;
use crate::proof::scenario::run_governed_flow_proof;

const EXPORT_ROOT: &str = "target/proof_artifacts/slice13_export";
const GOVERNED_FLOW_REPORT_FILENAME: &str = "governed_flow_report.json";
const REPLAY_REPORT_FILENAME: &str = "replay_report.json";
const REPLAY_BUNDLE_MANIFEST_FILENAME: &str = "replay_bundle_manifest.json";
const PACKAGE_INDEX_FILENAME: &str = "package_index.json";
const README_FILENAME: &str = "README.txt";

#[derive(Debug)]
pub enum ProofExportError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    ReplayScenario(ReplayScenarioError),
    MissingReplayManifest(PathBuf),
}

impl Display for ProofExportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Serde(err) => write!(f, "serialization error: {err}"),
            Self::ReplayScenario(err) => write!(f, "replay scenario error: {err}"),
            Self::MissingReplayManifest(path) => {
                write!(f, "missing replay manifest: {}", path.display())
            }
        }
    }
}

impl Error for ProofExportError {}

impl From<std::io::Error> for ProofExportError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ProofExportError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<ReplayScenarioError> for ProofExportError {
    fn from(value: ReplayScenarioError) -> Self {
        Self::ReplayScenario(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofExportReport {
    pub package_root: PathBuf,
    pub replay_bundle_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageIndex {
    pub package_root: String,
    pub replay_bundle_id: String,
    pub files: Vec<PackageIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageIndexEntry {
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GovernedFlowReportExport {
    pub scenario_id: String,
    pub deduped_events: usize,
    pub coalesced_batches: usize,
    pub initial_artifact_freshness: String,
    pub final_artifact_freshness: String,
    pub initial_packet_admissibility: String,
    pub final_packet_admissibility: String,
    pub remediation_required: bool,
    pub remediation_count: usize,
    pub affected_artifact_ids: Vec<String>,
    pub unaffected_artifact_ids: Vec<String>,
    pub affected_packet_ids: Vec<String>,
    pub unaffected_packet_ids: Vec<String>,
    pub triggering_event_ids: Vec<String>,
    pub schema_paths: Vec<String>,
    pub steps: Vec<GovernedFlowStepExport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GovernedFlowStepExport {
    pub step: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplayScenarioReportExport {
    pub replay_bundle_id: String,
    pub proof_root: String,
    pub replay_ok: bool,
    pub mismatch_count: usize,
    pub event_receipt_count: usize,
    pub artifact_invalidation_count: usize,
    pub packet_reevaluation_count: usize,
    pub remediation_count: usize,
}

pub fn export_proof_package(repo_root: &Path) -> Result<ProofExportReport, ProofExportError> {
    let governed_report = run_governed_flow_proof(repo_root);
    let replay_report = run_replay_scenario_proof(repo_root)?;

    let package_root = repo_root.join(EXPORT_ROOT);

    if package_root.exists() {
        fs::remove_dir_all(&package_root)?;
    }

    fs::create_dir_all(&package_root)?;

    let governed_export = map_governed_flow_report(&governed_report);
    let replay_export = map_replay_report(&replay_report);

    write_json(
        &package_root.join(GOVERNED_FLOW_REPORT_FILENAME),
        &governed_export,
    )?;
    write_json(&package_root.join(REPLAY_REPORT_FILENAME), &replay_export)?;

    let manifest_source = replay_manifest_source_path(&replay_report);
    let manifest_dest = package_root.join(REPLAY_BUNDLE_MANIFEST_FILENAME);

    if !manifest_source.exists() {
        return Err(ProofExportError::MissingReplayManifest(manifest_source));
    }

    fs::copy(&manifest_source, &manifest_dest)?;

    let package_index = PackageIndex {
        package_root: EXPORT_ROOT.into(),
        replay_bundle_id: replay_report.replay_bundle_id.clone(),
        files: vec![
            PackageIndexEntry {
                kind: "governed_flow_report".into(),
                path: GOVERNED_FLOW_REPORT_FILENAME.into(),
            },
            PackageIndexEntry {
                kind: "replay_report".into(),
                path: REPLAY_REPORT_FILENAME.into(),
            },
            PackageIndexEntry {
                kind: "replay_bundle_manifest".into(),
                path: REPLAY_BUNDLE_MANIFEST_FILENAME.into(),
            },
            PackageIndexEntry {
                kind: "readme".into(),
                path: README_FILENAME.into(),
            },
        ],
    };

    write_json(&package_root.join(PACKAGE_INDEX_FILENAME), &package_index)?;
    fs::write(package_root.join(README_FILENAME), readme_text())?;

    Ok(ProofExportReport {
        package_root,
        replay_bundle_id: replay_report.replay_bundle_id,
    })
}

fn map_governed_flow_report(report: &GovernedFlowReport) -> GovernedFlowReportExport {
    GovernedFlowReportExport {
        scenario_id: report.scenario_id.to_string(),
        deduped_events: report.deduped_events,
        coalesced_batches: report.coalesced_batches,
        initial_artifact_freshness: report.initial_artifact_freshness.to_string(),
        final_artifact_freshness: report.final_artifact_freshness.to_string(),
        initial_packet_admissibility: report.initial_packet_admissibility.to_string(),
        final_packet_admissibility: report.final_packet_admissibility.to_string(),
        remediation_required: report.remediation_required,
        remediation_count: report.remediation_count,
        affected_artifact_ids: report.affected_artifact_ids.clone(),
        unaffected_artifact_ids: report.unaffected_artifact_ids.clone(),
        affected_packet_ids: report.affected_packet_ids.clone(),
        unaffected_packet_ids: report.unaffected_packet_ids.clone(),
        triggering_event_ids: report.triggering_event_ids.clone(),
        schema_paths: report
            .schema_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        steps: report
            .steps
            .iter()
            .map(|step| GovernedFlowStepExport {
                step: step.step.to_string(),
                passed: step.passed,
                detail: step.detail.clone(),
            })
            .collect(),
    }
}

fn map_replay_report(report: &ReplayScenarioReport) -> ReplayScenarioReportExport {
    ReplayScenarioReportExport {
        replay_bundle_id: report.replay_bundle_id.clone(),
        proof_root: report.proof_root.display().to_string(),
        replay_ok: report.replay_ok,
        mismatch_count: report.mismatch_count,
        event_receipt_count: report.event_receipt_count,
        artifact_invalidation_count: report.artifact_invalidation_count,
        packet_reevaluation_count: report.packet_reevaluation_count,
        remediation_count: report.remediation_count,
    }
}

fn replay_manifest_source_path(report: &ReplayScenarioReport) -> PathBuf {
    report
        .proof_root
        .join("replay_bundles")
        .join(format!("{}.json", report.replay_bundle_id))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), ProofExportError> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes)?;
    Ok(())
}

fn readme_text() -> &'static str {
    "Slice 13 proof export package\n\
Generated by precomputed-context-core proof_check export mode.\n\
This package contains the governed flow report, replay report, replay bundle manifest copy, and package index.\n"
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::Value;

    use super::*;

    #[test]
    fn proof_export_writes_expected_files() {
        let root = unique_test_root("proof_export_writes_expected_files");

        let report = export_proof_package(&root).expect("export should succeed");
        let package_root = report.package_root;

        assert!(package_root.join(GOVERNED_FLOW_REPORT_FILENAME).exists());
        assert!(package_root.join(REPLAY_REPORT_FILENAME).exists());
        assert!(package_root.join(REPLAY_BUNDLE_MANIFEST_FILENAME).exists());
        assert!(package_root.join(PACKAGE_INDEX_FILENAME).exists());
        assert!(package_root.join(README_FILENAME).exists());

        let index_bytes =
            fs::read(package_root.join(PACKAGE_INDEX_FILENAME)).expect("index should be readable");
        let index_json: Value =
            serde_json::from_slice(&index_bytes).expect("index should parse as json");

        assert_eq!(
            index_json["replay_bundle_id"].as_str(),
            Some(report.replay_bundle_id.as_str())
        );

        cleanup_root(&root);
    }

    #[test]
    fn proof_export_uses_stable_file_names() {
        let root = unique_test_root("proof_export_uses_stable_file_names");

        let report = export_proof_package(&root).expect("export should succeed");
        let package_root = report.package_root;

        let files = vec![
            package_root.join(GOVERNED_FLOW_REPORT_FILENAME),
            package_root.join(REPLAY_REPORT_FILENAME),
            package_root.join(REPLAY_BUNDLE_MANIFEST_FILENAME),
            package_root.join(PACKAGE_INDEX_FILENAME),
            package_root.join(README_FILENAME),
        ];

        for path in files {
            assert!(path.exists(), "expected file to exist: {}", path.display());
        }

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

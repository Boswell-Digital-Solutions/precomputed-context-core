use crate::consumer_acknowledgment::{
    load_consumer_acknowledgment_receipt, load_return_channel_closure_receipt,
};
use crate::downstream_release::{
    default_downstream_release_package_dir, default_downstream_release_receipt_path,
    load_downstream_release_receipt,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const RELEASE_READINESS_RECEIPT_SCHEMA_VERSION: &str = "proof.release-readiness-receipt.v1";
pub const OPERATOR_RELEASE_SUMMARY_SCHEMA_VERSION: &str = "proof.operator-release-summary.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseReadinessReceipt {
    pub schema_version: String,
    pub downstream_release_receipt_sha256: String,
    pub consumer_acknowledgment_receipt_sha256: String,
    pub return_channel_closure_receipt_sha256: String,
    pub release_member_count: usize,
    pub ready_for_operator_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorReleaseSummary {
    pub schema_version: String,
    pub consumer_name: String,
    pub purpose: String,
    pub release_member_count: usize,
    pub ready_for_operator_review: bool,
}

pub fn default_release_readiness_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice32_downstream_release/current")
}

pub fn default_release_readiness_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice33_release_readiness/current")
}

pub fn default_release_readiness_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("release_readiness_receipt.json")
}

pub fn default_operator_release_summary_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("operator_release_summary.json")
}

pub fn build_release_readiness_receipt(
    downstream_release_workspace_current_dir: &Path,
) -> Result<ReleaseReadinessReceipt, Box<dyn Error>> {
    let release_receipt_path =
        default_downstream_release_receipt_path(downstream_release_workspace_current_dir);
    let release_package_dir =
        default_downstream_release_package_dir(downstream_release_workspace_current_dir);
    validate_downstream_release_package(&release_package_dir, 6)?;

    let release_receipt = load_downstream_release_receipt(&release_receipt_path)?;
    if !release_receipt.release_eligible {
        return Err("downstream release receipt was not eligible".into());
    }

    let acknowledgment_receipt_path =
        release_package_dir.join("consumer_acknowledgment_receipt.json");
    let closure_receipt_path = release_package_dir.join("return_channel_closure_receipt.json");
    let acknowledgment = load_consumer_acknowledgment_receipt(&acknowledgment_receipt_path)?;
    let closure = load_return_channel_closure_receipt(&closure_receipt_path)?;

    if !acknowledgment.acknowledged {
        return Err(
            "consumer acknowledgment in release package did not acknowledge handoff".into(),
        );
    }
    if !closure.closed {
        return Err("return channel closure in release package was not closed".into());
    }

    let actual_ack_sha = sha256_hex_file(&acknowledgment_receipt_path)?;
    let actual_closure_sha = sha256_hex_file(&closure_receipt_path)?;
    if release_receipt.acknowledgment_receipt_sha256 != actual_ack_sha {
        return Err("downstream release receipt acknowledgment continuity mismatch".into());
    }
    if release_receipt.closure_receipt_sha256 != actual_closure_sha {
        return Err("downstream release receipt closure continuity mismatch".into());
    }

    Ok(ReleaseReadinessReceipt {
        schema_version: RELEASE_READINESS_RECEIPT_SCHEMA_VERSION.to_string(),
        downstream_release_receipt_sha256: sha256_hex_file(&release_receipt_path)?,
        consumer_acknowledgment_receipt_sha256: actual_ack_sha,
        return_channel_closure_receipt_sha256: actual_closure_sha,
        release_member_count: 6,
        ready_for_operator_review: true,
    })
}

pub fn build_operator_release_summary(
    downstream_release_workspace_current_dir: &Path,
) -> Result<OperatorReleaseSummary, Box<dyn Error>> {
    let release_package_dir =
        default_downstream_release_package_dir(downstream_release_workspace_current_dir);
    let acknowledgment = load_consumer_acknowledgment_receipt(
        &release_package_dir.join("consumer_acknowledgment_receipt.json"),
    )?;

    Ok(OperatorReleaseSummary {
        schema_version: OPERATOR_RELEASE_SUMMARY_SCHEMA_VERSION.to_string(),
        consumer_name: acknowledgment.consumer_name,
        purpose: acknowledgment.purpose,
        release_member_count: 6,
        ready_for_operator_review: true,
    })
}

pub fn publish_release_readiness(
    workspace_current_dir: &Path,
    receipt: &ReleaseReadinessReceipt,
    summary: &OperatorReleaseSummary,
) -> Result<(), Box<dyn Error>> {
    write_release_readiness_receipt(
        &default_release_readiness_receipt_path(workspace_current_dir),
        receipt,
    )?;
    write_operator_release_summary(
        &default_operator_release_summary_path(workspace_current_dir),
        summary,
    )?;
    Ok(())
}

pub fn write_release_readiness_receipt(
    path: &Path,
    receipt: &ReleaseReadinessReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_release_readiness_receipt(
    path: &Path,
) -> Result<ReleaseReadinessReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: ReleaseReadinessReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

pub fn write_operator_release_summary(
    path: &Path,
    summary: &OperatorReleaseSummary,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(summary)?)?;
    Ok(())
}

pub fn load_operator_release_summary(
    path: &Path,
) -> Result<OperatorReleaseSummary, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let summary: OperatorReleaseSummary = serde_json::from_slice(&bytes)?;
    Ok(summary)
}

pub fn validate_downstream_release_package(
    package_dir: &Path,
    expected_member_count: usize,
) -> Result<(), Box<dyn Error>> {
    let mut file_names = Vec::new();
    for entry in fs::read_dir(package_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            file_names.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    file_names.sort();
    let expected = vec![
        "activation_receipt.json".to_string(),
        "attestation_receipt.json".to_string(),
        "consumer_acknowledgment_receipt.json".to_string(),
        "consumer_contract.json".to_string(),
        "return_channel_closure_receipt.json".to_string(),
        "supersession_chain_receipt.json".to_string(),
    ];
    if file_names != expected {
        return Err("downstream release package member set mismatch".into());
    }
    if file_names.len() != expected_member_count {
        return Err(format!(
            "downstream release package member count mismatch: expected {}, got {}",
            expected_member_count,
            file_names.len()
        )
        .into());
    }
    Ok(())
}

fn sha256_hex_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    Ok(to_hex(&digest))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{:02x}", byte);
    }
    output
}

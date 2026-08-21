use crate::downstream_release::{
    default_downstream_release_package_dir, default_downstream_release_receipt_path,
    load_downstream_release_receipt,
};
use crate::release_readiness::{
    default_operator_release_summary_path, default_release_readiness_receipt_path,
    load_operator_release_summary, load_release_readiness_receipt,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const RELEASE_ATTESTATION_RECEIPT_SCHEMA_VERSION: &str = "proof.release-attestation-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseAttestationReceipt {
    pub schema_version: String,
    pub downstream_release_receipt_sha256: String,
    pub release_readiness_receipt_sha256: String,
    pub operator_release_summary_sha256: String,
    pub export_member_count: usize,
    pub attested_for_handoff: bool,
}

pub fn default_release_attestation_downstream_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice32_downstream_release/current")
}

pub fn default_release_attestation_readiness_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice33_release_readiness/current")
}

pub fn default_release_attestation_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice34_release_attestation/current")
}

pub fn default_release_attestation_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("release_attestation_receipt.json")
}

pub fn default_handoff_boundary_package_dir(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("handoff_boundary_package")
}

pub fn build_release_attestation_receipt(
    downstream_release_workspace_current_dir: &Path,
    readiness_workspace_current_dir: &Path,
) -> Result<ReleaseAttestationReceipt, Box<dyn Error>> {
    let release_receipt_path =
        default_downstream_release_receipt_path(downstream_release_workspace_current_dir);
    let readiness_receipt_path =
        default_release_readiness_receipt_path(readiness_workspace_current_dir);
    let operator_summary_path =
        default_operator_release_summary_path(readiness_workspace_current_dir);

    let release_receipt = load_downstream_release_receipt(&release_receipt_path)?;
    let readiness_receipt = load_release_readiness_receipt(&readiness_receipt_path)?;
    let operator_summary = load_operator_release_summary(&operator_summary_path)?;

    if !release_receipt.release_eligible {
        return Err("downstream release receipt was not eligible".into());
    }
    if !readiness_receipt.ready_for_operator_review {
        return Err("release readiness receipt was not operator-ready".into());
    }
    if !operator_summary.ready_for_operator_review {
        return Err("operator release summary was not operator-ready".into());
    }

    let actual_release_sha = sha256_hex_file(&release_receipt_path)?;
    if readiness_receipt.downstream_release_receipt_sha256 != actual_release_sha {
        return Err("release readiness receipt continuity mismatch".into());
    }

    let release_package_dir =
        default_downstream_release_package_dir(downstream_release_workspace_current_dir);
    let ack_path = release_package_dir.join("consumer_acknowledgment_receipt.json");
    let closure_path = release_package_dir.join("return_channel_closure_receipt.json");
    if !ack_path.exists() {
        return Err("downstream release package missing consumer acknowledgment receipt".into());
    }
    if !closure_path.exists() {
        return Err("downstream release package missing return channel closure receipt".into());
    }
    let actual_ack_sha = sha256_hex_file(&ack_path)?;
    let actual_closure_sha = sha256_hex_file(&closure_path)?;
    if readiness_receipt.consumer_acknowledgment_receipt_sha256 != actual_ack_sha {
        return Err("release readiness acknowledgment continuity mismatch".into());
    }
    if readiness_receipt.return_channel_closure_receipt_sha256 != actual_closure_sha {
        return Err("release readiness closure continuity mismatch".into());
    }

    Ok(ReleaseAttestationReceipt {
        schema_version: RELEASE_ATTESTATION_RECEIPT_SCHEMA_VERSION.to_string(),
        downstream_release_receipt_sha256: actual_release_sha,
        release_readiness_receipt_sha256: sha256_hex_file(&readiness_receipt_path)?,
        operator_release_summary_sha256: sha256_hex_file(&operator_summary_path)?,
        export_member_count: 5,
        attested_for_handoff: true,
    })
}

pub fn publish_release_attestation_package(
    downstream_release_workspace_current_dir: &Path,
    readiness_workspace_current_dir: &Path,
    workspace_current_dir: &Path,
    receipt: &ReleaseAttestationReceipt,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(workspace_current_dir)?;
    let package_dir = default_handoff_boundary_package_dir(workspace_current_dir);
    fs::create_dir_all(&package_dir)?;

    fs::copy(
        default_downstream_release_receipt_path(downstream_release_workspace_current_dir),
        package_dir.join("downstream_release_receipt.json"),
    )?;
    fs::copy(
        default_release_readiness_receipt_path(readiness_workspace_current_dir),
        package_dir.join("release_readiness_receipt.json"),
    )?;
    fs::copy(
        default_operator_release_summary_path(readiness_workspace_current_dir),
        package_dir.join("operator_release_summary.json"),
    )?;

    let release_package_dir =
        default_downstream_release_package_dir(downstream_release_workspace_current_dir);
    fs::copy(
        release_package_dir.join("consumer_acknowledgment_receipt.json"),
        package_dir.join("consumer_acknowledgment_receipt.json"),
    )?;
    fs::copy(
        release_package_dir.join("return_channel_closure_receipt.json"),
        package_dir.join("return_channel_closure_receipt.json"),
    )?;

    write_release_attestation_receipt(
        &default_release_attestation_receipt_path(workspace_current_dir),
        receipt,
    )?;
    Ok(())
}

pub fn write_release_attestation_receipt(
    path: &Path,
    receipt: &ReleaseAttestationReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_release_attestation_receipt(
    path: &Path,
) -> Result<ReleaseAttestationReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: ReleaseAttestationReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
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

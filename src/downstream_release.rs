use crate::consumer_acknowledgment::{
    build_return_channel_closure_receipt, default_consumer_acknowledgment_receipt_path,
    default_return_channel_closure_receipt_path, load_consumer_acknowledgment_receipt,
    load_return_channel_closure_receipt, validate_bounded_handoff_package,
};
use crate::consumer_handoff::{
    default_bounded_consumer_handoff_package_dir, default_bounded_consumer_handoff_receipt_path,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const DOWNSTREAM_RELEASE_RECEIPT_SCHEMA_VERSION: &str = "proof.downstream-release-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownstreamReleaseReceipt {
    pub schema_version: String,
    pub handoff_receipt_sha256: String,
    pub acknowledgment_receipt_sha256: String,
    pub closure_receipt_sha256: String,
    pub release_member_count: usize,
    pub release_eligible: bool,
}

pub fn default_downstream_release_handoff_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice30_consumer_handoff/current")
}

pub fn default_downstream_release_ack_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice31_consumer_acknowledgment/current")
}

pub fn default_downstream_release_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice32_downstream_release/current")
}

pub fn default_downstream_release_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("downstream_release_receipt.json")
}

pub fn default_downstream_release_package_dir(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("release_package")
}

pub fn build_downstream_release_receipt(
    handoff_workspace_current_dir: &Path,
    acknowledgment_workspace_current_dir: &Path,
) -> Result<DownstreamReleaseReceipt, Box<dyn Error>> {
    let handoff_receipt_path =
        default_bounded_consumer_handoff_receipt_path(handoff_workspace_current_dir);
    let package_dir = default_bounded_consumer_handoff_package_dir(handoff_workspace_current_dir);
    let acknowledgment_receipt_path =
        default_consumer_acknowledgment_receipt_path(acknowledgment_workspace_current_dir);
    let closure_receipt_path =
        default_return_channel_closure_receipt_path(acknowledgment_workspace_current_dir);

    if !closure_receipt_path.exists() {
        return Err(format!(
            "closure receipt missing: {}",
            closure_receipt_path.display()
        )
        .into());
    }

    validate_bounded_handoff_package(&package_dir, 4)?;

    let acknowledgment = load_consumer_acknowledgment_receipt(&acknowledgment_receipt_path)?;
    if !acknowledgment.acknowledged {
        return Err("acknowledgment receipt did not acknowledge handoff".into());
    }

    let closure = load_return_channel_closure_receipt(&closure_receipt_path)?;
    let expected_closure = build_return_channel_closure_receipt(
        handoff_workspace_current_dir,
        acknowledgment_workspace_current_dir,
    )?;
    if closure != expected_closure {
        return Err("closure receipt does not match reconstructed continuity".into());
    }
    if !closure.closed {
        return Err("closure receipt was not closed".into());
    }

    Ok(DownstreamReleaseReceipt {
        schema_version: DOWNSTREAM_RELEASE_RECEIPT_SCHEMA_VERSION.to_string(),
        handoff_receipt_sha256: sha256_hex_file(&handoff_receipt_path)?,
        acknowledgment_receipt_sha256: sha256_hex_file(&acknowledgment_receipt_path)?,
        closure_receipt_sha256: sha256_hex_file(&closure_receipt_path)?,
        release_member_count: 6,
        release_eligible: true,
    })
}

pub fn publish_downstream_release_package(
    handoff_workspace_current_dir: &Path,
    acknowledgment_workspace_current_dir: &Path,
    workspace_current_dir: &Path,
    receipt: &DownstreamReleaseReceipt,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(workspace_current_dir)?;
    let package_dir = default_downstream_release_package_dir(workspace_current_dir);
    fs::create_dir_all(&package_dir)?;

    let handoff_package_dir =
        default_bounded_consumer_handoff_package_dir(handoff_workspace_current_dir);
    for file_name in [
        "activation_receipt.json",
        "attestation_receipt.json",
        "consumer_contract.json",
        "supersession_chain_receipt.json",
    ] {
        fs::copy(
            handoff_package_dir.join(file_name),
            package_dir.join(file_name),
        )?;
    }
    fs::copy(
        default_consumer_acknowledgment_receipt_path(acknowledgment_workspace_current_dir),
        package_dir.join("consumer_acknowledgment_receipt.json"),
    )?;
    fs::copy(
        default_return_channel_closure_receipt_path(acknowledgment_workspace_current_dir),
        package_dir.join("return_channel_closure_receipt.json"),
    )?;

    write_downstream_release_receipt(
        &default_downstream_release_receipt_path(workspace_current_dir),
        receipt,
    )?;
    Ok(())
}

pub fn write_downstream_release_receipt(
    path: &Path,
    receipt: &DownstreamReleaseReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_downstream_release_receipt(
    path: &Path,
) -> Result<DownstreamReleaseReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: DownstreamReleaseReceipt = serde_json::from_slice(&bytes)?;
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

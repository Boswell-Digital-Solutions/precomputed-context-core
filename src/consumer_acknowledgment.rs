use crate::consumer_handoff::{
    default_bounded_consumer_handoff_package_dir, default_bounded_consumer_handoff_receipt_path,
    load_bounded_consumer_handoff_receipt,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const CONSUMER_ACKNOWLEDGMENT_RECEIPT_SCHEMA_VERSION: &str =
    "proof.consumer-acknowledgment-receipt.v1";
pub const RETURN_CHANNEL_CLOSURE_RECEIPT_SCHEMA_VERSION: &str =
    "proof.return-channel-closure-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsumerAcknowledgmentReceipt {
    pub schema_version: String,
    pub handoff_receipt_sha256: String,
    pub consumer_name: String,
    pub purpose: String,
    pub handoff_package_digest: String,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReturnChannelClosureReceipt {
    pub schema_version: String,
    pub handoff_receipt_sha256: String,
    pub acknowledgment_receipt_sha256: String,
    pub closed: bool,
}

pub fn default_consumer_acknowledgment_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice30_consumer_handoff/current")
}

pub fn default_consumer_acknowledgment_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice31_consumer_acknowledgment/current")
}

pub fn default_consumer_acknowledgment_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("consumer_acknowledgment_receipt.json")
}

pub fn default_return_channel_closure_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("return_channel_closure_receipt.json")
}

pub fn build_consumer_acknowledgment_receipt(
    handoff_workspace_current_dir: &Path,
) -> Result<ConsumerAcknowledgmentReceipt, Box<dyn Error>> {
    let handoff_receipt_path =
        default_bounded_consumer_handoff_receipt_path(handoff_workspace_current_dir);
    let package_dir = default_bounded_consumer_handoff_package_dir(handoff_workspace_current_dir);

    if !handoff_receipt_path.exists() {
        return Err(format!(
            "handoff receipt missing: {}",
            handoff_receipt_path.display()
        )
        .into());
    }

    let handoff_receipt = load_bounded_consumer_handoff_receipt(&handoff_receipt_path)?;
    if !handoff_receipt.readiness_approved {
        return Err("handoff receipt was not readiness-approved".into());
    }

    validate_bounded_handoff_package(&package_dir, handoff_receipt.handoff_member_count)?;

    Ok(ConsumerAcknowledgmentReceipt {
        schema_version: CONSUMER_ACKNOWLEDGMENT_RECEIPT_SCHEMA_VERSION.to_string(),
        handoff_receipt_sha256: sha256_hex_file(&handoff_receipt_path)?,
        consumer_name: handoff_receipt.consumer_name,
        purpose: handoff_receipt.purpose,
        handoff_package_digest: compute_handoff_package_digest(&package_dir)?,
        acknowledged: true,
    })
}

pub fn build_return_channel_closure_receipt(
    handoff_workspace_current_dir: &Path,
    acknowledgment_workspace_current_dir: &Path,
) -> Result<ReturnChannelClosureReceipt, Box<dyn Error>> {
    let handoff_receipt_path =
        default_bounded_consumer_handoff_receipt_path(handoff_workspace_current_dir);
    let acknowledgment_receipt_path =
        default_consumer_acknowledgment_receipt_path(acknowledgment_workspace_current_dir);

    let handoff_receipt_sha256 = sha256_hex_file(&handoff_receipt_path)?;
    let acknowledgment = load_consumer_acknowledgment_receipt(&acknowledgment_receipt_path)?;
    if !acknowledgment.acknowledged {
        return Err("consumer acknowledgment did not acknowledge receipt".into());
    }
    if acknowledgment.handoff_receipt_sha256 != handoff_receipt_sha256 {
        return Err("consumer acknowledgment does not match handoff receipt continuity".into());
    }

    Ok(ReturnChannelClosureReceipt {
        schema_version: RETURN_CHANNEL_CLOSURE_RECEIPT_SCHEMA_VERSION.to_string(),
        handoff_receipt_sha256,
        acknowledgment_receipt_sha256: sha256_hex_file(&acknowledgment_receipt_path)?,
        closed: true,
    })
}

pub fn publish_consumer_acknowledgment(
    workspace_current_dir: &Path,
    receipt: &ConsumerAcknowledgmentReceipt,
) -> Result<(), Box<dyn Error>> {
    write_consumer_acknowledgment_receipt(
        &default_consumer_acknowledgment_receipt_path(workspace_current_dir),
        receipt,
    )
}

pub fn publish_return_channel_closure(
    workspace_current_dir: &Path,
    receipt: &ReturnChannelClosureReceipt,
) -> Result<(), Box<dyn Error>> {
    write_return_channel_closure_receipt(
        &default_return_channel_closure_receipt_path(workspace_current_dir),
        receipt,
    )
}

pub fn write_consumer_acknowledgment_receipt(
    path: &Path,
    receipt: &ConsumerAcknowledgmentReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_consumer_acknowledgment_receipt(
    path: &Path,
) -> Result<ConsumerAcknowledgmentReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: ConsumerAcknowledgmentReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

pub fn write_return_channel_closure_receipt(
    path: &Path,
    receipt: &ReturnChannelClosureReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_return_channel_closure_receipt(
    path: &Path,
) -> Result<ReturnChannelClosureReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: ReturnChannelClosureReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

pub fn compute_handoff_package_digest(package_dir: &Path) -> Result<String, Box<dyn Error>> {
    let ordered_files = [
        "activation_receipt.json",
        "attestation_receipt.json",
        "consumer_contract.json",
        "supersession_chain_receipt.json",
    ];

    let mut joined = String::new();
    for file_name in ordered_files {
        let path = package_dir.join(file_name);
        let file_hash = sha256_hex_file(&path)?;
        joined.push_str(file_name);
        joined.push(':');
        joined.push_str(&file_hash);
        joined.push('\n');
    }

    let mut hasher = Sha256::new();
    hasher.update(joined.as_bytes());
    let digest = hasher.finalize();
    Ok(to_hex(&digest))
}

pub fn validate_bounded_handoff_package(
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
        "consumer_contract.json".to_string(),
        "supersession_chain_receipt.json".to_string(),
    ];
    if file_names != expected {
        return Err("bounded handoff package member set mismatch".into());
    }
    if file_names.len() != expected_member_count {
        return Err(format!(
            "bounded handoff package member count mismatch: expected {}, got {}",
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

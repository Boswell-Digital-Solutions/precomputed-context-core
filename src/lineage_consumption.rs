use crate::lineage_activation::{
    default_lineage_activation_receipt_path, load_lineage_activation_receipt,
};
use crate::supersession_chain::build_supersession_chain_receipt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const ACTIVE_LINEAGE_CONSUMER_CONTRACT_SCHEMA_VERSION: &str =
    "proof.active-lineage-consumer-contract.v1";
pub const ACTIVE_LINEAGE_ATTESTATION_RECEIPT_SCHEMA_VERSION: &str =
    "proof.active-lineage-attestation-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveLineageConsumerContract {
    pub schema_version: String,
    pub consumer_name: String,
    pub purpose: String,
    pub requires_admitted_lineage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveLineageAttestationReceipt {
    pub schema_version: String,
    pub activation_receipt_sha256: String,
    pub active_lineage_digest: String,
    pub consumer_name: String,
    pub purpose: String,
    pub attested: bool,
}

pub fn default_active_lineage_consumer_contract() -> ActiveLineageConsumerContract {
    ActiveLineageConsumerContract {
        schema_version: ACTIVE_LINEAGE_CONSUMER_CONTRACT_SCHEMA_VERSION.to_string(),
        consumer_name: "downstream-proof-consumer".to_string(),
        purpose: "governed-lineage-consumption".to_string(),
        requires_admitted_lineage: true,
    }
}

pub fn default_lineage_consumption_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice28_lineage_activation/current")
}

pub fn default_lineage_consumption_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice29_lineage_consumption/current")
}

pub fn default_active_lineage_consumer_contract_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("consumer_contract.json")
}

pub fn default_active_lineage_attestation_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("attestation_receipt.json")
}

pub fn validate_active_lineage_consumer_contract(
    contract: &ActiveLineageConsumerContract,
) -> Result<(), Box<dyn Error>> {
    if contract.schema_version != ACTIVE_LINEAGE_CONSUMER_CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "invalid consumer contract schema version: {}",
            contract.schema_version
        )
        .into());
    }
    if contract.consumer_name.trim().is_empty() {
        return Err("consumer_name must not be empty".into());
    }
    if contract.purpose.trim().is_empty() {
        return Err("purpose must not be empty".into());
    }
    if !contract.requires_admitted_lineage {
        return Err("consumer contract must require admitted lineage".into());
    }
    Ok(())
}

pub fn build_active_lineage_attestation_receipt(
    active_lineage_workspace_current_dir: &Path,
    contract_path: &Path,
) -> Result<ActiveLineageAttestationReceipt, Box<dyn Error>> {
    let contract = load_active_lineage_consumer_contract(contract_path)?;
    validate_active_lineage_consumer_contract(&contract)?;

    let activation_receipt_path =
        default_lineage_activation_receipt_path(active_lineage_workspace_current_dir);
    if !activation_receipt_path.exists() {
        return Err(format!(
            "lineage activation receipt missing: {}",
            activation_receipt_path.display()
        )
        .into());
    }
    let activation_receipt = load_lineage_activation_receipt(&activation_receipt_path)?;
    if !activation_receipt.admitted {
        return Err("lineage activation receipt was not admitted".into());
    }

    let active_lineage_dir = active_lineage_workspace_current_dir.join("active_lineage");
    let canonical_supersession = build_supersession_chain_receipt(
        &active_lineage_dir.join("promotion_receipt.json"),
        &active_lineage_dir.join("rollback_receipt.json"),
        &active_lineage_dir.join("re_promotion_receipt.json"),
    )?;
    let stored_supersession_value: serde_json::Value = serde_json::from_slice(&fs::read(
        active_lineage_dir.join("supersession_chain_receipt.json"),
    )?)?;
    let canonical_supersession_value = serde_json::to_value(&canonical_supersession)?;
    if stored_supersession_value != canonical_supersession_value {
        return Err(
            "active lineage supersession chain receipt does not match canonical reconstruction"
                .into(),
        );
    }

    Ok(ActiveLineageAttestationReceipt {
        schema_version: ACTIVE_LINEAGE_ATTESTATION_RECEIPT_SCHEMA_VERSION.to_string(),
        activation_receipt_sha256: sha256_hex_file(&activation_receipt_path)?,
        active_lineage_digest: compute_active_lineage_digest(&active_lineage_dir)?,
        consumer_name: contract.consumer_name,
        purpose: contract.purpose,
        attested: true,
    })
}

pub fn publish_active_lineage_attestation(
    workspace_current_dir: &Path,
    contract: &ActiveLineageConsumerContract,
    receipt: &ActiveLineageAttestationReceipt,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(workspace_current_dir)?;
    write_active_lineage_consumer_contract(
        &default_active_lineage_consumer_contract_path(workspace_current_dir),
        contract,
    )?;
    write_active_lineage_attestation_receipt(
        &default_active_lineage_attestation_receipt_path(workspace_current_dir),
        receipt,
    )?;
    Ok(())
}

pub fn write_active_lineage_consumer_contract(
    path: &Path,
    contract: &ActiveLineageConsumerContract,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(contract)?)?;
    Ok(())
}

pub fn load_active_lineage_consumer_contract(
    path: &Path,
) -> Result<ActiveLineageConsumerContract, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let contract: ActiveLineageConsumerContract = serde_json::from_slice(&bytes)?;
    Ok(contract)
}

pub fn write_active_lineage_attestation_receipt(
    path: &Path,
    receipt: &ActiveLineageAttestationReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_active_lineage_attestation_receipt(
    path: &Path,
) -> Result<ActiveLineageAttestationReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: ActiveLineageAttestationReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

pub fn compute_active_lineage_digest(active_lineage_dir: &Path) -> Result<String, Box<dyn Error>> {
    let ordered_files = [
        "promotion_receipt.json",
        "rollback_receipt.json",
        "re_promotion_receipt.json",
        "supersession_chain_receipt.json",
    ];

    let mut joined = String::new();
    for file_name in ordered_files {
        let path = active_lineage_dir.join(file_name);
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

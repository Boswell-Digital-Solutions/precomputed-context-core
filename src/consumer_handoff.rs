use crate::lineage_activation::{
    default_lineage_activation_receipt_path, load_lineage_activation_receipt,
};
use crate::lineage_consumption::{
    build_active_lineage_attestation_receipt, default_active_lineage_attestation_receipt_path,
    default_active_lineage_consumer_contract_path, load_active_lineage_attestation_receipt,
    load_active_lineage_consumer_contract, validate_active_lineage_consumer_contract,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const BOUNDED_CONSUMER_HANDOFF_RECEIPT_SCHEMA_VERSION: &str =
    "proof.bounded-consumer-handoff-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoundedConsumerHandoffReceipt {
    pub schema_version: String,
    pub consumer_name: String,
    pub purpose: String,
    pub activation_receipt_sha256: String,
    pub active_lineage_digest: String,
    pub handoff_member_count: usize,
    pub readiness_approved: bool,
}

pub fn default_consumer_handoff_active_lineage_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice28_lineage_activation/current")
}

pub fn default_consumer_handoff_consumption_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice29_lineage_consumption/current")
}

pub fn default_consumer_handoff_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice30_consumer_handoff/current")
}

pub fn default_bounded_consumer_handoff_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("handoff_receipt.json")
}

pub fn default_bounded_consumer_handoff_package_dir(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("handoff_package")
}

pub fn build_bounded_consumer_handoff_receipt(
    active_lineage_workspace_current_dir: &Path,
    consumption_workspace_current_dir: &Path,
) -> Result<BoundedConsumerHandoffReceipt, Box<dyn Error>> {
    let contract_path =
        default_active_lineage_consumer_contract_path(consumption_workspace_current_dir);
    let attestation_path =
        default_active_lineage_attestation_receipt_path(consumption_workspace_current_dir);
    let activation_receipt_path =
        default_lineage_activation_receipt_path(active_lineage_workspace_current_dir);

    let contract = load_active_lineage_consumer_contract(&contract_path)?;
    validate_active_lineage_consumer_contract(&contract)?;

    if !attestation_path.exists() {
        return Err(format!(
            "attestation receipt missing: {}",
            attestation_path.display()
        )
        .into());
    }
    if !activation_receipt_path.exists() {
        return Err(format!(
            "activation receipt missing: {}",
            activation_receipt_path.display()
        )
        .into());
    }

    let activation_receipt = load_lineage_activation_receipt(&activation_receipt_path)?;
    if !activation_receipt.admitted {
        return Err("activation receipt was not admitted".into());
    }

    let stored_attestation = load_active_lineage_attestation_receipt(&attestation_path)?;
    if !stored_attestation.attested {
        return Err("attestation receipt was not attested".into());
    }

    let expected_attestation = build_active_lineage_attestation_receipt(
        active_lineage_workspace_current_dir,
        &contract_path,
    )?;
    if stored_attestation != expected_attestation {
        return Err("stored attestation receipt does not match readiness reconstruction".into());
    }

    Ok(BoundedConsumerHandoffReceipt {
        schema_version: BOUNDED_CONSUMER_HANDOFF_RECEIPT_SCHEMA_VERSION.to_string(),
        consumer_name: stored_attestation.consumer_name,
        purpose: stored_attestation.purpose,
        activation_receipt_sha256: stored_attestation.activation_receipt_sha256,
        active_lineage_digest: stored_attestation.active_lineage_digest,
        handoff_member_count: 4,
        readiness_approved: true,
    })
}

pub fn publish_bounded_consumer_handoff(
    active_lineage_workspace_current_dir: &Path,
    consumption_workspace_current_dir: &Path,
    workspace_current_dir: &Path,
    receipt: &BoundedConsumerHandoffReceipt,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(workspace_current_dir)?;
    let package_dir = default_bounded_consumer_handoff_package_dir(workspace_current_dir);
    fs::create_dir_all(&package_dir)?;

    fs::copy(
        default_active_lineage_consumer_contract_path(consumption_workspace_current_dir),
        package_dir.join("consumer_contract.json"),
    )?;
    fs::copy(
        default_active_lineage_attestation_receipt_path(consumption_workspace_current_dir),
        package_dir.join("attestation_receipt.json"),
    )?;
    fs::copy(
        default_lineage_activation_receipt_path(active_lineage_workspace_current_dir),
        package_dir.join("activation_receipt.json"),
    )?;
    fs::copy(
        active_lineage_workspace_current_dir.join("active_lineage/supersession_chain_receipt.json"),
        package_dir.join("supersession_chain_receipt.json"),
    )?;

    write_bounded_consumer_handoff_receipt(
        &default_bounded_consumer_handoff_receipt_path(workspace_current_dir),
        receipt,
    )?;
    Ok(())
}

pub fn write_bounded_consumer_handoff_receipt(
    path: &Path,
    receipt: &BoundedConsumerHandoffReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_bounded_consumer_handoff_receipt(
    path: &Path,
) -> Result<BoundedConsumerHandoffReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: BoundedConsumerHandoffReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

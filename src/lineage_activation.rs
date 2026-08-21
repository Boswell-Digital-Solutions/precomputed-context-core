use crate::lineage_bundle_rehydrate::{
    default_lineage_rehydrate_receipt_path, load_lineage_rehydrate_receipt,
};
use crate::supersession_chain::build_supersession_chain_receipt;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const LINEAGE_ACTIVATION_RECEIPT_SCHEMA_VERSION: &str = "proof.lineage-activation-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineageActivationReceipt {
    pub schema_version: String,
    pub rehydrate_receipt_sha256: String,
    pub supersession_chain_receipt_sha256: String,
    pub activated_lineage_status: String,
    pub admitted: bool,
}

pub fn default_lineage_activation_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice27_lineage_rehydrate/current")
}

pub fn default_lineage_activation_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice28_lineage_activation/current")
}

pub fn default_lineage_activation_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("activation_receipt.json")
}

pub fn default_active_lineage_dir(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("active_lineage")
}

pub fn build_lineage_activation_receipt(
    rehydrate_workspace_current_dir: &Path,
) -> Result<LineageActivationReceipt, Box<dyn Error>> {
    let rehydrate_receipt_path =
        default_lineage_rehydrate_receipt_path(rehydrate_workspace_current_dir);
    if !rehydrate_receipt_path.exists() {
        return Err(format!(
            "lineage rehydrate receipt missing: {}",
            rehydrate_receipt_path.display()
        )
        .into());
    }
    let rehydrate_receipt = load_lineage_rehydrate_receipt(&rehydrate_receipt_path)?;
    if !rehydrate_receipt.accepted {
        return Err("lineage rehydrate receipt was not accepted".into());
    }

    let lineage_dir = rehydrate_workspace_current_dir.join("lineage_state");
    let canonical_supersession = build_supersession_chain_receipt(
        &lineage_dir.join("promotion_receipt.json"),
        &lineage_dir.join("rollback_receipt.json"),
        &lineage_dir.join("re_promotion_receipt.json"),
    )?;
    let stored_supersession_value: serde_json::Value = serde_json::from_slice(&fs::read(
        lineage_dir.join("supersession_chain_receipt.json"),
    )?)?;
    let canonical_supersession_value = serde_json::to_value(&canonical_supersession)?;
    if stored_supersession_value != canonical_supersession_value {
        return Err(
            "rehydrated supersession chain receipt does not match canonical reconstruction".into(),
        );
    }

    Ok(LineageActivationReceipt {
        schema_version: LINEAGE_ACTIVATION_RECEIPT_SCHEMA_VERSION.to_string(),
        rehydrate_receipt_sha256: sha256_hex_file(&rehydrate_receipt_path)?,
        supersession_chain_receipt_sha256: sha256_hex_file(
            &lineage_dir.join("supersession_chain_receipt.json"),
        )?,
        activated_lineage_status: "admitted_active_lineage".to_string(),
        admitted: true,
    })
}

pub fn publish_activated_lineage_state(
    rehydrate_workspace_current_dir: &Path,
    workspace_current_dir: &Path,
    receipt: &LineageActivationReceipt,
) -> Result<(), Box<dyn Error>> {
    let _ = build_lineage_activation_receipt(rehydrate_workspace_current_dir)?;

    fs::create_dir_all(workspace_current_dir)?;
    let active_lineage_dir = default_active_lineage_dir(workspace_current_dir);
    if active_lineage_dir.exists() {
        fs::remove_dir_all(&active_lineage_dir)?;
    }
    fs::create_dir_all(&active_lineage_dir)?;

    let source_lineage_dir = rehydrate_workspace_current_dir.join("lineage_state");
    for file_name in [
        "promotion_receipt.json",
        "rollback_receipt.json",
        "re_promotion_receipt.json",
        "supersession_chain_receipt.json",
    ] {
        fs::copy(
            source_lineage_dir.join(file_name),
            active_lineage_dir.join(file_name),
        )?;
    }

    write_lineage_activation_receipt(
        &default_lineage_activation_receipt_path(workspace_current_dir),
        receipt,
    )?;
    Ok(())
}

pub fn write_lineage_activation_receipt(
    path: &Path,
    receipt: &LineageActivationReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_lineage_activation_receipt(
    path: &Path,
) -> Result<LineageActivationReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: LineageActivationReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

fn sha256_hex_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest as _;
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{:02x}", byte);
    }
    Ok(output)
}

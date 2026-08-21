use crate::lineage_bundle::{
    default_lineage_bundle_envelope_path, default_lineage_bundle_manifest_path,
    load_lineage_bundle_manifest, sha256_hex_file, verify_lineage_bundle,
};
use crate::lineage_bundle_intake::{
    default_intaken_bundle_dir, default_lineage_bundle_intake_receipt_path,
    load_lineage_bundle_intake_receipt,
};
use crate::supersession_chain::build_supersession_chain_receipt;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const LINEAGE_REHYDRATE_RECEIPT_SCHEMA_VERSION: &str = "proof.lineage-rehydrate-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineageRehydrateReceipt {
    pub schema_version: String,
    pub intake_receipt_sha256: String,
    pub manifest_sha256: String,
    pub envelope_sha256: String,
    pub rehydrated_lineage_state: String,
    pub accepted: bool,
}

pub fn default_lineage_rehydrate_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice26_lineage_bundle_intake/current")
}

pub fn default_lineage_rehydrate_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice27_lineage_rehydrate/current")
}

pub fn default_lineage_rehydrate_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("rehydrate_receipt.json")
}

pub fn default_rehydrated_lineage_dir(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("lineage_state")
}

pub fn build_lineage_rehydrate_receipt(
    intake_workspace_current_dir: &Path,
) -> Result<LineageRehydrateReceipt, Box<dyn Error>> {
    let intake_receipt_path =
        default_lineage_bundle_intake_receipt_path(intake_workspace_current_dir);
    if !intake_receipt_path.exists() {
        return Err(format!(
            "lineage intake receipt missing: {}",
            intake_receipt_path.display()
        )
        .into());
    }
    let intake_receipt = load_lineage_bundle_intake_receipt(&intake_receipt_path)?;
    if !intake_receipt.accepted {
        return Err("lineage intake receipt was not accepted".into());
    }

    let bundle_dir = default_intaken_bundle_dir(intake_workspace_current_dir);
    verify_lineage_bundle(&bundle_dir)?;

    let manifest_path = default_lineage_bundle_manifest_path(&bundle_dir);
    let envelope_path = default_lineage_bundle_envelope_path(&bundle_dir);
    let actual_manifest_sha256 = sha256_hex_file(&manifest_path)?;
    let actual_envelope_sha256 = sha256_hex_file(&envelope_path)?;

    if actual_manifest_sha256 != intake_receipt.manifest_sha256 {
        return Err(format!(
            "lineage intake manifest hash mismatch: expected={} actual={}",
            intake_receipt.manifest_sha256, actual_manifest_sha256
        )
        .into());
    }
    if actual_envelope_sha256 != intake_receipt.envelope_sha256 {
        return Err(format!(
            "lineage intake envelope hash mismatch: expected={} actual={}",
            intake_receipt.envelope_sha256, actual_envelope_sha256
        )
        .into());
    }

    let canonical_supersession = build_supersession_chain_receipt(
        &bundle_dir.join("promotion_receipt.json"),
        &bundle_dir.join("rollback_receipt.json"),
        &bundle_dir.join("re_promotion_receipt.json"),
    )?;
    let stored_supersession_value: serde_json::Value = serde_json::from_slice(&fs::read(
        bundle_dir.join("supersession_chain_receipt.json"),
    )?)?;
    let canonical_supersession_value = serde_json::to_value(&canonical_supersession)?;
    if stored_supersession_value != canonical_supersession_value {
        return Err(
            "stored supersession chain receipt does not match canonical reconstruction".into(),
        );
    }

    Ok(LineageRehydrateReceipt {
        schema_version: LINEAGE_REHYDRATE_RECEIPT_SCHEMA_VERSION.to_string(),
        intake_receipt_sha256: sha256_hex_file(&intake_receipt_path)?,
        manifest_sha256: actual_manifest_sha256,
        envelope_sha256: actual_envelope_sha256,
        rehydrated_lineage_state: "reconstructed_supersession_lineage".to_string(),
        accepted: true,
    })
}

pub fn publish_rehydrated_lineage_state(
    intake_workspace_current_dir: &Path,
    workspace_current_dir: &Path,
    receipt: &LineageRehydrateReceipt,
) -> Result<(), Box<dyn Error>> {
    let _ = build_lineage_rehydrate_receipt(intake_workspace_current_dir)?;

    fs::create_dir_all(workspace_current_dir)?;
    let lineage_dir = default_rehydrated_lineage_dir(workspace_current_dir);
    if lineage_dir.exists() {
        fs::remove_dir_all(&lineage_dir)?;
    }
    fs::create_dir_all(&lineage_dir)?;

    let bundle_dir = default_intaken_bundle_dir(intake_workspace_current_dir);
    let manifest =
        load_lineage_bundle_manifest(&default_lineage_bundle_manifest_path(&bundle_dir))?;
    for entry in &manifest.entries {
        fs::copy(
            bundle_dir.join(&entry.relative_path),
            lineage_dir.join(&entry.relative_path),
        )?;
    }

    write_lineage_rehydrate_receipt(
        &default_lineage_rehydrate_receipt_path(workspace_current_dir),
        receipt,
    )?;
    Ok(())
}

pub fn write_lineage_rehydrate_receipt(
    path: &Path,
    receipt: &LineageRehydrateReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_lineage_rehydrate_receipt(
    path: &Path,
) -> Result<LineageRehydrateReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: LineageRehydrateReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

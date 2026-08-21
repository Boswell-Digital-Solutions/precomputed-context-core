use crate::lineage_bundle::{
    default_lineage_bundle_envelope_path, default_lineage_bundle_manifest_path,
    default_lineage_bundle_workspace_current, load_lineage_bundle_manifest, sha256_hex_file,
    verify_lineage_bundle,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const LINEAGE_BUNDLE_INTAKE_RECEIPT_SCHEMA_VERSION: &str =
    "proof.lineage-bundle-intake-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineageBundleIntakeReceipt {
    pub schema_version: String,
    pub source_bundle_path: String,
    pub manifest_sha256: String,
    pub envelope_sha256: String,
    pub accepted: bool,
    pub decision: String,
}

pub fn default_lineage_bundle_intake_source_dir() -> PathBuf {
    default_lineage_bundle_workspace_current()
}

pub fn default_lineage_bundle_intake_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice26_lineage_bundle_intake/current")
}

pub fn default_lineage_bundle_intake_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("intake_receipt.json")
}

pub fn default_intaken_bundle_dir(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("bundle")
}

pub fn build_lineage_bundle_intake_receipt(
    source_bundle_dir: &Path,
) -> Result<LineageBundleIntakeReceipt, Box<dyn Error>> {
    verify_lineage_bundle(source_bundle_dir)?;

    let manifest_path = default_lineage_bundle_manifest_path(source_bundle_dir);
    let envelope_path = default_lineage_bundle_envelope_path(source_bundle_dir);

    Ok(LineageBundleIntakeReceipt {
        schema_version: LINEAGE_BUNDLE_INTAKE_RECEIPT_SCHEMA_VERSION.to_string(),
        source_bundle_path: source_bundle_dir.display().to_string(),
        manifest_sha256: sha256_hex_file(&manifest_path)?,
        envelope_sha256: sha256_hex_file(&envelope_path)?,
        accepted: true,
        decision: "accepted_lineage_bundle".to_string(),
    })
}

pub fn publish_intaken_lineage_bundle(
    source_bundle_dir: &Path,
    workspace_current_dir: &Path,
    receipt: &LineageBundleIntakeReceipt,
) -> Result<(), Box<dyn Error>> {
    verify_lineage_bundle(source_bundle_dir)?;

    fs::create_dir_all(workspace_current_dir)?;
    let bundle_dir = default_intaken_bundle_dir(workspace_current_dir);
    if bundle_dir.exists() {
        fs::remove_dir_all(&bundle_dir)?;
    }
    fs::create_dir_all(&bundle_dir)?;

    let manifest =
        load_lineage_bundle_manifest(&default_lineage_bundle_manifest_path(source_bundle_dir))?;
    for entry in &manifest.entries {
        let source_path = source_bundle_dir.join(&entry.relative_path);
        let dest_path = bundle_dir.join(&entry.relative_path);
        fs::copy(source_path, dest_path)?;
    }

    fs::copy(
        default_lineage_bundle_manifest_path(source_bundle_dir),
        bundle_dir.join("lineage_bundle_manifest.json"),
    )?;
    fs::copy(
        default_lineage_bundle_envelope_path(source_bundle_dir),
        bundle_dir.join("lineage_bundle_envelope.json"),
    )?;

    write_lineage_bundle_intake_receipt(
        &default_lineage_bundle_intake_receipt_path(workspace_current_dir),
        receipt,
    )?;
    Ok(())
}

pub fn write_lineage_bundle_intake_receipt(
    path: &Path,
    receipt: &LineageBundleIntakeReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_lineage_bundle_intake_receipt(
    path: &Path,
) -> Result<LineageBundleIntakeReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: LineageBundleIntakeReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

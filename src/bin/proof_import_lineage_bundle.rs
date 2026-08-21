use precomputed_context_core::lineage_bundle::{
    default_lineage_bundle_envelope_path, default_lineage_bundle_manifest_path,
    default_lineage_bundle_source_paths, default_lineage_bundle_workspace_current,
    publish_lineage_bundle, verify_lineage_bundle,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct LineageBundleReport {
    schema_version: String,
    manifest_file_name: String,
    manifest_sha256: String,
    envelope_file_name: String,
    envelope_sha256: String,
    repeated_manifest_sha256: String,
    stable_repeated_manifest: bool,
    extra_member_rejected: bool,
    manifest_hash_mismatch_rejected: bool,
    member_sha_mismatch_rejected: bool,
    no_success_receipt_on_extra_member: bool,
    no_success_receipt_on_manifest_hash_mismatch: bool,
    no_success_receipt_on_member_sha_mismatch: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let workspace_current = default_lineage_bundle_workspace_current();
    let sources = default_lineage_bundle_source_paths();

    reset_dir(&workspace_current)?;
    let _ = publish_lineage_bundle(&workspace_current, &sources)?;
    verify_lineage_bundle(&workspace_current)?;

    let manifest_path = default_lineage_bundle_manifest_path(&workspace_current);
    let envelope_path = default_lineage_bundle_envelope_path(&workspace_current);
    let manifest_sha256 = sha256_file(&manifest_path)?;
    let envelope_sha256 = sha256_file(&envelope_path)?;

    let _ = publish_lineage_bundle(&workspace_current, &sources)?;
    verify_lineage_bundle(&workspace_current)?;
    let repeated_manifest_sha256 = sha256_file(&manifest_path)?;

    let scenario_root = PathBuf::from("target/proof_artifacts/slice25_lineage_bundle/scenarios");
    reset_dir(&scenario_root)?;

    let (extra_member_rejected, no_success_receipt_on_extra_member) = {
        let scenario_dir = scenario_root.join("extra_member");
        reset_dir(&scenario_dir)?;
        let _ = publish_lineage_bundle(&scenario_dir, &sources)?;
        fs::write(scenario_dir.join("rogue.txt"), b"rogue")?;
        let result = verify_lineage_bundle(&scenario_dir);
        (
            result.is_err(),
            !scenario_dir.join("bundle_verified.txt").exists(),
        )
    };

    let (manifest_hash_mismatch_rejected, no_success_receipt_on_manifest_hash_mismatch) = {
        let scenario_dir = scenario_root.join("manifest_hash_mismatch");
        reset_dir(&scenario_dir)?;
        let _ = publish_lineage_bundle(&scenario_dir, &sources)?;
        let envelope_path = default_lineage_bundle_envelope_path(&scenario_dir);
        let mut envelope_value: serde_json::Value =
            serde_json::from_slice(&fs::read(&envelope_path)?)?;
        envelope_value["manifest_sha256"] = serde_json::Value::String("0".repeat(64));
        fs::write(&envelope_path, serde_json::to_vec_pretty(&envelope_value)?)?;
        let result = verify_lineage_bundle(&scenario_dir);
        (
            result.is_err(),
            !scenario_dir.join("bundle_verified.txt").exists(),
        )
    };

    let (member_sha_mismatch_rejected, no_success_receipt_on_member_sha_mismatch) = {
        let scenario_dir = scenario_root.join("member_sha_mismatch");
        reset_dir(&scenario_dir)?;
        let _ = publish_lineage_bundle(&scenario_dir, &sources)?;
        fs::write(scenario_dir.join("promotion_receipt.json"), b"tampered")?;
        let result = verify_lineage_bundle(&scenario_dir);
        (
            result.is_err(),
            !scenario_dir.join("bundle_verified.txt").exists(),
        )
    };

    let report = LineageBundleReport {
        schema_version: "proof.lineage-bundle-report.v1".to_string(),
        manifest_file_name: file_name_string(&manifest_path)?,
        manifest_sha256: manifest_sha256.clone(),
        envelope_file_name: file_name_string(&envelope_path)?,
        envelope_sha256,
        repeated_manifest_sha256: repeated_manifest_sha256.clone(),
        stable_repeated_manifest: manifest_sha256 == repeated_manifest_sha256,
        extra_member_rejected,
        manifest_hash_mismatch_rejected,
        member_sha_mismatch_rejected,
        no_success_receipt_on_extra_member,
        no_success_receipt_on_manifest_hash_mismatch,
        no_success_receipt_on_member_sha_mismatch,
    };

    let report_path =
        PathBuf::from("target/proof_artifacts/slice25_lineage_bundle/lineage_bundle_report.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

fn reset_dir(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn file_name_string(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .file_name()
        .ok_or("path has no file name")?
        .to_string_lossy()
        .to_string())
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn Error>> {
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

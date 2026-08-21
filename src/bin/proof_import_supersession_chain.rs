use precomputed_context_core::supersession_chain::{
    build_supersession_chain_receipt, default_slice21_promotion_receipt_path,
    default_slice22_rollback_receipt_path, default_slice23_repromotion_receipt_path,
    default_supersession_chain_receipt_path, default_supersession_workspace_current,
    load_supersession_chain_receipt, publish_supersession_chain,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct SupersessionReport {
    schema_version: String,
    chain_receipt_file_name: String,
    chain_receipt_sha256: String,
    repeated_chain_receipt_sha256: String,
    stable_repeated_chain_receipt: bool,
    missing_rollback_receipt_rejected: bool,
    repromotion_rollback_hash_mismatch_rejected: bool,
    missing_repromotion_receipt_rejected: bool,
    no_publication_on_missing_rollback_receipt: bool,
    no_publication_on_repromotion_rollback_hash_mismatch: bool,
    no_publication_on_missing_repromotion_receipt: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let promotion_receipt_path = default_slice21_promotion_receipt_path();
    let rollback_receipt_path = default_slice22_rollback_receipt_path();
    let repromotion_receipt_path = default_slice23_repromotion_receipt_path();
    let workspace_current = default_supersession_workspace_current();

    reset_dir(&workspace_current)?;
    let receipt = build_supersession_chain_receipt(
        &promotion_receipt_path,
        &rollback_receipt_path,
        &repromotion_receipt_path,
    )?;
    publish_supersession_chain(&workspace_current, &receipt)?;

    let receipt_path = default_supersession_chain_receipt_path(&workspace_current);
    let receipt_sha256 = sha256_file(&receipt_path)?;

    let repeated_receipt = build_supersession_chain_receipt(
        &promotion_receipt_path,
        &rollback_receipt_path,
        &repromotion_receipt_path,
    )?;
    publish_supersession_chain(&workspace_current, &repeated_receipt)?;
    let repeated_receipt_sha256 = sha256_file(&receipt_path)?;

    let scenario_root = PathBuf::from("target/proof_artifacts/slice24_supersession/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_rollback_receipt_rejected, no_publication_on_missing_rollback_receipt) = {
        let scenario_dir = scenario_root.join("missing_rollback_receipt");
        fs::create_dir_all(&scenario_dir)?;
        let result = build_supersession_chain_receipt(
            &promotion_receipt_path,
            &scenario_dir.join("rollback_receipt.json"),
            &repromotion_receipt_path,
        );
        let published = default_supersession_chain_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let (
        repromotion_rollback_hash_mismatch_rejected,
        no_publication_on_repromotion_rollback_hash_mismatch,
    ) = {
        let scenario_dir = scenario_root.join("repromotion_rollback_hash_mismatch");
        fs::create_dir_all(&scenario_dir)?;
        let scenario_repromotion_path = scenario_dir.join("re_promotion_receipt.json");
        let mut repromotion_value: serde_json::Value =
            serde_json::from_slice(&fs::read(&repromotion_receipt_path)?)?;
        repromotion_value["rollback_receipt_sha256"] = serde_json::Value::String("0".repeat(64));
        fs::write(
            &scenario_repromotion_path,
            serde_json::to_vec_pretty(&repromotion_value)?,
        )?;
        let result = build_supersession_chain_receipt(
            &promotion_receipt_path,
            &rollback_receipt_path,
            &scenario_repromotion_path,
        );
        let published = default_supersession_chain_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let (missing_repromotion_receipt_rejected, no_publication_on_missing_repromotion_receipt) = {
        let scenario_dir = scenario_root.join("missing_repromotion_receipt");
        fs::create_dir_all(&scenario_dir)?;
        let result = build_supersession_chain_receipt(
            &promotion_receipt_path,
            &rollback_receipt_path,
            &scenario_dir.join("re_promotion_receipt.json"),
        );
        let published = default_supersession_chain_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let report = SupersessionReport {
        schema_version: "proof.supersession-report.v1".to_string(),
        chain_receipt_file_name: file_name_string(&receipt_path)?,
        chain_receipt_sha256: receipt_sha256.clone(),
        repeated_chain_receipt_sha256: repeated_receipt_sha256.clone(),
        stable_repeated_chain_receipt: receipt_sha256 == repeated_receipt_sha256,
        missing_rollback_receipt_rejected,
        repromotion_rollback_hash_mismatch_rejected,
        missing_repromotion_receipt_rejected,
        no_publication_on_missing_rollback_receipt,
        no_publication_on_repromotion_rollback_hash_mismatch,
        no_publication_on_missing_repromotion_receipt,
    };

    let report_path =
        PathBuf::from("target/proof_artifacts/slice24_supersession/supersession_report.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_supersession_chain_receipt(&receipt_path)?;
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

use precomputed_context_core::re_promotion::{
    build_repromotion_approval, default_operator_id, default_repromoted_import_receipt_path,
    default_repromotion_approval_path, default_repromotion_receipt_path,
    default_repromotion_workspace_current, default_slice21_source_promoted_import_receipt_path,
    default_slice21_source_promotion_receipt_path, default_slice22_rollback_receipt_path,
    load_repromotion_receipt, publish_repromoted_import_receipt, validate_repromotion_approval,
    write_repromotion_approval,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct RePromotionReport {
    schema_version: String,
    operator_id: String,
    re_promotion_receipt_file_name: String,
    re_promotion_receipt_sha256: String,
    re_promoted_import_receipt_file_name: String,
    re_promoted_import_receipt_sha256: String,
    repeated_re_promotion_receipt_sha256: String,
    stable_repeated_re_promotion_receipt: bool,
    missing_approval_rejected: bool,
    rollback_hash_mismatch_rejected: bool,
    missing_source_promoted_import_receipt_rejected: bool,
    no_publication_on_missing_approval: bool,
    no_publication_on_rollback_hash_mismatch: bool,
    no_publication_on_missing_source_promoted_import_receipt: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let rollback_receipt_path = default_slice22_rollback_receipt_path();
    let source_promotion_receipt_path = default_slice21_source_promotion_receipt_path();
    let source_promoted_import_receipt_path = default_slice21_source_promoted_import_receipt_path();
    let approval_path = default_repromotion_approval_path();
    let workspace_current = default_repromotion_workspace_current();

    reset_dir(&workspace_current)?;
    let approval = build_repromotion_approval(
        &rollback_receipt_path,
        &source_promotion_receipt_path,
        &source_promoted_import_receipt_path,
    )?;
    write_repromotion_approval(&approval_path, &approval)?;

    let receipt = validate_repromotion_approval(
        &rollback_receipt_path,
        &source_promotion_receipt_path,
        &source_promoted_import_receipt_path,
        &approval_path,
    )?;
    publish_repromoted_import_receipt(
        &source_promoted_import_receipt_path,
        &workspace_current,
        &receipt,
    )?;

    let receipt_path = default_repromotion_receipt_path(&workspace_current);
    let promoted_path = default_repromoted_import_receipt_path(&workspace_current);
    let receipt_sha256 = sha256_file(&receipt_path)?;
    let promoted_sha256 = sha256_file(&promoted_path)?;

    let receipt_repeat = validate_repromotion_approval(
        &rollback_receipt_path,
        &source_promotion_receipt_path,
        &source_promoted_import_receipt_path,
        &approval_path,
    )?;
    publish_repromoted_import_receipt(
        &source_promoted_import_receipt_path,
        &workspace_current,
        &receipt_repeat,
    )?;
    let repeated_receipt_sha256 = sha256_file(&receipt_path)?;

    let scenario_root = PathBuf::from("target/proof_artifacts/slice23_repromotion/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_approval_rejected, no_publication_on_missing_approval) = {
        let scenario_dir = scenario_root.join("missing_approval");
        fs::create_dir_all(&scenario_dir)?;
        let result = validate_repromotion_approval(
            &rollback_receipt_path,
            &source_promotion_receipt_path,
            &source_promoted_import_receipt_path,
            &scenario_dir.join("operator_reapproval.json"),
        );
        let published = default_repromoted_import_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let (rollback_hash_mismatch_rejected, no_publication_on_rollback_hash_mismatch) = {
        let scenario_dir = scenario_root.join("rollback_hash_mismatch");
        fs::create_dir_all(&scenario_dir)?;
        let bad_approval_path = scenario_dir.join("operator_reapproval.json");
        let mut bad_approval = build_repromotion_approval(
            &rollback_receipt_path,
            &source_promotion_receipt_path,
            &source_promoted_import_receipt_path,
        )?;
        bad_approval.rollback_receipt_sha256 = "0".repeat(64);
        write_repromotion_approval(&bad_approval_path, &bad_approval)?;
        let result = validate_repromotion_approval(
            &rollback_receipt_path,
            &source_promotion_receipt_path,
            &source_promoted_import_receipt_path,
            &bad_approval_path,
        );
        let published = default_repromoted_import_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let (
        missing_source_promoted_import_receipt_rejected,
        no_publication_on_missing_source_promoted_import_receipt,
    ) = {
        let scenario_dir = scenario_root.join("missing_source_promoted_import_receipt");
        fs::create_dir_all(&scenario_dir)?;
        let scenario_approval_path = scenario_dir.join("operator_reapproval.json");
        let approval = build_repromotion_approval(
            &rollback_receipt_path,
            &source_promotion_receipt_path,
            &source_promoted_import_receipt_path,
        )?;
        write_repromotion_approval(&scenario_approval_path, &approval)?;
        let result = validate_repromotion_approval(
            &rollback_receipt_path,
            &source_promotion_receipt_path,
            &scenario_dir.join("import_receipt.json"),
            &scenario_approval_path,
        );
        let published = default_repromoted_import_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let report = RePromotionReport {
        schema_version: "proof.re-promotion-report.v1".to_string(),
        operator_id: default_operator_id().to_string(),
        re_promotion_receipt_file_name: file_name_string(&receipt_path)?,
        re_promotion_receipt_sha256: receipt_sha256.clone(),
        re_promoted_import_receipt_file_name: file_name_string(&promoted_path)?,
        re_promoted_import_receipt_sha256: promoted_sha256,
        repeated_re_promotion_receipt_sha256: repeated_receipt_sha256.clone(),
        stable_repeated_re_promotion_receipt: receipt_sha256 == repeated_receipt_sha256,
        missing_approval_rejected,
        rollback_hash_mismatch_rejected,
        missing_source_promoted_import_receipt_rejected,
        no_publication_on_missing_approval,
        no_publication_on_rollback_hash_mismatch,
        no_publication_on_missing_source_promoted_import_receipt,
    };

    let report_path =
        PathBuf::from("target/proof_artifacts/slice23_repromotion/repromotion_report.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_repromotion_receipt(&receipt_path)?;
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

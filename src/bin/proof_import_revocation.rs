use precomputed_context_core::promotion_revocation::{
    build_revocation_request, default_operator_id, default_revocation_request_path,
    default_revocation_workspace_current, default_rollback_receipt_path,
    default_slice21_promoted_import_receipt_path, default_slice21_promotion_receipt_path,
    load_rollback_receipt, publish_rollback_state, validate_revocation_request,
    write_revocation_request,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct RevocationReport {
    schema_version: String,
    operator_id: String,
    rollback_receipt_file_name: String,
    rollback_receipt_sha256: String,
    repeated_rollback_receipt_sha256: String,
    stable_repeated_rollback_receipt: bool,
    missing_request_rejected: bool,
    promotion_hash_mismatch_rejected: bool,
    missing_promoted_import_receipt_rejected: bool,
    no_promoted_import_publication_after_rollback: bool,
    no_publication_on_missing_request: bool,
    no_publication_on_promotion_hash_mismatch: bool,
    no_publication_on_missing_promoted_import_receipt: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let promotion_receipt_path = default_slice21_promotion_receipt_path();
    let promoted_import_receipt_path = default_slice21_promoted_import_receipt_path();
    let revocation_request_path = default_revocation_request_path();
    let workspace_current = default_revocation_workspace_current();

    reset_dir(&workspace_current)?;
    let request = build_revocation_request(&promotion_receipt_path, &promoted_import_receipt_path)?;
    write_revocation_request(&revocation_request_path, &request)?;

    let rollback_receipt = validate_revocation_request(
        &promotion_receipt_path,
        &promoted_import_receipt_path,
        &revocation_request_path,
    )?;
    publish_rollback_state(&workspace_current, &rollback_receipt)?;

    let rollback_receipt_path = default_rollback_receipt_path(&workspace_current);
    let rollback_receipt_sha256 = sha256_file(&rollback_receipt_path)?;

    let rollback_receipt_repeat = validate_revocation_request(
        &promotion_receipt_path,
        &promoted_import_receipt_path,
        &revocation_request_path,
    )?;
    publish_rollback_state(&workspace_current, &rollback_receipt_repeat)?;
    let repeated_rollback_receipt_sha256 = sha256_file(&rollback_receipt_path)?;

    let scenario_root = PathBuf::from("target/proof_artifacts/slice22_revocation/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_request_rejected, no_publication_on_missing_request) = {
        let scenario_dir = scenario_root.join("missing_request");
        fs::create_dir_all(&scenario_dir)?;
        let result = validate_revocation_request(
            &promotion_receipt_path,
            &promoted_import_receipt_path,
            &scenario_dir.join("operator_revocation.json"),
        );
        let published = scenario_dir.join("rollback_receipt.json").exists();
        (result.is_err(), !published)
    };

    let (promotion_hash_mismatch_rejected, no_publication_on_promotion_hash_mismatch) = {
        let scenario_dir = scenario_root.join("promotion_hash_mismatch");
        fs::create_dir_all(&scenario_dir)?;
        let bad_request_path = scenario_dir.join("operator_revocation.json");
        let mut bad_request =
            build_revocation_request(&promotion_receipt_path, &promoted_import_receipt_path)?;
        bad_request.promotion_receipt_sha256 = "0".repeat(64);
        write_revocation_request(&bad_request_path, &bad_request)?;
        let result = validate_revocation_request(
            &promotion_receipt_path,
            &promoted_import_receipt_path,
            &bad_request_path,
        );
        let published = scenario_dir.join("rollback_receipt.json").exists();
        (result.is_err(), !published)
    };

    let (
        missing_promoted_import_receipt_rejected,
        no_publication_on_missing_promoted_import_receipt,
    ) = {
        let scenario_dir = scenario_root.join("missing_promoted_import_receipt");
        fs::create_dir_all(&scenario_dir)?;
        let scenario_request_path = scenario_dir.join("operator_revocation.json");
        let request =
            build_revocation_request(&promotion_receipt_path, &promoted_import_receipt_path)?;
        write_revocation_request(&scenario_request_path, &request)?;
        let result = validate_revocation_request(
            &promotion_receipt_path,
            &scenario_dir.join("import_receipt.json"),
            &scenario_request_path,
        );
        let published = scenario_dir.join("rollback_receipt.json").exists();
        (result.is_err(), !published)
    };

    let report = RevocationReport {
        schema_version: "proof.revocation-report.v1".to_string(),
        operator_id: default_operator_id().to_string(),
        rollback_receipt_file_name: file_name_string(&rollback_receipt_path)?,
        rollback_receipt_sha256: rollback_receipt_sha256.clone(),
        repeated_rollback_receipt_sha256: repeated_rollback_receipt_sha256.clone(),
        stable_repeated_rollback_receipt: rollback_receipt_sha256
            == repeated_rollback_receipt_sha256,
        missing_request_rejected,
        promotion_hash_mismatch_rejected,
        missing_promoted_import_receipt_rejected,
        no_promoted_import_publication_after_rollback: !workspace_current
            .join("import_receipt.json")
            .exists(),
        no_publication_on_missing_request,
        no_publication_on_promotion_hash_mismatch,
        no_publication_on_missing_promoted_import_receipt,
    };

    let report_path =
        PathBuf::from("target/proof_artifacts/slice22_revocation/revocation_report.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_rollback_receipt(&rollback_receipt_path)?;
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

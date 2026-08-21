use precomputed_context_core::promotion_gate::{
    build_promotion_approval, default_operator_id, default_promoted_import_receipt_path,
    default_promotion_approval_path, default_promotion_receipt_path,
    default_promotion_workspace_current, default_slice20_gate_receipt_path,
    default_slice20_gated_import_receipt_path_from_workspace, load_promotion_receipt,
    publish_promoted_import_receipt, validate_promotion_approval, write_promotion_approval,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct PromotionReport {
    schema_version: String,
    operator_id: String,
    promotion_receipt_file_name: String,
    promotion_receipt_sha256: String,
    promoted_import_receipt_file_name: String,
    promoted_import_receipt_sha256: String,
    repeated_promotion_receipt_sha256: String,
    stable_repeated_promotion_receipt: bool,
    missing_approval_rejected: bool,
    gate_hash_mismatch_rejected: bool,
    missing_gated_import_receipt_rejected: bool,
    no_publication_on_missing_approval: bool,
    no_publication_on_gate_hash_mismatch: bool,
    no_publication_on_missing_gated_import_receipt: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let gate_receipt_path = default_slice20_gate_receipt_path();
    let gated_import_receipt_path = default_slice20_gated_import_receipt_path_from_workspace();
    let approval_path = default_promotion_approval_path();
    let workspace_current = default_promotion_workspace_current();

    reset_dir(&workspace_current)?;
    let approval = build_promotion_approval(&gate_receipt_path, &gated_import_receipt_path)?;
    write_promotion_approval(&approval_path, &approval)?;

    let promotion_receipt = validate_promotion_approval(
        &gate_receipt_path,
        &gated_import_receipt_path,
        &approval_path,
    )?;
    publish_promoted_import_receipt(
        &gated_import_receipt_path,
        &workspace_current,
        &promotion_receipt,
    )?;

    let promotion_receipt_path = default_promotion_receipt_path(&workspace_current);
    let promoted_import_receipt_path = default_promoted_import_receipt_path(&workspace_current);
    let promotion_receipt_sha256 = sha256_file(&promotion_receipt_path)?;
    let promoted_import_receipt_sha256 = sha256_file(&promoted_import_receipt_path)?;

    let promotion_receipt_repeat = validate_promotion_approval(
        &gate_receipt_path,
        &gated_import_receipt_path,
        &approval_path,
    )?;
    publish_promoted_import_receipt(
        &gated_import_receipt_path,
        &workspace_current,
        &promotion_receipt_repeat,
    )?;
    let repeated_promotion_receipt_sha256 = sha256_file(&promotion_receipt_path)?;

    let scenario_root = PathBuf::from("target/proof_artifacts/slice21_promotion/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_approval_rejected, no_publication_on_missing_approval) = {
        let scenario_dir = scenario_root.join("missing_approval");
        fs::create_dir_all(&scenario_dir)?;
        let result = validate_promotion_approval(
            &gate_receipt_path,
            &gated_import_receipt_path,
            &scenario_dir.join("operator_approval.json"),
        );
        let published = default_promoted_import_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let (gate_hash_mismatch_rejected, no_publication_on_gate_hash_mismatch) = {
        let scenario_dir = scenario_root.join("gate_hash_mismatch");
        fs::create_dir_all(&scenario_dir)?;
        let bad_approval_path = scenario_dir.join("operator_approval.json");
        let mut bad_approval =
            build_promotion_approval(&gate_receipt_path, &gated_import_receipt_path)?;
        bad_approval.gate_receipt_sha256 = "0".repeat(64);
        write_promotion_approval(&bad_approval_path, &bad_approval)?;
        let result = validate_promotion_approval(
            &gate_receipt_path,
            &gated_import_receipt_path,
            &bad_approval_path,
        );
        let published = default_promoted_import_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let (missing_gated_import_receipt_rejected, no_publication_on_missing_gated_import_receipt) = {
        let scenario_dir = scenario_root.join("missing_gated_import_receipt");
        fs::create_dir_all(&scenario_dir)?;
        let scenario_approval_path = scenario_dir.join("operator_approval.json");
        let approval = build_promotion_approval(&gate_receipt_path, &gated_import_receipt_path)?;
        write_promotion_approval(&scenario_approval_path, &approval)?;
        let result = validate_promotion_approval(
            &gate_receipt_path,
            &scenario_dir.join("import_receipt.json"),
            &scenario_approval_path,
        );
        let published = default_promoted_import_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let report = PromotionReport {
        schema_version: "proof.promotion-report.v1".to_string(),
        operator_id: default_operator_id().to_string(),
        promotion_receipt_file_name: file_name_string(&promotion_receipt_path)?,
        promotion_receipt_sha256: promotion_receipt_sha256.clone(),
        promoted_import_receipt_file_name: file_name_string(&promoted_import_receipt_path)?,
        promoted_import_receipt_sha256,
        repeated_promotion_receipt_sha256: repeated_promotion_receipt_sha256.clone(),
        stable_repeated_promotion_receipt: promotion_receipt_sha256
            == repeated_promotion_receipt_sha256,
        missing_approval_rejected,
        gate_hash_mismatch_rejected,
        missing_gated_import_receipt_rejected,
        no_publication_on_missing_approval,
        no_publication_on_gate_hash_mismatch,
        no_publication_on_missing_gated_import_receipt,
    };

    let report_path =
        PathBuf::from("target/proof_artifacts/slice21_promotion/promotion_report.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_promotion_receipt(&promotion_receipt_path)?;
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

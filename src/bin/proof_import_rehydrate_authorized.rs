use precomputed_context_core::authorization_evidence::default_authorization_evidence_link_path;
use precomputed_context_core::import_gate::{
    default_gated_import_receipt_path, default_rehydrate_gate_receipt_path,
    default_rehydrate_gate_workspace_current, load_rehydrate_gate_receipt,
    publish_authorized_import_receipt, validate_rehydrate_gate,
};
use precomputed_context_core::import_policy::default_import_policy_path;
use precomputed_context_core::trust_envelope::default_trust_envelope_path;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct RehydrateGateReport {
    schema_version: String,
    gate_receipt_file_name: String,
    gate_receipt_sha256: String,
    gated_import_receipt_file_name: String,
    gated_import_receipt_sha256: String,
    repeated_gate_receipt_sha256: String,
    stable_repeated_gate_receipt: bool,
    missing_authorization_receipt_rejected: bool,
    evidence_hash_mismatch_rejected: bool,
    missing_import_receipt_rejected: bool,
    no_publication_on_missing_authorization: bool,
    no_publication_on_evidence_mismatch: bool,
    no_publication_on_missing_import_receipt: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let zip_path = PathBuf::from("target/proof_artifacts/slice14_export.zip");
    let policy_path = default_import_policy_path();
    let trust_envelope_path = default_trust_envelope_path(&zip_path);
    let authorization_receipt_path =
        PathBuf::from("target/proof_artifacts/slice19_policy/current/authorization_receipt.json");
    let evidence_link_path = default_authorization_evidence_link_path();
    let import_receipt_path =
        PathBuf::from("target/proof_artifacts/slice16_import/current/import_receipt.json");

    let workspace_current = default_rehydrate_gate_workspace_current();
    reset_dir(&workspace_current)?;

    let gate_receipt = validate_rehydrate_gate(
        &zip_path,
        &policy_path,
        &trust_envelope_path,
        &authorization_receipt_path,
        &evidence_link_path,
        &import_receipt_path,
    )?;
    publish_authorized_import_receipt(&import_receipt_path, &workspace_current, &gate_receipt)?;

    let gate_receipt_path = default_rehydrate_gate_receipt_path(&workspace_current);
    let gated_import_receipt_path = default_gated_import_receipt_path(&workspace_current);
    let gate_receipt_sha256 = sha256_file(&gate_receipt_path)?;
    let gated_import_receipt_sha256 = sha256_file(&gated_import_receipt_path)?;

    let gate_receipt_repeat = validate_rehydrate_gate(
        &zip_path,
        &policy_path,
        &trust_envelope_path,
        &authorization_receipt_path,
        &evidence_link_path,
        &import_receipt_path,
    )?;
    publish_authorized_import_receipt(
        &import_receipt_path,
        &workspace_current,
        &gate_receipt_repeat,
    )?;
    let repeated_gate_receipt_sha256 = sha256_file(&gate_receipt_path)?;

    let scenario_root = PathBuf::from("target/proof_artifacts/slice20_rehydrate_gate/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_authorization_receipt_rejected, no_publication_on_missing_authorization) = {
        let scenario_dir = scenario_root.join("missing_authorization_receipt");
        fs::create_dir_all(&scenario_dir)?;
        let result = validate_rehydrate_gate(
            &zip_path,
            &policy_path,
            &trust_envelope_path,
            &scenario_dir.join("authorization_receipt.json"),
            &evidence_link_path,
            &import_receipt_path,
        );
        let published = default_gated_import_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let (evidence_hash_mismatch_rejected, no_publication_on_evidence_mismatch) = {
        let scenario_dir = scenario_root.join("evidence_hash_mismatch");
        fs::create_dir_all(&scenario_dir)?;
        let bad_evidence_path = scenario_dir.join("authorization_evidence_link.json");
        let mut bytes = fs::read(&evidence_link_path)?;
        if let Some(last) = bytes.last_mut() {
            *last = if *last == b'0' { b'1' } else { b'0' };
        } else {
            bytes.push(b'0');
        }
        fs::write(&bad_evidence_path, bytes)?;
        let result = validate_rehydrate_gate(
            &zip_path,
            &policy_path,
            &trust_envelope_path,
            &authorization_receipt_path,
            &bad_evidence_path,
            &import_receipt_path,
        );
        let published = default_gated_import_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let (missing_import_receipt_rejected, no_publication_on_missing_import_receipt) = {
        let scenario_dir = scenario_root.join("missing_import_receipt");
        fs::create_dir_all(&scenario_dir)?;
        let result = validate_rehydrate_gate(
            &zip_path,
            &policy_path,
            &trust_envelope_path,
            &authorization_receipt_path,
            &evidence_link_path,
            &scenario_dir.join("import_receipt.json"),
        );
        let published = default_gated_import_receipt_path(&scenario_dir).exists();
        (result.is_err(), !published)
    };

    let report = RehydrateGateReport {
        schema_version: "proof.rehydrate-gate-report.v1".to_string(),
        gate_receipt_file_name: file_name_string(&gate_receipt_path)?,
        gate_receipt_sha256: gate_receipt_sha256.clone(),
        gated_import_receipt_file_name: file_name_string(&gated_import_receipt_path)?,
        gated_import_receipt_sha256,
        repeated_gate_receipt_sha256: repeated_gate_receipt_sha256.clone(),
        stable_repeated_gate_receipt: gate_receipt_sha256 == repeated_gate_receipt_sha256,
        missing_authorization_receipt_rejected,
        evidence_hash_mismatch_rejected,
        missing_import_receipt_rejected,
        no_publication_on_missing_authorization,
        no_publication_on_evidence_mismatch,
        no_publication_on_missing_import_receipt,
    };

    let report_path =
        PathBuf::from("target/proof_artifacts/slice20_rehydrate_gate/rehydrate_gate_report.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_rehydrate_gate_receipt(&gate_receipt_path)?;
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

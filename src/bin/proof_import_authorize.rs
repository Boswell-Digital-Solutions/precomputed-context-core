use precomputed_context_core::authorization_evidence::{
    build_authorization_evidence_link, default_authorization_evidence_link_path,
    default_import_receipt_path, write_authorization_evidence_link,
};
use precomputed_context_core::import_authorization::{
    authorize_zip_import_from_policy_file, default_authorization_receipt_path,
    write_authorization_receipt,
};
use precomputed_context_core::import_policy::{
    default_import_authorization_policy, load_import_authorization_policy,
    write_default_import_authorization_policy, write_import_authorization_policy,
};
use precomputed_context_core::trust_envelope::{
    build_signed_trust_envelope_for_zip, default_proof_signer, default_sha256_sidecar_path,
    default_trust_envelope_path, load_signed_trust_envelope, write_signed_trust_envelope,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct AuthorizationReport {
    schema_version: String,
    policy_file_name: String,
    policy_sha256: String,
    trust_envelope_file_name: String,
    authorization_receipt_file_name: String,
    authorization_receipt_sha256: String,
    evidence_link_file_name: String,
    evidence_link_sha256: String,
    repeated_authorization_receipt_sha256: String,
    stable_repeated_authorization_receipt: bool,
    missing_policy_rejected: bool,
    signer_removed_by_policy_rejected: bool,
    policy_scope_mismatch_rejected: bool,
    invalid_signature_rejected: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let zip_path = PathBuf::from("target/proof_artifacts/slice14_export.zip");
    let envelope_path = default_trust_envelope_path(&zip_path);
    if !envelope_path.exists() {
        let sha_path = default_sha256_sidecar_path(&zip_path);
        let envelope =
            build_signed_trust_envelope_for_zip(&zip_path, &sha_path, default_proof_signer())?;
        write_signed_trust_envelope(&envelope_path, &envelope)?;
    }

    let policy_path = write_default_import_authorization_policy()?;
    let policy_sha256 = sha256_file(&policy_path)?;

    let workspace_current = PathBuf::from("target/proof_artifacts/slice19_policy/current");
    fs::create_dir_all(&workspace_current)?;
    let receipt_path = default_authorization_receipt_path(&workspace_current);

    let receipt =
        authorize_zip_import_from_policy_file(&zip_path, Some(&envelope_path), &policy_path)?;
    write_authorization_receipt(&receipt_path, &receipt)?;
    let receipt_sha256 = sha256_file(&receipt_path)?;

    let receipt_repeat =
        authorize_zip_import_from_policy_file(&zip_path, Some(&envelope_path), &policy_path)?;
    write_authorization_receipt(&receipt_path, &receipt_repeat)?;
    let repeated_receipt_sha256 = sha256_file(&receipt_path)?;

    let import_receipt_path = default_import_receipt_path();
    let evidence = build_authorization_evidence_link(
        &policy_path,
        &envelope_path,
        &receipt_path,
        Some(&import_receipt_path),
    )?;
    let evidence_path = default_authorization_evidence_link_path();
    write_authorization_evidence_link(&evidence_path, &evidence)?;
    let evidence_sha256 = sha256_file(&evidence_path)?;

    let scenario_root = PathBuf::from("target/proof_artifacts/slice19_policy/scenarios");
    reset_dir(&scenario_root)?;

    let missing_policy_rejected = authorize_zip_import_from_policy_file(
        &zip_path,
        Some(&envelope_path),
        &scenario_root.join("missing_policy.json"),
    )
    .is_err();

    let signer_removed_by_policy_rejected = {
        let mut policy = default_import_authorization_policy();
        policy.signer_rules.clear();
        let policy_without_signer_path = scenario_root.join("policy_without_signer.json");
        write_import_authorization_policy(&policy_without_signer_path, &policy)?;
        authorize_zip_import_from_policy_file(
            &zip_path,
            Some(&envelope_path),
            &policy_without_signer_path,
        )
        .is_err()
    };

    let policy_scope_mismatch_rejected = {
        let mut policy = load_import_authorization_policy(&policy_path)?;
        policy.required_import_scope = "wrong_scope".to_string();
        let scope_mismatch_path = scenario_root.join("policy_scope_mismatch.json");
        write_import_authorization_policy(&scope_mismatch_path, &policy)?;
        authorize_zip_import_from_policy_file(&zip_path, Some(&envelope_path), &scope_mismatch_path)
            .is_err()
    };

    let invalid_signature_rejected = {
        let mut envelope = load_signed_trust_envelope(&envelope_path)?;
        envelope.signature = "0".repeat(64);
        let invalid_signature_path = scenario_root.join("invalid_signature.trust_envelope.json");
        write_signed_trust_envelope(&invalid_signature_path, &envelope)?;
        authorize_zip_import_from_policy_file(
            &zip_path,
            Some(&invalid_signature_path),
            &policy_path,
        )
        .is_err()
    };

    let report = AuthorizationReport {
        schema_version: "proof.import-authorization-report.v3".to_string(),
        policy_file_name: file_name_string(&policy_path)?,
        policy_sha256,
        trust_envelope_file_name: file_name_string(&envelope_path)?,
        authorization_receipt_file_name: file_name_string(&receipt_path)?,
        authorization_receipt_sha256: receipt_sha256.clone(),
        evidence_link_file_name: file_name_string(&evidence_path)?,
        evidence_link_sha256: evidence_sha256,
        repeated_authorization_receipt_sha256: repeated_receipt_sha256.clone(),
        stable_repeated_authorization_receipt: receipt_sha256 == repeated_receipt_sha256,
        missing_policy_rejected,
        signer_removed_by_policy_rejected,
        policy_scope_mismatch_rejected,
        invalid_signature_rejected,
    };

    let report_path =
        PathBuf::from("target/proof_artifacts/slice19_policy/authorization_report.json");
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

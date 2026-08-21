use crate::import_policy::{
    default_import_authorization_policy as default_import_authorization_policy_record,
    hash_import_authorization_policy_file, load_import_authorization_policy, signer_is_authorized,
    ImportAuthorizationPolicy,
};
use crate::trust_envelope::{
    default_trust_envelope_path, load_signed_trust_envelope, sha256_hex_file,
    verify_signed_trust_envelope, verify_zip_against_envelope,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const AUTHORIZATION_RECEIPT_SCHEMA_VERSION: &str = "proof.import-authorization-receipt.v2";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportAuthorizationReceipt {
    pub schema_version: String,
    pub policy_id: String,
    pub policy_sha256: String,
    pub repo_id: String,
    pub package_file_name: String,
    pub package_sha256: String,
    pub signer_id: String,
    pub envelope_file_name: String,
    pub trust_envelope_sha256: String,
    pub authorized: bool,
    pub decision: String,
}

pub fn default_import_authorization_policy() -> ImportAuthorizationPolicy {
    default_import_authorization_policy_record()
}

pub fn authorize_zip_import(
    zip_path: &Path,
    envelope_path: Option<&Path>,
    policy: &ImportAuthorizationPolicy,
) -> Result<ImportAuthorizationReceipt, Box<dyn Error>> {
    let resolved_envelope_path = envelope_path
        .map(|value| value.to_path_buf())
        .unwrap_or_else(|| default_trust_envelope_path(zip_path));

    if policy.require_trust_envelope && !resolved_envelope_path.exists() {
        return Err(format!(
            "required trust envelope is missing: {}",
            resolved_envelope_path.display()
        )
        .into());
    }

    let envelope = load_signed_trust_envelope(&resolved_envelope_path)?;
    verify_signed_trust_envelope(&envelope)?;
    verify_zip_against_envelope(zip_path, &envelope)?;

    if envelope.authorization_policy_id != policy.policy_id {
        return Err(format!(
            "authorization policy mismatch: expected={} actual={}",
            policy.policy_id, envelope.authorization_policy_id
        )
        .into());
    }

    if envelope.repo_id != policy.repo_id {
        return Err(format!(
            "authorization repo mismatch: expected={} actual={}",
            policy.repo_id, envelope.repo_id
        )
        .into());
    }

    if envelope.import_scope != policy.required_import_scope {
        return Err(format!(
            "authorization import scope mismatch: expected={} actual={}",
            policy.required_import_scope, envelope.import_scope
        )
        .into());
    }

    if !signer_is_authorized(policy, &envelope.signer_id) {
        return Err(format!(
            "signer is not authorized for import under current policy: signer_id={}",
            envelope.signer_id
        )
        .into());
    }

    let envelope_file_name = resolved_envelope_path
        .file_name()
        .ok_or("envelope path has no file name")?
        .to_string_lossy()
        .to_string();

    Ok(ImportAuthorizationReceipt {
        schema_version: AUTHORIZATION_RECEIPT_SCHEMA_VERSION.to_string(),
        policy_id: policy.policy_id.clone(),
        policy_sha256: String::new(),
        repo_id: policy.repo_id.clone(),
        package_file_name: envelope.package_file_name,
        package_sha256: envelope.package_sha256,
        signer_id: envelope.signer_id,
        envelope_file_name,
        trust_envelope_sha256: sha256_hex_file(&resolved_envelope_path)?,
        authorized: true,
        decision: "authorized".to_string(),
    })
}

pub fn authorize_zip_import_from_policy_file(
    zip_path: &Path,
    envelope_path: Option<&Path>,
    policy_path: &Path,
) -> Result<ImportAuthorizationReceipt, Box<dyn Error>> {
    let policy = load_import_authorization_policy(policy_path)?;
    let mut receipt = authorize_zip_import(zip_path, envelope_path, &policy)?;
    receipt.policy_sha256 = hash_import_authorization_policy_file(policy_path)?;
    Ok(receipt)
}

pub fn default_authorization_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("authorization_receipt.json")
}

pub fn write_authorization_receipt(
    path: &Path,
    receipt: &ImportAuthorizationReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(receipt)?;
    fs::write(path, bytes)?;
    Ok(())
}

pub fn load_authorization_receipt(
    path: &Path,
) -> Result<ImportAuthorizationReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: ImportAuthorizationReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

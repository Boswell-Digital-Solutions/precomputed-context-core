use crate::trust_envelope::{
    default_proof_signer, sha256_hex_file, TRUST_ENVELOPE_IMPORT_SCOPE, TRUST_ENVELOPE_POLICY_ID,
    TRUST_ENVELOPE_REPO_ID,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const IMPORT_POLICY_SCHEMA_VERSION: &str = "proof.import-authorization-policy.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignerAuthorizationRule {
    pub signer_id: String,
    pub allow_import: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportAuthorizationPolicy {
    pub schema_version: String,
    pub policy_id: String,
    pub repo_id: String,
    pub required_import_scope: String,
    pub require_trust_envelope: bool,
    pub require_sha256_sidecar: bool,
    pub signer_rules: Vec<SignerAuthorizationRule>,
}

pub fn default_import_authorization_policy() -> ImportAuthorizationPolicy {
    ImportAuthorizationPolicy {
        schema_version: IMPORT_POLICY_SCHEMA_VERSION.to_string(),
        policy_id: TRUST_ENVELOPE_POLICY_ID.to_string(),
        repo_id: TRUST_ENVELOPE_REPO_ID.to_string(),
        required_import_scope: TRUST_ENVELOPE_IMPORT_SCOPE.to_string(),
        require_trust_envelope: true,
        require_sha256_sidecar: true,
        signer_rules: vec![SignerAuthorizationRule {
            signer_id: default_proof_signer().signer_id.to_string(),
            allow_import: true,
        }],
    }
}

pub fn default_import_policy_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice19_policy/import_authorization_policy.json")
}

pub fn load_import_authorization_policy(
    path: &Path,
) -> Result<ImportAuthorizationPolicy, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let policy: ImportAuthorizationPolicy = serde_json::from_slice(&bytes)?;
    Ok(policy)
}

pub fn write_import_authorization_policy(
    path: &Path,
    policy: &ImportAuthorizationPolicy,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(policy)?;
    fs::write(path, bytes)?;
    Ok(())
}

pub fn write_default_import_authorization_policy() -> Result<PathBuf, Box<dyn Error>> {
    let path = default_import_policy_path();
    let policy = default_import_authorization_policy();
    write_import_authorization_policy(&path, &policy)?;
    Ok(path)
}

pub fn hash_import_authorization_policy_file(path: &Path) -> Result<String, Box<dyn Error>> {
    sha256_hex_file(path)
}

pub fn signer_is_authorized(policy: &ImportAuthorizationPolicy, signer_id: &str) -> bool {
    policy
        .signer_rules
        .iter()
        .any(|rule| rule.signer_id == signer_id && rule.allow_import)
}

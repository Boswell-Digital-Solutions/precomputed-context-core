use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const TRUST_ENVELOPE_SCHEMA_VERSION: &str = "proof.trust-envelope.v1";
pub const TRUST_ENVELOPE_IMPORT_SCOPE: &str = "proof_import";
pub const TRUST_ENVELOPE_POLICY_ID: &str = "slice18_import_authorization_v1";
pub const TRUST_ENVELOPE_REPO_ID: &str = "precomputed-context-core";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofSignerProfile {
    pub signer_id: &'static str,
    pub signing_secret: &'static str,
    pub trusted_for_import: bool,
}

pub const TRUSTED_PROOF_SIGNER: ProofSignerProfile = ProofSignerProfile {
    signer_id: "slice18-proof-signer-v1",
    signing_secret: "slice18-proof-signer-secret-v1",
    trusted_for_import: true,
};

pub const ROGUE_PROOF_SIGNER: ProofSignerProfile = ProofSignerProfile {
    signer_id: "slice18-rogue-signer-v1",
    signing_secret: "slice18-rogue-signer-secret-v1",
    trusted_for_import: false,
};

pub fn default_proof_signer() -> ProofSignerProfile {
    TRUSTED_PROOF_SIGNER
}

pub fn rogue_proof_signer() -> ProofSignerProfile {
    ROGUE_PROOF_SIGNER
}

pub fn known_proof_signer_by_id(signer_id: &str) -> Option<ProofSignerProfile> {
    if signer_id == TRUSTED_PROOF_SIGNER.signer_id {
        Some(TRUSTED_PROOF_SIGNER)
    } else if signer_id == ROGUE_PROOF_SIGNER.signer_id {
        Some(ROGUE_PROOF_SIGNER)
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedTrustEnvelope {
    pub schema_version: String,
    pub repo_id: String,
    pub authorization_policy_id: String,
    pub import_scope: String,
    pub signer_id: String,
    pub package_file_name: String,
    pub package_sha256: String,
    pub sha256_file_name: String,
    pub signature: String,
}

impl SignedTrustEnvelope {
    pub fn canonical_payload(&self) -> String {
        [
            format!("schema_version={}", self.schema_version),
            format!("repo_id={}", self.repo_id),
            format!("authorization_policy_id={}", self.authorization_policy_id),
            format!("import_scope={}", self.import_scope),
            format!("signer_id={}", self.signer_id),
            format!("package_file_name={}", self.package_file_name),
            format!("package_sha256={}", self.package_sha256),
            format!("sha256_file_name={}", self.sha256_file_name),
        ]
        .join("\n")
    }
}

pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    to_hex(&digest)
}

pub fn sha256_hex_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(sha256_hex_bytes(&bytes))
}

pub fn sign_payload_for_signer(payload: &str, signer: ProofSignerProfile) -> String {
    let signing_material = format!("{}\n{}", signer.signing_secret, payload);
    sha256_hex_bytes(signing_material.as_bytes())
}

pub fn build_signed_trust_envelope_for_zip(
    zip_path: &Path,
    sha256_path: &Path,
    signer: ProofSignerProfile,
) -> Result<SignedTrustEnvelope, Box<dyn Error>> {
    let package_file_name = zip_path
        .file_name()
        .ok_or("zip path has no file name")?
        .to_string_lossy()
        .to_string();
    let sha256_file_name = sha256_path
        .file_name()
        .ok_or("sha256 path has no file name")?
        .to_string_lossy()
        .to_string();

    let computed_package_sha256 = sha256_hex_file(zip_path)?;
    let declared_package_sha256 = read_sha256_sidecar(sha256_path)?;
    if computed_package_sha256 != declared_package_sha256 {
        return Err(format!(
            "zip sha mismatch while building trust envelope: computed={} declared={}",
            computed_package_sha256, declared_package_sha256
        )
        .into());
    }

    let mut envelope = SignedTrustEnvelope {
        schema_version: TRUST_ENVELOPE_SCHEMA_VERSION.to_string(),
        repo_id: TRUST_ENVELOPE_REPO_ID.to_string(),
        authorization_policy_id: TRUST_ENVELOPE_POLICY_ID.to_string(),
        import_scope: TRUST_ENVELOPE_IMPORT_SCOPE.to_string(),
        signer_id: signer.signer_id.to_string(),
        package_file_name,
        package_sha256: computed_package_sha256,
        sha256_file_name,
        signature: String::new(),
    };

    envelope.signature = sign_payload_for_signer(&envelope.canonical_payload(), signer);
    Ok(envelope)
}

pub fn verify_signed_trust_envelope(envelope: &SignedTrustEnvelope) -> Result<(), Box<dyn Error>> {
    let signer = known_proof_signer_by_id(&envelope.signer_id).ok_or_else(|| {
        format!(
            "unknown signer_id in trust envelope: {}",
            envelope.signer_id
        )
    })?;

    let expected_signature = sign_payload_for_signer(&envelope.canonical_payload(), signer);
    if envelope.signature != expected_signature {
        return Err(format!(
            "trust envelope signature mismatch: expected={} actual={}",
            expected_signature, envelope.signature
        )
        .into());
    }

    Ok(())
}

pub fn write_signed_trust_envelope(
    path: &Path,
    envelope: &SignedTrustEnvelope,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(envelope)?;
    fs::write(path, bytes)?;
    Ok(())
}

pub fn load_signed_trust_envelope(path: &Path) -> Result<SignedTrustEnvelope, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let envelope: SignedTrustEnvelope = serde_json::from_slice(&bytes)?;
    Ok(envelope)
}

pub fn write_default_trust_envelope_for_zip(zip_path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let sha256_path = default_sha256_sidecar_path(zip_path);
    let envelope =
        build_signed_trust_envelope_for_zip(zip_path, &sha256_path, default_proof_signer())?;
    let envelope_path = default_trust_envelope_path(zip_path);
    write_signed_trust_envelope(&envelope_path, &envelope)?;
    Ok(envelope_path)
}

pub fn default_sha256_sidecar_path(zip_path: &Path) -> PathBuf {
    let file_name = zip_path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "package.zip".to_string());
    zip_path.with_file_name(format!("{}.sha256", file_name))
}

pub fn default_trust_envelope_path(zip_path: &Path) -> PathBuf {
    let file_name = zip_path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "package.zip".to_string());
    zip_path.with_file_name(format!("{}.trust_envelope.json", file_name))
}

pub fn read_sha256_sidecar(path: &Path) -> Result<String, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;
    let declared = raw
        .split_whitespace()
        .next()
        .ok_or("sha256 sidecar is empty")?
        .trim()
        .to_string();
    Ok(declared)
}

pub fn verify_zip_against_envelope(
    zip_path: &Path,
    envelope: &SignedTrustEnvelope,
) -> Result<(), Box<dyn Error>> {
    let actual_package_file_name = zip_path
        .file_name()
        .ok_or("zip path has no file name")?
        .to_string_lossy()
        .to_string();
    if actual_package_file_name != envelope.package_file_name {
        return Err(format!(
            "trust envelope package file mismatch: expected={} actual={}",
            envelope.package_file_name, actual_package_file_name
        )
        .into());
    }

    let actual_package_sha256 = sha256_hex_file(zip_path)?;
    if actual_package_sha256 != envelope.package_sha256 {
        return Err(format!(
            "trust envelope package sha mismatch: expected={} actual={}",
            envelope.package_sha256, actual_package_sha256
        )
        .into());
    }

    Ok(())
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{:02x}", byte);
    }
    output
}

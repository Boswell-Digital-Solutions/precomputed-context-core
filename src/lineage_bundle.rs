use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const LINEAGE_BUNDLE_MANIFEST_SCHEMA_VERSION: &str = "proof.lineage-bundle-manifest.v1";
pub const LINEAGE_BUNDLE_ENVELOPE_SCHEMA_VERSION: &str = "proof.lineage-bundle-envelope.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineageBundleManifestEntry {
    pub logical_name: String,
    pub relative_path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineageBundleManifest {
    pub schema_version: String,
    pub bundle_name: String,
    pub entries: Vec<LineageBundleManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineageBundleEnvelope {
    pub schema_version: String,
    pub signer_id: String,
    pub manifest_sha256: String,
    pub signature: String,
}

#[derive(Debug, Clone)]
pub struct LineageBundleSourcePaths {
    pub promotion_receipt_path: PathBuf,
    pub rollback_receipt_path: PathBuf,
    pub repromotion_receipt_path: PathBuf,
    pub supersession_chain_receipt_path: PathBuf,
}

pub fn default_lineage_bundle_signer_id() -> &'static str {
    "trusted-proof-signer"
}

pub fn default_slice21_promotion_receipt_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice21_promotion/current/promotion_receipt.json")
}

pub fn default_slice22_rollback_receipt_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice22_revocation/current/rollback_receipt.json")
}

pub fn default_slice23_repromotion_receipt_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice23_repromotion/current/re_promotion_receipt.json")
}

pub fn default_slice24_supersession_chain_receipt_path() -> PathBuf {
    PathBuf::from(
        "target/proof_artifacts/slice24_supersession/current/supersession_chain_receipt.json",
    )
}

pub fn default_lineage_bundle_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice25_lineage_bundle/current")
}

pub fn default_lineage_bundle_manifest_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("lineage_bundle_manifest.json")
}

pub fn default_lineage_bundle_envelope_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("lineage_bundle_envelope.json")
}

pub fn default_lineage_bundle_source_paths() -> LineageBundleSourcePaths {
    LineageBundleSourcePaths {
        promotion_receipt_path: default_slice21_promotion_receipt_path(),
        rollback_receipt_path: default_slice22_rollback_receipt_path(),
        repromotion_receipt_path: default_slice23_repromotion_receipt_path(),
        supersession_chain_receipt_path: default_slice24_supersession_chain_receipt_path(),
    }
}

pub fn publish_lineage_bundle(
    workspace_current_dir: &Path,
    sources: &LineageBundleSourcePaths,
) -> Result<(LineageBundleManifest, LineageBundleEnvelope), Box<dyn Error>> {
    fs::create_dir_all(workspace_current_dir)?;

    let source_map = vec![
        (
            "promotion_receipt",
            "promotion_receipt.json",
            &sources.promotion_receipt_path,
        ),
        (
            "rollback_receipt",
            "rollback_receipt.json",
            &sources.rollback_receipt_path,
        ),
        (
            "re_promotion_receipt",
            "re_promotion_receipt.json",
            &sources.repromotion_receipt_path,
        ),
        (
            "supersession_chain_receipt",
            "supersession_chain_receipt.json",
            &sources.supersession_chain_receipt_path,
        ),
    ];

    let mut entries = Vec::new();
    for (logical_name, file_name, source_path) in source_map {
        if !source_path.exists() {
            return Err(format!("lineage source missing: {}", source_path.display()).into());
        }
        let dest_path = workspace_current_dir.join(file_name);
        fs::copy(source_path, &dest_path)?;
        entries.push(LineageBundleManifestEntry {
            logical_name: logical_name.to_string(),
            relative_path: file_name.to_string(),
            sha256: sha256_hex_file(&dest_path)?,
        });
    }

    let manifest = LineageBundleManifest {
        schema_version: LINEAGE_BUNDLE_MANIFEST_SCHEMA_VERSION.to_string(),
        bundle_name: "precomputed-context-core-lineage-bundle".to_string(),
        entries,
    };
    let manifest_path = default_lineage_bundle_manifest_path(workspace_current_dir);
    write_lineage_bundle_manifest(&manifest_path, &manifest)?;

    let envelope =
        build_lineage_bundle_envelope(&manifest_path, default_lineage_bundle_signer_id())?;
    let envelope_path = default_lineage_bundle_envelope_path(workspace_current_dir);
    write_lineage_bundle_envelope(&envelope_path, &envelope)?;

    Ok((manifest, envelope))
}

pub fn verify_lineage_bundle(workspace_current_dir: &Path) -> Result<(), Box<dyn Error>> {
    let manifest_path = default_lineage_bundle_manifest_path(workspace_current_dir);
    let envelope_path = default_lineage_bundle_envelope_path(workspace_current_dir);

    if !manifest_path.exists() {
        return Err(format!("lineage manifest missing: {}", manifest_path.display()).into());
    }
    if !envelope_path.exists() {
        return Err(format!("lineage envelope missing: {}", envelope_path.display()).into());
    }

    let manifest = load_lineage_bundle_manifest(&manifest_path)?;
    let envelope = load_lineage_bundle_envelope(&envelope_path)?;

    let expected_member_set: BTreeSet<String> = [
        "promotion_receipt.json",
        "rollback_receipt.json",
        "re_promotion_receipt.json",
        "supersession_chain_receipt.json",
        "lineage_bundle_manifest.json",
        "lineage_bundle_envelope.json",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect();

    let actual_member_set: BTreeSet<String> = fs::read_dir(workspace_current_dir)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    if actual_member_set != expected_member_set {
        return Err(format!(
            "lineage bundle member set mismatch: expected={:?} actual={:?}",
            expected_member_set, actual_member_set
        )
        .into());
    }

    for entry in &manifest.entries {
        let member_path = workspace_current_dir.join(&entry.relative_path);
        if !member_path.exists() {
            return Err(format!("lineage bundle member missing: {}", entry.relative_path).into());
        }
        let actual_sha256 = sha256_hex_file(&member_path)?;
        if actual_sha256 != entry.sha256 {
            return Err(format!(
                "lineage bundle member sha mismatch for {}: expected={} actual={}",
                entry.relative_path, entry.sha256, actual_sha256
            )
            .into());
        }
    }

    let actual_manifest_sha256 = sha256_hex_file(&manifest_path)?;
    if envelope.manifest_sha256 != actual_manifest_sha256 {
        return Err(format!(
            "lineage bundle envelope manifest hash mismatch: expected={} actual={}",
            actual_manifest_sha256, envelope.manifest_sha256
        )
        .into());
    }

    let expected_signature =
        compute_lineage_bundle_signature(&envelope.signer_id, &envelope.manifest_sha256);
    if envelope.signature != expected_signature {
        return Err("lineage bundle envelope signature mismatch".into());
    }

    Ok(())
}

pub fn build_lineage_bundle_envelope(
    manifest_path: &Path,
    signer_id: &str,
) -> Result<LineageBundleEnvelope, Box<dyn Error>> {
    let manifest_sha256 = sha256_hex_file(manifest_path)?;
    Ok(LineageBundleEnvelope {
        schema_version: LINEAGE_BUNDLE_ENVELOPE_SCHEMA_VERSION.to_string(),
        signer_id: signer_id.to_string(),
        manifest_sha256: manifest_sha256.clone(),
        signature: compute_lineage_bundle_signature(signer_id, &manifest_sha256),
    })
}

pub fn write_lineage_bundle_manifest(
    path: &Path,
    manifest: &LineageBundleManifest,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(manifest)?)?;
    Ok(())
}

pub fn load_lineage_bundle_manifest(path: &Path) -> Result<LineageBundleManifest, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let manifest: LineageBundleManifest = serde_json::from_slice(&bytes)?;
    Ok(manifest)
}

pub fn write_lineage_bundle_envelope(
    path: &Path,
    envelope: &LineageBundleEnvelope,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(envelope)?)?;
    Ok(())
}

pub fn load_lineage_bundle_envelope(path: &Path) -> Result<LineageBundleEnvelope, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let envelope: LineageBundleEnvelope = serde_json::from_slice(&bytes)?;
    Ok(envelope)
}

pub fn sha256_hex_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    Ok(to_hex(&digest))
}

pub fn compute_lineage_bundle_signature(signer_id: &str, manifest_sha256: &str) -> String {
    let payload = format!(
        "lineage-bundle-signature:v1:{}:{}",
        signer_id, manifest_sha256
    );
    sha256_hex_bytes(payload.as_bytes())
}

fn sha256_hex_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    to_hex(&digest)
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{:02x}", byte);
    }
    output
}

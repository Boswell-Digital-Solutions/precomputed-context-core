use crate::release_attestation::{
    default_handoff_boundary_package_dir, default_release_attestation_receipt_path,
    load_release_attestation_receipt,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const SEALED_RELEASE_RECEIPT_SCHEMA_VERSION: &str = "proof.sealed-release-receipt.v1";
pub const TERMINAL_BOUNDARY_MANIFEST_SCHEMA_VERSION: &str = "proof.terminal-boundary-manifest.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalBoundaryManifestEntry {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalBoundaryManifest {
    pub schema_version: String,
    pub entries: Vec<TerminalBoundaryManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealedReleaseReceipt {
    pub schema_version: String,
    pub release_attestation_receipt_sha256: String,
    pub terminal_boundary_manifest_sha256: String,
    pub sealed_member_count: usize,
    pub sealed_for_terminal_boundary: bool,
}

pub fn default_sealed_release_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice34_release_attestation/current")
}

pub fn default_sealed_release_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice35_sealed_release/current")
}

pub fn default_sealed_release_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("sealed_release_receipt.json")
}

pub fn default_terminal_boundary_manifest_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("terminal_boundary_manifest.json")
}

pub fn default_sealed_release_bundle_dir(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("sealed_release_bundle")
}

pub fn validate_handoff_boundary_source(source_dir: &Path) -> Result<(), Box<dyn Error>> {
    let receipt_path = default_release_attestation_receipt_path(source_dir);
    if !receipt_path.exists() {
        return Err("release attestation receipt missing from source".into());
    }
    let package_dir = default_handoff_boundary_package_dir(source_dir);
    let mut names = Vec::new();
    for entry in fs::read_dir(&package_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            names.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    names.sort();
    let expected = vec![
        "consumer_acknowledgment_receipt.json".to_string(),
        "downstream_release_receipt.json".to_string(),
        "operator_release_summary.json".to_string(),
        "release_readiness_receipt.json".to_string(),
        "return_channel_closure_receipt.json".to_string(),
    ];
    if names != expected {
        return Err("handoff boundary package member set mismatch".into());
    }
    Ok(())
}

pub fn build_terminal_boundary_manifest(
    source_dir: &Path,
) -> Result<TerminalBoundaryManifest, Box<dyn Error>> {
    validate_handoff_boundary_source(source_dir)?;

    let mut entries = Vec::new();
    let receipt_path = default_release_attestation_receipt_path(source_dir);
    entries.push(TerminalBoundaryManifestEntry {
        path: "release_attestation_receipt.json".to_string(),
        sha256: sha256_hex_file(&receipt_path)?,
    });

    let package_dir = default_handoff_boundary_package_dir(source_dir);
    let member_names = vec![
        "consumer_acknowledgment_receipt.json",
        "downstream_release_receipt.json",
        "operator_release_summary.json",
        "release_readiness_receipt.json",
        "return_channel_closure_receipt.json",
    ];
    for name in member_names {
        entries.push(TerminalBoundaryManifestEntry {
            path: format!("handoff_boundary_package/{}", name),
            sha256: sha256_hex_file(&package_dir.join(name))?,
        });
    }

    Ok(TerminalBoundaryManifest {
        schema_version: TERMINAL_BOUNDARY_MANIFEST_SCHEMA_VERSION.to_string(),
        entries,
    })
}

pub fn build_sealed_release_receipt(
    source_dir: &Path,
    manifest: &TerminalBoundaryManifest,
) -> Result<SealedReleaseReceipt, Box<dyn Error>> {
    validate_handoff_boundary_source(source_dir)?;
    let attestation =
        load_release_attestation_receipt(&default_release_attestation_receipt_path(source_dir))?;
    if !attestation.attested_for_handoff {
        return Err("release attestation receipt was not attested for handoff".into());
    }
    if attestation.export_member_count != 5 {
        return Err("release attestation export member count mismatch".into());
    }
    if manifest.entries.len() != 6 {
        return Err("terminal boundary manifest entry count mismatch".into());
    }

    Ok(SealedReleaseReceipt {
        schema_version: SEALED_RELEASE_RECEIPT_SCHEMA_VERSION.to_string(),
        release_attestation_receipt_sha256: sha256_hex_file(
            &default_release_attestation_receipt_path(source_dir),
        )?,
        terminal_boundary_manifest_sha256: sha256_hex_bytes(&serde_json::to_vec_pretty(manifest)?),
        sealed_member_count: 6,
        sealed_for_terminal_boundary: true,
    })
}

pub fn publish_sealed_release_bundle(
    source_dir: &Path,
    workspace_current_dir: &Path,
    manifest: &TerminalBoundaryManifest,
    receipt: &SealedReleaseReceipt,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(workspace_current_dir)?;
    let bundle_dir = default_sealed_release_bundle_dir(workspace_current_dir);
    fs::create_dir_all(&bundle_dir)?;

    fs::copy(
        default_release_attestation_receipt_path(source_dir),
        bundle_dir.join("release_attestation_receipt.json"),
    )?;
    let source_package_dir = default_handoff_boundary_package_dir(source_dir);
    for name in [
        "consumer_acknowledgment_receipt.json",
        "downstream_release_receipt.json",
        "operator_release_summary.json",
        "release_readiness_receipt.json",
        "return_channel_closure_receipt.json",
    ] {
        fs::copy(source_package_dir.join(name), bundle_dir.join(name))?;
    }

    write_terminal_boundary_manifest(
        &default_terminal_boundary_manifest_path(workspace_current_dir),
        manifest,
    )?;
    write_sealed_release_receipt(
        &default_sealed_release_receipt_path(workspace_current_dir),
        receipt,
    )?;
    Ok(())
}

pub fn write_terminal_boundary_manifest(
    path: &Path,
    manifest: &TerminalBoundaryManifest,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(manifest)?)?;
    Ok(())
}

pub fn load_terminal_boundary_manifest(
    path: &Path,
) -> Result<TerminalBoundaryManifest, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let manifest: TerminalBoundaryManifest = serde_json::from_slice(&bytes)?;
    Ok(manifest)
}

pub fn write_sealed_release_receipt(
    path: &Path,
    receipt: &SealedReleaseReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_sealed_release_receipt(path: &Path) -> Result<SealedReleaseReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: SealedReleaseReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

fn sha256_hex_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(sha256_hex_bytes(&bytes))
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

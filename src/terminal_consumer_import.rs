use crate::release_attestation::load_release_attestation_receipt;
use crate::sealed_release_bundle::{
    default_sealed_release_bundle_dir, default_sealed_release_receipt_path,
    default_terminal_boundary_manifest_path, load_sealed_release_receipt,
    load_terminal_boundary_manifest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const TERMINAL_CONSUMER_IMPORT_RECEIPT_SCHEMA_VERSION: &str =
    "proof.terminal-consumer-import-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalConsumerImportReceipt {
    pub schema_version: String,
    pub sealed_release_receipt_sha256: String,
    pub terminal_boundary_manifest_sha256: String,
    pub validated_bundle_member_count: usize,
    pub terminal_consumer_ready: bool,
}

pub fn default_terminal_consumer_source_dir() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice35_sealed_release/current")
}

pub fn default_terminal_consumer_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice36_terminal_consumer/current")
}

pub fn default_terminal_consumer_import_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("terminal_consumer_import_receipt.json")
}

pub fn default_validated_terminal_bundle_dir(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("validated_terminal_bundle")
}

pub fn validate_terminal_consumer_source(source_dir: &Path) -> Result<(), Box<dyn Error>> {
    let sealed_release_receipt =
        load_sealed_release_receipt(&default_sealed_release_receipt_path(source_dir))?;
    if !sealed_release_receipt.sealed_for_terminal_boundary {
        return Err("sealed release receipt was not sealed for terminal boundary".into());
    }
    if sealed_release_receipt.sealed_member_count != 6 {
        return Err("sealed release receipt member count mismatch".into());
    }

    let manifest =
        load_terminal_boundary_manifest(&default_terminal_boundary_manifest_path(source_dir))?;
    if manifest.entries.len() != 6 {
        return Err("terminal boundary manifest entry count mismatch".into());
    }

    let bundle_dir = default_sealed_release_bundle_dir(source_dir);
    let mut actual_names = BTreeSet::new();
    for entry in fs::read_dir(&bundle_dir)? {
        let entry = entry?;
        if entry.path().is_file() {
            actual_names.insert(entry.file_name().to_string_lossy().to_string());
        }
    }
    let expected_names = BTreeSet::from([
        "consumer_acknowledgment_receipt.json".to_string(),
        "downstream_release_receipt.json".to_string(),
        "operator_release_summary.json".to_string(),
        "release_attestation_receipt.json".to_string(),
        "release_readiness_receipt.json".to_string(),
        "return_channel_closure_receipt.json".to_string(),
    ]);
    if actual_names != expected_names {
        return Err("validated terminal bundle member set mismatch".into());
    }

    for entry in &manifest.entries {
        let path = match entry.path.as_str() {
            "release_attestation_receipt.json" => {
                bundle_dir.join("release_attestation_receipt.json")
            }
            other => {
                let prefix = "handoff_boundary_package/";
                if !other.starts_with(prefix) {
                    return Err("terminal boundary manifest path prefix mismatch".into());
                }
                bundle_dir.join(other.trim_start_matches(prefix))
            }
        };
        let actual_sha = sha256_hex_file(&path)?;
        if actual_sha != entry.sha256 {
            return Err("terminal boundary manifest hash mismatch".into());
        }
    }

    let attestation =
        load_release_attestation_receipt(&bundle_dir.join("release_attestation_receipt.json"))?;
    if !attestation.attested_for_handoff {
        return Err("release attestation receipt was not attested for handoff".into());
    }
    if attestation.export_member_count != 5 {
        return Err("release attestation export member count mismatch".into());
    }

    Ok(())
}

pub fn build_terminal_consumer_import_receipt(
    source_dir: &Path,
) -> Result<TerminalConsumerImportReceipt, Box<dyn Error>> {
    validate_terminal_consumer_source(source_dir)?;

    Ok(TerminalConsumerImportReceipt {
        schema_version: TERMINAL_CONSUMER_IMPORT_RECEIPT_SCHEMA_VERSION.to_string(),
        sealed_release_receipt_sha256: sha256_hex_file(&default_sealed_release_receipt_path(
            source_dir,
        ))?,
        terminal_boundary_manifest_sha256: sha256_hex_file(
            &default_terminal_boundary_manifest_path(source_dir),
        )?,
        validated_bundle_member_count: 6,
        terminal_consumer_ready: true,
    })
}

pub fn publish_terminal_consumer_import(
    source_dir: &Path,
    workspace_current_dir: &Path,
    receipt: &TerminalConsumerImportReceipt,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(workspace_current_dir)?;
    let validated_bundle_dir = default_validated_terminal_bundle_dir(workspace_current_dir);
    fs::create_dir_all(&validated_bundle_dir)?;

    let source_bundle_dir = default_sealed_release_bundle_dir(source_dir);
    for name in [
        "consumer_acknowledgment_receipt.json",
        "downstream_release_receipt.json",
        "operator_release_summary.json",
        "release_attestation_receipt.json",
        "release_readiness_receipt.json",
        "return_channel_closure_receipt.json",
    ] {
        fs::copy(
            source_bundle_dir.join(name),
            validated_bundle_dir.join(name),
        )?;
    }
    write_terminal_consumer_import_receipt(
        &default_terminal_consumer_import_receipt_path(workspace_current_dir),
        receipt,
    )?;
    Ok(())
}

pub fn write_terminal_consumer_import_receipt(
    path: &Path,
    receipt: &TerminalConsumerImportReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_terminal_consumer_import_receipt(
    path: &Path,
) -> Result<TerminalConsumerImportReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: TerminalConsumerImportReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

fn sha256_hex_file(path: &Path) -> Result<String, Box<dyn Error>> {
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

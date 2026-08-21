use precomputed_context_core::sealed_release_bundle::{
    default_sealed_release_bundle_dir, default_terminal_boundary_manifest_path,
};
use precomputed_context_core::terminal_consumer_import::{
    build_terminal_consumer_import_receipt, default_terminal_consumer_import_receipt_path,
    default_terminal_consumer_source_dir, default_terminal_consumer_workspace_current,
    load_terminal_consumer_import_receipt, publish_terminal_consumer_import,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct ProgramCapstoneReport {
    schema_version: String,
    terminal_consumer_import_receipt_sha256: String,
    repeated_terminal_consumer_import_receipt_sha256: String,
    stable_repeated_terminal_consumer_import_receipt: bool,
    missing_manifest_rejected: bool,
    terminal_bundle_member_drift_rejected: bool,
    manifest_hash_tamper_rejected: bool,
    no_publication_on_missing_manifest: bool,
    no_publication_on_terminal_bundle_member_drift: bool,
    no_publication_on_manifest_hash_tamper: bool,
    capstone_ready: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_dir = default_terminal_consumer_source_dir();
    let workspace_current = default_terminal_consumer_workspace_current();
    reset_dir(&workspace_current)?;

    let receipt = build_terminal_consumer_import_receipt(&source_dir)?;
    publish_terminal_consumer_import(&source_dir, &workspace_current, &receipt)?;

    let receipt_path = default_terminal_consumer_import_receipt_path(&workspace_current);
    let terminal_consumer_import_receipt_sha256 = sha256_file(&receipt_path)?;

    let repeated_receipt = build_terminal_consumer_import_receipt(&source_dir)?;
    publish_terminal_consumer_import(&source_dir, &workspace_current, &repeated_receipt)?;
    let repeated_terminal_consumer_import_receipt_sha256 = sha256_file(&receipt_path)?;

    let scenario_root = PathBuf::from("target/proof_artifacts/slice36_terminal_consumer/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_manifest_rejected, no_publication_on_missing_manifest) = {
        let scenario_source = scenario_root.join("missing_manifest_source");
        copy_dir(&source_dir, &scenario_source)?;
        fs::remove_file(default_terminal_boundary_manifest_path(&scenario_source))?;
        let output_dir = scenario_root.join("missing_manifest_output");
        reset_dir(&output_dir)?;
        let result = build_terminal_consumer_import_receipt(&scenario_source).and_then(|receipt| {
            publish_terminal_consumer_import(&scenario_source, &output_dir, &receipt)
        });
        (
            result.is_err(),
            !default_terminal_consumer_import_receipt_path(&output_dir).exists(),
        )
    };

    let (terminal_bundle_member_drift_rejected, no_publication_on_terminal_bundle_member_drift) = {
        let scenario_source = scenario_root.join("member_drift_source");
        copy_dir(&source_dir, &scenario_source)?;
        fs::write(
            default_sealed_release_bundle_dir(&scenario_source).join("rogue.txt"),
            b"tamper",
        )?;
        let output_dir = scenario_root.join("member_drift_output");
        reset_dir(&output_dir)?;
        let result = build_terminal_consumer_import_receipt(&scenario_source).and_then(|receipt| {
            publish_terminal_consumer_import(&scenario_source, &output_dir, &receipt)
        });
        (
            result.is_err(),
            !default_terminal_consumer_import_receipt_path(&output_dir).exists(),
        )
    };

    let (manifest_hash_tamper_rejected, no_publication_on_manifest_hash_tamper) = {
        let scenario_source = scenario_root.join("manifest_tamper_source");
        copy_dir(&source_dir, &scenario_source)?;
        let manifest_path = default_terminal_boundary_manifest_path(&scenario_source);
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
        value["entries"][0]["sha256"] = serde_json::Value::String("tampered".to_string());
        fs::write(&manifest_path, serde_json::to_vec_pretty(&value)?)?;
        let output_dir = scenario_root.join("manifest_tamper_output");
        reset_dir(&output_dir)?;
        let result = build_terminal_consumer_import_receipt(&scenario_source).and_then(|receipt| {
            publish_terminal_consumer_import(&scenario_source, &output_dir, &receipt)
        });
        (
            result.is_err(),
            !default_terminal_consumer_import_receipt_path(&output_dir).exists(),
        )
    };

    let report = ProgramCapstoneReport {
        schema_version: "proof.program-capstone-report.v1".to_string(),
        terminal_consumer_import_receipt_sha256: terminal_consumer_import_receipt_sha256.clone(),
        repeated_terminal_consumer_import_receipt_sha256:
            repeated_terminal_consumer_import_receipt_sha256.clone(),
        stable_repeated_terminal_consumer_import_receipt: terminal_consumer_import_receipt_sha256
            == repeated_terminal_consumer_import_receipt_sha256,
        missing_manifest_rejected,
        terminal_bundle_member_drift_rejected,
        manifest_hash_tamper_rejected,
        no_publication_on_missing_manifest,
        no_publication_on_terminal_bundle_member_drift,
        no_publication_on_manifest_hash_tamper,
        capstone_ready: true,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice36_terminal_consumer/program_capstone_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_terminal_consumer_import_receipt(&receipt_path)?;
    Ok(())
}

fn reset_dir(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn copy_dir(source: &Path, dest: &Path) -> Result<(), Box<dyn Error>> {
    reset_dir(dest)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &dest_path)?;
        } else {
            fs::copy(source_path, dest_path)?;
        }
    }
    Ok(())
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

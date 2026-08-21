use precomputed_context_core::sealed_release_bundle::{
    build_sealed_release_receipt, build_terminal_boundary_manifest,
    default_sealed_release_receipt_path, default_sealed_release_source_dir,
    default_sealed_release_workspace_current, default_terminal_boundary_manifest_path,
    load_sealed_release_receipt, load_terminal_boundary_manifest, publish_sealed_release_bundle,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct SealedReleaseReport {
    schema_version: String,
    sealed_release_receipt_sha256: String,
    repeated_sealed_release_receipt_sha256: String,
    stable_repeated_sealed_release_receipt: bool,
    terminal_boundary_manifest_sha256: String,
    repeated_terminal_boundary_manifest_sha256: String,
    stable_repeated_terminal_boundary_manifest: bool,
    missing_attestation_receipt_rejected: bool,
    handoff_boundary_member_drift_rejected: bool,
    attestation_member_count_tamper_rejected: bool,
    no_publication_on_missing_attestation_receipt: bool,
    no_publication_on_handoff_boundary_member_drift: bool,
    no_publication_on_attestation_member_count_tamper: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_dir = default_sealed_release_source_dir();
    let workspace_current = default_sealed_release_workspace_current();
    reset_dir(&workspace_current)?;

    let manifest = build_terminal_boundary_manifest(&source_dir)?;
    let receipt = build_sealed_release_receipt(&source_dir, &manifest)?;
    publish_sealed_release_bundle(&source_dir, &workspace_current, &manifest, &receipt)?;

    let receipt_path = default_sealed_release_receipt_path(&workspace_current);
    let manifest_path = default_terminal_boundary_manifest_path(&workspace_current);
    let sealed_release_receipt_sha256 = sha256_file(&receipt_path)?;
    let terminal_boundary_manifest_sha256 = sha256_file(&manifest_path)?;

    let repeated_manifest = build_terminal_boundary_manifest(&source_dir)?;
    let repeated_receipt = build_sealed_release_receipt(&source_dir, &repeated_manifest)?;
    publish_sealed_release_bundle(
        &source_dir,
        &workspace_current,
        &repeated_manifest,
        &repeated_receipt,
    )?;
    let repeated_sealed_release_receipt_sha256 = sha256_file(&receipt_path)?;
    let repeated_terminal_boundary_manifest_sha256 = sha256_file(&manifest_path)?;

    let scenario_root = PathBuf::from("target/proof_artifacts/slice35_sealed_release/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_attestation_receipt_rejected, no_publication_on_missing_attestation_receipt) = {
        let scenario_source = scenario_root.join("missing_attestation_source");
        copy_dir(&source_dir, &scenario_source)?;
        fs::remove_file(scenario_source.join("release_attestation_receipt.json"))?;
        let output_dir = scenario_root.join("missing_attestation_output");
        reset_dir(&output_dir)?;
        let result = build_terminal_boundary_manifest(&scenario_source).and_then(|manifest| {
            let receipt = build_sealed_release_receipt(&scenario_source, &manifest)?;
            publish_sealed_release_bundle(&scenario_source, &output_dir, &manifest, &receipt)
        });
        (
            result.is_err(),
            !default_sealed_release_receipt_path(&output_dir).exists()
                && !default_terminal_boundary_manifest_path(&output_dir).exists(),
        )
    };

    let (handoff_boundary_member_drift_rejected, no_publication_on_handoff_boundary_member_drift) = {
        let scenario_source = scenario_root.join("member_drift_source");
        copy_dir(&source_dir, &scenario_source)?;
        fs::write(
            scenario_source.join("handoff_boundary_package/extra.txt"),
            b"tamper",
        )?;
        let output_dir = scenario_root.join("member_drift_output");
        reset_dir(&output_dir)?;
        let result = build_terminal_boundary_manifest(&scenario_source).and_then(|manifest| {
            let receipt = build_sealed_release_receipt(&scenario_source, &manifest)?;
            publish_sealed_release_bundle(&scenario_source, &output_dir, &manifest, &receipt)
        });
        (
            result.is_err(),
            !default_sealed_release_receipt_path(&output_dir).exists()
                && !default_terminal_boundary_manifest_path(&output_dir).exists(),
        )
    };

    let (
        attestation_member_count_tamper_rejected,
        no_publication_on_attestation_member_count_tamper,
    ) = {
        let scenario_source = scenario_root.join("attestation_tamper_source");
        copy_dir(&source_dir, &scenario_source)?;
        let attestation_path = scenario_source.join("release_attestation_receipt.json");
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&attestation_path)?)?;
        value["export_member_count"] = serde_json::Value::Number(serde_json::Number::from(4));
        fs::write(&attestation_path, serde_json::to_vec_pretty(&value)?)?;
        let output_dir = scenario_root.join("attestation_tamper_output");
        reset_dir(&output_dir)?;
        let result = build_terminal_boundary_manifest(&scenario_source).and_then(|manifest| {
            let receipt = build_sealed_release_receipt(&scenario_source, &manifest)?;
            publish_sealed_release_bundle(&scenario_source, &output_dir, &manifest, &receipt)
        });
        (
            result.is_err(),
            !default_sealed_release_receipt_path(&output_dir).exists()
                && !default_terminal_boundary_manifest_path(&output_dir).exists(),
        )
    };

    let report = SealedReleaseReport {
        schema_version: "proof.sealed-release-report.v1".to_string(),
        sealed_release_receipt_sha256: sealed_release_receipt_sha256.clone(),
        repeated_sealed_release_receipt_sha256: repeated_sealed_release_receipt_sha256.clone(),
        stable_repeated_sealed_release_receipt: sealed_release_receipt_sha256
            == repeated_sealed_release_receipt_sha256,
        terminal_boundary_manifest_sha256: terminal_boundary_manifest_sha256.clone(),
        repeated_terminal_boundary_manifest_sha256: repeated_terminal_boundary_manifest_sha256
            .clone(),
        stable_repeated_terminal_boundary_manifest: terminal_boundary_manifest_sha256
            == repeated_terminal_boundary_manifest_sha256,
        missing_attestation_receipt_rejected,
        handoff_boundary_member_drift_rejected,
        attestation_member_count_tamper_rejected,
        no_publication_on_missing_attestation_receipt,
        no_publication_on_handoff_boundary_member_drift,
        no_publication_on_attestation_member_count_tamper,
    };

    let report_path =
        PathBuf::from("target/proof_artifacts/slice35_sealed_release/sealed_release_report.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_sealed_release_receipt(&receipt_path)?;
    let _ = load_terminal_boundary_manifest(&manifest_path)?;
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

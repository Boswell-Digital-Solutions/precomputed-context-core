use precomputed_context_core::release_attestation::{
    build_release_attestation_receipt, default_release_attestation_downstream_source_dir,
    default_release_attestation_readiness_source_dir, default_release_attestation_receipt_path,
    default_release_attestation_workspace_current, load_release_attestation_receipt,
    publish_release_attestation_package,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct ReleaseAttestationReport {
    schema_version: String,
    attestation_receipt_sha256: String,
    repeated_attestation_receipt_sha256: String,
    stable_repeated_attestation_receipt: bool,
    missing_operator_summary_rejected: bool,
    readiness_receipt_continuity_tamper_rejected: bool,
    missing_closure_receipt_rejected: bool,
    no_publication_on_missing_operator_summary: bool,
    no_publication_on_readiness_receipt_continuity_tamper: bool,
    no_publication_on_missing_closure_receipt: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let downstream_source = default_release_attestation_downstream_source_dir();
    let readiness_source = default_release_attestation_readiness_source_dir();
    let workspace_current = default_release_attestation_workspace_current();
    reset_dir(&workspace_current)?;

    let receipt = build_release_attestation_receipt(&downstream_source, &readiness_source)?;
    publish_release_attestation_package(
        &downstream_source,
        &readiness_source,
        &workspace_current,
        &receipt,
    )?;

    let receipt_path = default_release_attestation_receipt_path(&workspace_current);
    let attestation_receipt_sha256 = sha256_file(&receipt_path)?;

    let repeated_receipt =
        build_release_attestation_receipt(&downstream_source, &readiness_source)?;
    publish_release_attestation_package(
        &downstream_source,
        &readiness_source,
        &workspace_current,
        &repeated_receipt,
    )?;
    let repeated_attestation_receipt_sha256 = sha256_file(&receipt_path)?;

    let scenario_root =
        PathBuf::from("target/proof_artifacts/slice34_release_attestation/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_operator_summary_rejected, no_publication_on_missing_operator_summary) = {
        let readiness_scenario = scenario_root.join("missing_summary_readiness");
        copy_dir(&readiness_source, &readiness_scenario)?;
        fs::remove_file(readiness_scenario.join("operator_release_summary.json"))?;
        let output_dir = scenario_root.join("missing_summary_output");
        reset_dir(&output_dir)?;
        let result = build_release_attestation_receipt(&downstream_source, &readiness_scenario)
            .and_then(|receipt| {
                publish_release_attestation_package(
                    &downstream_source,
                    &readiness_scenario,
                    &output_dir,
                    &receipt,
                )
            });
        (
            result.is_err(),
            !default_release_attestation_receipt_path(&output_dir).exists(),
        )
    };

    let (
        readiness_receipt_continuity_tamper_rejected,
        no_publication_on_readiness_receipt_continuity_tamper,
    ) = {
        let readiness_scenario = scenario_root.join("continuity_tamper_readiness");
        copy_dir(&readiness_source, &readiness_scenario)?;
        let readiness_receipt_path = readiness_scenario.join("release_readiness_receipt.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&readiness_receipt_path)?)?;
        value["downstream_release_receipt_sha256"] =
            serde_json::Value::String("tampered".to_string());
        fs::write(&readiness_receipt_path, serde_json::to_vec_pretty(&value)?)?;
        let output_dir = scenario_root.join("continuity_tamper_output");
        reset_dir(&output_dir)?;
        let result = build_release_attestation_receipt(&downstream_source, &readiness_scenario)
            .and_then(|receipt| {
                publish_release_attestation_package(
                    &downstream_source,
                    &readiness_scenario,
                    &output_dir,
                    &receipt,
                )
            });
        (
            result.is_err(),
            !default_release_attestation_receipt_path(&output_dir).exists(),
        )
    };

    let (missing_closure_receipt_rejected, no_publication_on_missing_closure_receipt) = {
        let downstream_scenario = scenario_root.join("missing_closure_downstream");
        copy_dir(&downstream_source, &downstream_scenario)?;
        fs::remove_file(
            downstream_scenario
                .join("release_package")
                .join("return_channel_closure_receipt.json"),
        )?;
        let output_dir = scenario_root.join("missing_closure_output");
        reset_dir(&output_dir)?;
        let result = build_release_attestation_receipt(&downstream_scenario, &readiness_source)
            .and_then(|receipt| {
                publish_release_attestation_package(
                    &downstream_scenario,
                    &readiness_source,
                    &output_dir,
                    &receipt,
                )
            });
        (
            result.is_err(),
            !default_release_attestation_receipt_path(&output_dir).exists(),
        )
    };

    let report = ReleaseAttestationReport {
        schema_version: "proof.release-attestation-report.v1".to_string(),
        attestation_receipt_sha256: attestation_receipt_sha256.clone(),
        repeated_attestation_receipt_sha256: repeated_attestation_receipt_sha256.clone(),
        stable_repeated_attestation_receipt: attestation_receipt_sha256
            == repeated_attestation_receipt_sha256,
        missing_operator_summary_rejected,
        readiness_receipt_continuity_tamper_rejected,
        missing_closure_receipt_rejected,
        no_publication_on_missing_operator_summary,
        no_publication_on_readiness_receipt_continuity_tamper,
        no_publication_on_missing_closure_receipt,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice34_release_attestation/release_attestation_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_release_attestation_receipt(&receipt_path)?;
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

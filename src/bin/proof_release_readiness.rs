use precomputed_context_core::downstream_release::{
    default_downstream_release_package_dir, default_downstream_release_receipt_path,
};
use precomputed_context_core::release_readiness::{
    build_operator_release_summary, build_release_readiness_receipt,
    default_operator_release_summary_path, default_release_readiness_receipt_path,
    default_release_readiness_source_dir, default_release_readiness_workspace_current,
    load_operator_release_summary, load_release_readiness_receipt, publish_release_readiness,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct ReleaseReadinessReport {
    schema_version: String,
    readiness_receipt_sha256: String,
    repeated_readiness_receipt_sha256: String,
    stable_repeated_readiness_receipt: bool,
    operator_summary_sha256: String,
    repeated_operator_summary_sha256: String,
    stable_repeated_operator_summary: bool,
    missing_closure_receipt_rejected: bool,
    release_package_member_drift_rejected: bool,
    release_receipt_continuity_tamper_rejected: bool,
    no_publication_on_missing_closure_receipt: bool,
    no_publication_on_release_package_member_drift: bool,
    no_publication_on_release_receipt_continuity_tamper: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_dir = default_release_readiness_source_dir();
    let workspace_current = default_release_readiness_workspace_current();
    reset_dir(&workspace_current)?;

    let receipt = build_release_readiness_receipt(&source_dir)?;
    let summary = build_operator_release_summary(&source_dir)?;
    publish_release_readiness(&workspace_current, &receipt, &summary)?;

    let readiness_receipt_path = default_release_readiness_receipt_path(&workspace_current);
    let operator_summary_path = default_operator_release_summary_path(&workspace_current);
    let readiness_receipt_sha256 = sha256_file(&readiness_receipt_path)?;
    let operator_summary_sha256 = sha256_file(&operator_summary_path)?;

    let repeated_receipt = build_release_readiness_receipt(&source_dir)?;
    let repeated_summary = build_operator_release_summary(&source_dir)?;
    publish_release_readiness(&workspace_current, &repeated_receipt, &repeated_summary)?;
    let repeated_readiness_receipt_sha256 = sha256_file(&readiness_receipt_path)?;
    let repeated_operator_summary_sha256 = sha256_file(&operator_summary_path)?;

    let scenario_root = PathBuf::from("target/proof_artifacts/slice33_release_readiness/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_closure_receipt_rejected, no_publication_on_missing_closure_receipt) = {
        let scenario_source = scenario_root.join("missing_closure_source");
        copy_dir(&source_dir, &scenario_source)?;
        fs::remove_file(
            default_downstream_release_package_dir(&scenario_source)
                .join("return_channel_closure_receipt.json"),
        )?;
        let scenario_output = scenario_root.join("missing_closure_output");
        reset_dir(&scenario_output)?;
        let result = build_release_readiness_receipt(&scenario_source).and_then(|receipt| {
            let summary = build_operator_release_summary(&scenario_source)?;
            publish_release_readiness(&scenario_output, &receipt, &summary)
        });
        (
            result.is_err(),
            !default_release_readiness_receipt_path(&scenario_output).exists()
                && !default_operator_release_summary_path(&scenario_output).exists(),
        )
    };

    let (release_package_member_drift_rejected, no_publication_on_release_package_member_drift) = {
        let scenario_source = scenario_root.join("member_drift_source");
        copy_dir(&source_dir, &scenario_source)?;
        fs::write(
            default_downstream_release_package_dir(&scenario_source).join("extra.txt"),
            b"tamper",
        )?;
        let scenario_output = scenario_root.join("member_drift_output");
        reset_dir(&scenario_output)?;
        let result = build_release_readiness_receipt(&scenario_source).and_then(|receipt| {
            let summary = build_operator_release_summary(&scenario_source)?;
            publish_release_readiness(&scenario_output, &receipt, &summary)
        });
        (
            result.is_err(),
            !default_release_readiness_receipt_path(&scenario_output).exists()
                && !default_operator_release_summary_path(&scenario_output).exists(),
        )
    };

    let (
        release_receipt_continuity_tamper_rejected,
        no_publication_on_release_receipt_continuity_tamper,
    ) = {
        let scenario_source = scenario_root.join("receipt_tamper_source");
        copy_dir(&source_dir, &scenario_source)?;
        let receipt_path = default_downstream_release_receipt_path(&scenario_source);
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
        value["closure_receipt_sha256"] = serde_json::Value::String("tampered".to_string());
        fs::write(&receipt_path, serde_json::to_vec_pretty(&value)?)?;
        let scenario_output = scenario_root.join("receipt_tamper_output");
        reset_dir(&scenario_output)?;
        let result = build_release_readiness_receipt(&scenario_source).and_then(|receipt| {
            let summary = build_operator_release_summary(&scenario_source)?;
            publish_release_readiness(&scenario_output, &receipt, &summary)
        });
        (
            result.is_err(),
            !default_release_readiness_receipt_path(&scenario_output).exists()
                && !default_operator_release_summary_path(&scenario_output).exists(),
        )
    };

    let report = ReleaseReadinessReport {
        schema_version: "proof.release-readiness-report.v1".to_string(),
        readiness_receipt_sha256: readiness_receipt_sha256.clone(),
        repeated_readiness_receipt_sha256: repeated_readiness_receipt_sha256.clone(),
        stable_repeated_readiness_receipt: readiness_receipt_sha256
            == repeated_readiness_receipt_sha256,
        operator_summary_sha256: operator_summary_sha256.clone(),
        repeated_operator_summary_sha256: repeated_operator_summary_sha256.clone(),
        stable_repeated_operator_summary: operator_summary_sha256
            == repeated_operator_summary_sha256,
        missing_closure_receipt_rejected,
        release_package_member_drift_rejected,
        release_receipt_continuity_tamper_rejected,
        no_publication_on_missing_closure_receipt,
        no_publication_on_release_package_member_drift,
        no_publication_on_release_receipt_continuity_tamper,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice33_release_readiness/release_readiness_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_release_readiness_receipt(&readiness_receipt_path)?;
    let _ = load_operator_release_summary(&operator_summary_path)?;
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

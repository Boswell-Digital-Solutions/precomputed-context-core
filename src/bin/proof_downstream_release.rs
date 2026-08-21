use precomputed_context_core::consumer_acknowledgment::default_return_channel_closure_receipt_path;
use precomputed_context_core::consumer_handoff::default_bounded_consumer_handoff_package_dir;
use precomputed_context_core::downstream_release::{
    build_downstream_release_receipt, default_downstream_release_ack_source_dir,
    default_downstream_release_handoff_source_dir, default_downstream_release_package_dir,
    default_downstream_release_receipt_path, default_downstream_release_workspace_current,
    load_downstream_release_receipt, publish_downstream_release_package,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct DownstreamReleaseReport {
    schema_version: String,
    release_receipt_sha256: String,
    repeated_release_receipt_sha256: String,
    stable_repeated_release_receipt: bool,
    bounded_release_member_set_enforced: bool,
    missing_closure_receipt_rejected: bool,
    closure_drift_rejected: bool,
    package_member_drift_rejected: bool,
    no_release_publication_on_missing_closure_receipt: bool,
    no_release_publication_on_closure_drift: bool,
    no_release_publication_on_package_member_drift: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let handoff_source = default_downstream_release_handoff_source_dir();
    let ack_source = default_downstream_release_ack_source_dir();
    let workspace_current = default_downstream_release_workspace_current();
    reset_dir(&workspace_current)?;

    let receipt = build_downstream_release_receipt(&handoff_source, &ack_source)?;
    publish_downstream_release_package(&handoff_source, &ack_source, &workspace_current, &receipt)?;

    let release_receipt_path = default_downstream_release_receipt_path(&workspace_current);
    let release_receipt_sha256 = sha256_file(&release_receipt_path)?;

    let repeated_receipt = build_downstream_release_receipt(&handoff_source, &ack_source)?;
    publish_downstream_release_package(
        &handoff_source,
        &ack_source,
        &workspace_current,
        &repeated_receipt,
    )?;
    let repeated_release_receipt_sha256 = sha256_file(&release_receipt_path)?;

    let bounded_release_member_set_enforced =
        sorted_relative_files(&default_downstream_release_package_dir(&workspace_current))?
            == vec![
                "activation_receipt.json".to_string(),
                "attestation_receipt.json".to_string(),
                "consumer_acknowledgment_receipt.json".to_string(),
                "consumer_contract.json".to_string(),
                "return_channel_closure_receipt.json".to_string(),
                "supersession_chain_receipt.json".to_string(),
            ];

    let scenario_root =
        PathBuf::from("target/proof_artifacts/slice32_downstream_release/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_closure_receipt_rejected, no_release_publication_on_missing_closure_receipt) = {
        let scenario_ack = scenario_root.join("missing_closure_ack");
        copy_dir(&ack_source, &scenario_ack)?;
        fs::remove_file(default_return_channel_closure_receipt_path(&scenario_ack))?;
        let scenario_output = scenario_root.join("missing_closure_output");
        reset_dir(&scenario_output)?;
        let result =
            build_downstream_release_receipt(&handoff_source, &scenario_ack).and_then(|receipt| {
                publish_downstream_release_package(
                    &handoff_source,
                    &scenario_ack,
                    &scenario_output,
                    &receipt,
                )
            });
        (
            result.is_err(),
            !default_downstream_release_receipt_path(&scenario_output).exists(),
        )
    };

    let (closure_drift_rejected, no_release_publication_on_closure_drift) = {
        let scenario_ack = scenario_root.join("closure_drift_ack");
        copy_dir(&ack_source, &scenario_ack)?;
        let closure_path = default_return_channel_closure_receipt_path(&scenario_ack);
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&closure_path)?)?;
        value["handoff_receipt_sha256"] = serde_json::Value::String("tampered".to_string());
        fs::write(&closure_path, serde_json::to_vec_pretty(&value)?)?;
        let scenario_output = scenario_root.join("closure_drift_output");
        reset_dir(&scenario_output)?;
        let result =
            build_downstream_release_receipt(&handoff_source, &scenario_ack).and_then(|receipt| {
                publish_downstream_release_package(
                    &handoff_source,
                    &scenario_ack,
                    &scenario_output,
                    &receipt,
                )
            });
        (
            result.is_err(),
            !default_downstream_release_receipt_path(&scenario_output).exists(),
        )
    };

    let (package_member_drift_rejected, no_release_publication_on_package_member_drift) = {
        let scenario_handoff = scenario_root.join("package_drift_handoff");
        copy_dir(&handoff_source, &scenario_handoff)?;
        fs::write(
            default_bounded_consumer_handoff_package_dir(&scenario_handoff).join("drift.txt"),
            b"tamper",
        )?;
        let scenario_output = scenario_root.join("package_drift_output");
        reset_dir(&scenario_output)?;
        let result =
            build_downstream_release_receipt(&scenario_handoff, &ack_source).and_then(|receipt| {
                publish_downstream_release_package(
                    &scenario_handoff,
                    &ack_source,
                    &scenario_output,
                    &receipt,
                )
            });
        (
            result.is_err(),
            !default_downstream_release_receipt_path(&scenario_output).exists(),
        )
    };

    let report = DownstreamReleaseReport {
        schema_version: "proof.downstream-release-report.v1".to_string(),
        release_receipt_sha256: release_receipt_sha256.clone(),
        repeated_release_receipt_sha256: repeated_release_receipt_sha256.clone(),
        stable_repeated_release_receipt: release_receipt_sha256 == repeated_release_receipt_sha256,
        bounded_release_member_set_enforced,
        missing_closure_receipt_rejected,
        closure_drift_rejected,
        package_member_drift_rejected,
        no_release_publication_on_missing_closure_receipt,
        no_release_publication_on_closure_drift,
        no_release_publication_on_package_member_drift,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice32_downstream_release/downstream_release_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_downstream_release_receipt(&release_receipt_path)?;
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

fn sorted_relative_files(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            files.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    files.sort();
    Ok(files)
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

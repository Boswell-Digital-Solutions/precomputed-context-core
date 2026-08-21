use precomputed_context_core::consumer_handoff::{
    build_bounded_consumer_handoff_receipt, default_bounded_consumer_handoff_package_dir,
    default_bounded_consumer_handoff_receipt_path,
    default_consumer_handoff_active_lineage_source_dir,
    default_consumer_handoff_consumption_source_dir, default_consumer_handoff_workspace_current,
    load_bounded_consumer_handoff_receipt, publish_bounded_consumer_handoff,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct ConsumerHandoffReport {
    schema_version: String,
    handoff_receipt_file_name: String,
    handoff_receipt_sha256: String,
    repeated_handoff_receipt_sha256: String,
    stable_repeated_handoff_receipt: bool,
    bounded_member_set_enforced: bool,
    missing_attestation_receipt_rejected: bool,
    attestation_mismatch_rejected: bool,
    missing_activation_receipt_rejected: bool,
    no_receipt_publication_on_missing_attestation_receipt: bool,
    no_receipt_publication_on_attestation_mismatch: bool,
    no_receipt_publication_on_missing_activation_receipt: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let active_lineage_source = default_consumer_handoff_active_lineage_source_dir();
    let consumption_source = default_consumer_handoff_consumption_source_dir();
    let workspace_current = default_consumer_handoff_workspace_current();

    reset_dir(&workspace_current)?;
    let receipt =
        build_bounded_consumer_handoff_receipt(&active_lineage_source, &consumption_source)?;
    publish_bounded_consumer_handoff(
        &active_lineage_source,
        &consumption_source,
        &workspace_current,
        &receipt,
    )?;

    let handoff_receipt_path = default_bounded_consumer_handoff_receipt_path(&workspace_current);
    let handoff_receipt_sha256 = sha256_file(&handoff_receipt_path)?;

    let repeated_receipt =
        build_bounded_consumer_handoff_receipt(&active_lineage_source, &consumption_source)?;
    publish_bounded_consumer_handoff(
        &active_lineage_source,
        &consumption_source,
        &workspace_current,
        &repeated_receipt,
    )?;
    let repeated_handoff_receipt_sha256 = sha256_file(&handoff_receipt_path)?;

    let package_dir = default_bounded_consumer_handoff_package_dir(&workspace_current);
    let bounded_member_set_enforced = sorted_relative_files(&package_dir)?
        == vec![
            "activation_receipt.json".to_string(),
            "attestation_receipt.json".to_string(),
            "consumer_contract.json".to_string(),
            "supersession_chain_receipt.json".to_string(),
        ];

    let scenario_root = PathBuf::from("target/proof_artifacts/slice30_consumer_handoff/scenarios");
    reset_dir(&scenario_root)?;

    let (
        missing_attestation_receipt_rejected,
        no_receipt_publication_on_missing_attestation_receipt,
    ) = {
        let scenario_consumption = scenario_root.join("missing_attestation_consumption");
        copy_dir(&consumption_source, &scenario_consumption)?;
        fs::remove_file(scenario_consumption.join("attestation_receipt.json"))?;
        let scenario_output = scenario_root.join("missing_attestation_output");
        reset_dir(&scenario_output)?;
        let result =
            build_bounded_consumer_handoff_receipt(&active_lineage_source, &scenario_consumption)
                .and_then(|receipt| {
                    publish_bounded_consumer_handoff(
                        &active_lineage_source,
                        &scenario_consumption,
                        &scenario_output,
                        &receipt,
                    )
                });
        (
            result.is_err(),
            !default_bounded_consumer_handoff_receipt_path(&scenario_output).exists(),
        )
    };

    let (attestation_mismatch_rejected, no_receipt_publication_on_attestation_mismatch) = {
        let scenario_consumption = scenario_root.join("attestation_mismatch_consumption");
        copy_dir(&consumption_source, &scenario_consumption)?;
        fs::write(
            scenario_consumption.join("attestation_receipt.json"),
            b"{\"tampered\":true}",
        )?;
        let scenario_output = scenario_root.join("attestation_mismatch_output");
        reset_dir(&scenario_output)?;
        let result =
            build_bounded_consumer_handoff_receipt(&active_lineage_source, &scenario_consumption)
                .and_then(|receipt| {
                    publish_bounded_consumer_handoff(
                        &active_lineage_source,
                        &scenario_consumption,
                        &scenario_output,
                        &receipt,
                    )
                });
        (
            result.is_err(),
            !default_bounded_consumer_handoff_receipt_path(&scenario_output).exists(),
        )
    };

    let (missing_activation_receipt_rejected, no_receipt_publication_on_missing_activation_receipt) = {
        let scenario_active = scenario_root.join("missing_activation_active");
        copy_dir(&active_lineage_source, &scenario_active)?;
        fs::remove_file(scenario_active.join("activation_receipt.json"))?;
        let scenario_output = scenario_root.join("missing_activation_output");
        reset_dir(&scenario_output)?;
        let result = build_bounded_consumer_handoff_receipt(&scenario_active, &consumption_source)
            .and_then(|receipt| {
                publish_bounded_consumer_handoff(
                    &scenario_active,
                    &consumption_source,
                    &scenario_output,
                    &receipt,
                )
            });
        (
            result.is_err(),
            !default_bounded_consumer_handoff_receipt_path(&scenario_output).exists(),
        )
    };

    let report = ConsumerHandoffReport {
        schema_version: "proof.consumer-handoff-report.v1".to_string(),
        handoff_receipt_file_name: file_name_string(&handoff_receipt_path)?,
        handoff_receipt_sha256: handoff_receipt_sha256.clone(),
        repeated_handoff_receipt_sha256: repeated_handoff_receipt_sha256.clone(),
        stable_repeated_handoff_receipt: handoff_receipt_sha256 == repeated_handoff_receipt_sha256,
        bounded_member_set_enforced,
        missing_attestation_receipt_rejected,
        attestation_mismatch_rejected,
        missing_activation_receipt_rejected,
        no_receipt_publication_on_missing_attestation_receipt,
        no_receipt_publication_on_attestation_mismatch,
        no_receipt_publication_on_missing_activation_receipt,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice30_consumer_handoff/consumer_handoff_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_bounded_consumer_handoff_receipt(&handoff_receipt_path)?;
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

fn file_name_string(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .file_name()
        .ok_or("path has no file name")?
        .to_string_lossy()
        .to_string())
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

use precomputed_context_core::lineage_consumption::{
    build_active_lineage_attestation_receipt, default_active_lineage_attestation_receipt_path,
    default_active_lineage_consumer_contract, default_active_lineage_consumer_contract_path,
    default_lineage_consumption_source_dir, default_lineage_consumption_workspace_current,
    load_active_lineage_attestation_receipt, publish_active_lineage_attestation,
    write_active_lineage_consumer_contract,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct ActiveLineageAttestationReport {
    schema_version: String,
    attestation_receipt_file_name: String,
    attestation_receipt_sha256: String,
    repeated_attestation_receipt_sha256: String,
    stable_repeated_attestation_receipt: bool,
    missing_activation_receipt_rejected: bool,
    contract_without_admitted_requirement_rejected: bool,
    corrupted_supersession_rejected: bool,
    no_receipt_publication_on_missing_activation_receipt: bool,
    no_receipt_publication_on_contract_without_admitted_requirement: bool,
    no_receipt_publication_on_corrupted_supersession: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_dir = default_lineage_consumption_source_dir();
    let workspace_current = default_lineage_consumption_workspace_current();

    reset_dir(&workspace_current)?;
    let contract = default_active_lineage_consumer_contract();
    let contract_path = default_active_lineage_consumer_contract_path(&workspace_current);
    write_active_lineage_consumer_contract(&contract_path, &contract)?;

    let receipt = build_active_lineage_attestation_receipt(&source_dir, &contract_path)?;
    publish_active_lineage_attestation(&workspace_current, &contract, &receipt)?;

    let attestation_receipt_path =
        default_active_lineage_attestation_receipt_path(&workspace_current);
    let attestation_receipt_sha256 = sha256_file(&attestation_receipt_path)?;

    let repeated_receipt = build_active_lineage_attestation_receipt(&source_dir, &contract_path)?;
    publish_active_lineage_attestation(&workspace_current, &contract, &repeated_receipt)?;
    let repeated_attestation_receipt_sha256 = sha256_file(&attestation_receipt_path)?;

    let scenario_root =
        PathBuf::from("target/proof_artifacts/slice29_lineage_consumption/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_activation_receipt_rejected, no_receipt_publication_on_missing_activation_receipt) = {
        let scenario_source_dir = scenario_root.join("missing_activation_receipt_source");
        copy_dir(&source_dir, &scenario_source_dir)?;
        fs::remove_file(scenario_source_dir.join("activation_receipt.json"))?;

        let scenario_output_dir = scenario_root.join("missing_activation_receipt_output");
        reset_dir(&scenario_output_dir)?;
        let scenario_contract_path =
            default_active_lineage_consumer_contract_path(&scenario_output_dir);
        write_active_lineage_consumer_contract(&scenario_contract_path, &contract)?;
        let result =
            build_active_lineage_attestation_receipt(&scenario_source_dir, &scenario_contract_path)
                .and_then(|receipt| {
                    publish_active_lineage_attestation(&scenario_output_dir, &contract, &receipt)
                });
        (
            result.is_err(),
            !default_active_lineage_attestation_receipt_path(&scenario_output_dir).exists(),
        )
    };

    let (
        contract_without_admitted_requirement_rejected,
        no_receipt_publication_on_contract_without_admitted_requirement,
    ) = {
        let scenario_output_dir =
            scenario_root.join("contract_without_admitted_requirement_output");
        reset_dir(&scenario_output_dir)?;
        let mut invalid_contract = default_active_lineage_consumer_contract();
        invalid_contract.requires_admitted_lineage = false;
        let scenario_contract_path =
            default_active_lineage_consumer_contract_path(&scenario_output_dir);
        write_active_lineage_consumer_contract(&scenario_contract_path, &invalid_contract)?;
        let result = build_active_lineage_attestation_receipt(&source_dir, &scenario_contract_path)
            .and_then(|receipt| {
                publish_active_lineage_attestation(
                    &scenario_output_dir,
                    &invalid_contract,
                    &receipt,
                )
            });
        (
            result.is_err(),
            !default_active_lineage_attestation_receipt_path(&scenario_output_dir).exists(),
        )
    };

    let (corrupted_supersession_rejected, no_receipt_publication_on_corrupted_supersession) = {
        let scenario_source_dir = scenario_root.join("corrupted_supersession_source");
        copy_dir(&source_dir, &scenario_source_dir)?;
        fs::write(
            scenario_source_dir.join("active_lineage/supersession_chain_receipt.json"),
            b"{\"tampered\":true}",
        )?;

        let scenario_output_dir = scenario_root.join("corrupted_supersession_output");
        reset_dir(&scenario_output_dir)?;
        let scenario_contract_path =
            default_active_lineage_consumer_contract_path(&scenario_output_dir);
        write_active_lineage_consumer_contract(&scenario_contract_path, &contract)?;
        let result =
            build_active_lineage_attestation_receipt(&scenario_source_dir, &scenario_contract_path)
                .and_then(|receipt| {
                    publish_active_lineage_attestation(&scenario_output_dir, &contract, &receipt)
                });
        (
            result.is_err(),
            !default_active_lineage_attestation_receipt_path(&scenario_output_dir).exists(),
        )
    };

    let report = ActiveLineageAttestationReport {
        schema_version: "proof.active-lineage-attestation-report.v1".to_string(),
        attestation_receipt_file_name: file_name_string(&attestation_receipt_path)?,
        attestation_receipt_sha256: attestation_receipt_sha256.clone(),
        repeated_attestation_receipt_sha256: repeated_attestation_receipt_sha256.clone(),
        stable_repeated_attestation_receipt: attestation_receipt_sha256
            == repeated_attestation_receipt_sha256,
        missing_activation_receipt_rejected,
        contract_without_admitted_requirement_rejected,
        corrupted_supersession_rejected,
        no_receipt_publication_on_missing_activation_receipt,
        no_receipt_publication_on_contract_without_admitted_requirement,
        no_receipt_publication_on_corrupted_supersession,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice29_lineage_consumption/lineage_attestation_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_active_lineage_attestation_receipt(&attestation_receipt_path)?;
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

use precomputed_context_core::lineage_activation::{
    build_lineage_activation_receipt, default_lineage_activation_receipt_path,
    default_lineage_activation_source_dir, default_lineage_activation_workspace_current,
    load_lineage_activation_receipt, publish_activated_lineage_state,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct LineageActivationReport {
    schema_version: String,
    activation_receipt_file_name: String,
    activation_receipt_sha256: String,
    repeated_activation_receipt_sha256: String,
    stable_repeated_activation_receipt: bool,
    missing_rehydrate_receipt_rejected: bool,
    corrupted_supersession_rejected: bool,
    missing_repromotion_receipt_rejected: bool,
    no_receipt_publication_on_missing_rehydrate_receipt: bool,
    no_receipt_publication_on_corrupted_supersession: bool,
    no_receipt_publication_on_missing_repromotion_receipt: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_dir = default_lineage_activation_source_dir();
    let workspace_current = default_lineage_activation_workspace_current();

    reset_dir(&workspace_current)?;
    let receipt = build_lineage_activation_receipt(&source_dir)?;
    publish_activated_lineage_state(&source_dir, &workspace_current, &receipt)?;

    let activation_receipt_path = default_lineage_activation_receipt_path(&workspace_current);
    let activation_receipt_sha256 = sha256_file(&activation_receipt_path)?;

    let repeated_receipt = build_lineage_activation_receipt(&source_dir)?;
    publish_activated_lineage_state(&source_dir, &workspace_current, &repeated_receipt)?;
    let repeated_activation_receipt_sha256 = sha256_file(&activation_receipt_path)?;

    let scenario_root =
        PathBuf::from("target/proof_artifacts/slice28_lineage_activation/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_rehydrate_receipt_rejected, no_receipt_publication_on_missing_rehydrate_receipt) = {
        let scenario_dir = scenario_root.join("missing_rehydrate_receipt");
        copy_dir(&source_dir, &scenario_dir)?;
        fs::remove_file(scenario_dir.join("rehydrate_receipt.json"))?;
        let result = build_lineage_activation_receipt(&scenario_dir).and_then(|receipt| {
            publish_activated_lineage_state(&scenario_dir, &scenario_dir, &receipt)
        });
        (
            result.is_err(),
            !default_lineage_activation_receipt_path(&scenario_dir).exists(),
        )
    };

    let (corrupted_supersession_rejected, no_receipt_publication_on_corrupted_supersession) = {
        let scenario_dir = scenario_root.join("corrupted_supersession");
        copy_dir(&source_dir, &scenario_dir)?;
        fs::write(
            scenario_dir.join("lineage_state/supersession_chain_receipt.json"),
            b"{\"tampered\":true}",
        )?;
        let result = build_lineage_activation_receipt(&scenario_dir).and_then(|receipt| {
            publish_activated_lineage_state(&scenario_dir, &scenario_dir, &receipt)
        });
        (
            result.is_err(),
            !default_lineage_activation_receipt_path(&scenario_dir).exists(),
        )
    };

    let (
        missing_repromotion_receipt_rejected,
        no_receipt_publication_on_missing_repromotion_receipt,
    ) = {
        let scenario_dir = scenario_root.join("missing_repromotion_receipt");
        copy_dir(&source_dir, &scenario_dir)?;
        fs::remove_file(scenario_dir.join("lineage_state/re_promotion_receipt.json"))?;
        let result = build_lineage_activation_receipt(&scenario_dir).and_then(|receipt| {
            publish_activated_lineage_state(&scenario_dir, &scenario_dir, &receipt)
        });
        (
            result.is_err(),
            !default_lineage_activation_receipt_path(&scenario_dir).exists(),
        )
    };

    let report = LineageActivationReport {
        schema_version: "proof.lineage-activation-report.v1".to_string(),
        activation_receipt_file_name: file_name_string(&activation_receipt_path)?,
        activation_receipt_sha256: activation_receipt_sha256.clone(),
        repeated_activation_receipt_sha256: repeated_activation_receipt_sha256.clone(),
        stable_repeated_activation_receipt: activation_receipt_sha256
            == repeated_activation_receipt_sha256,
        missing_rehydrate_receipt_rejected,
        corrupted_supersession_rejected,
        missing_repromotion_receipt_rejected,
        no_receipt_publication_on_missing_rehydrate_receipt,
        no_receipt_publication_on_corrupted_supersession,
        no_receipt_publication_on_missing_repromotion_receipt,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice28_lineage_activation/lineage_activation_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_lineage_activation_receipt(&activation_receipt_path)?;
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

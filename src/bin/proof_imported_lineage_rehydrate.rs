use precomputed_context_core::lineage_bundle_intake::default_lineage_bundle_intake_receipt_path;
use precomputed_context_core::lineage_bundle_rehydrate::{
    build_lineage_rehydrate_receipt, default_lineage_rehydrate_receipt_path,
    default_lineage_rehydrate_source_dir, default_lineage_rehydrate_workspace_current,
    load_lineage_rehydrate_receipt, publish_rehydrated_lineage_state,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct LineageRehydrateReport {
    schema_version: String,
    rehydrate_receipt_file_name: String,
    rehydrate_receipt_sha256: String,
    repeated_rehydrate_receipt_sha256: String,
    stable_repeated_rehydrate_receipt: bool,
    missing_intake_receipt_rejected: bool,
    manifest_hash_mismatch_rejected: bool,
    corrupted_supersession_rejected: bool,
    no_receipt_publication_on_missing_intake_receipt: bool,
    no_receipt_publication_on_manifest_hash_mismatch: bool,
    no_receipt_publication_on_corrupted_supersession: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_dir = default_lineage_rehydrate_source_dir();
    let workspace_current = default_lineage_rehydrate_workspace_current();

    reset_dir(&workspace_current)?;
    let receipt = build_lineage_rehydrate_receipt(&source_dir)?;
    publish_rehydrated_lineage_state(&source_dir, &workspace_current, &receipt)?;

    let rehydrate_receipt_path = default_lineage_rehydrate_receipt_path(&workspace_current);
    let rehydrate_receipt_sha256 = sha256_file(&rehydrate_receipt_path)?;

    let repeated_receipt = build_lineage_rehydrate_receipt(&source_dir)?;
    publish_rehydrated_lineage_state(&source_dir, &workspace_current, &repeated_receipt)?;
    let repeated_rehydrate_receipt_sha256 = sha256_file(&rehydrate_receipt_path)?;

    let scenario_root = PathBuf::from("target/proof_artifacts/slice27_lineage_rehydrate/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_intake_receipt_rejected, no_receipt_publication_on_missing_intake_receipt) = {
        let scenario_dir = scenario_root.join("missing_intake_receipt");
        copy_dir(&source_dir, &scenario_dir)?;
        fs::remove_file(default_lineage_bundle_intake_receipt_path(&scenario_dir))?;
        let result = build_lineage_rehydrate_receipt(&scenario_dir).and_then(|receipt| {
            publish_rehydrated_lineage_state(&scenario_dir, &scenario_dir, &receipt)
        });
        (
            result.is_err(),
            !default_lineage_rehydrate_receipt_path(&scenario_dir).exists(),
        )
    };

    let (manifest_hash_mismatch_rejected, no_receipt_publication_on_manifest_hash_mismatch) = {
        let scenario_dir = scenario_root.join("manifest_hash_mismatch");
        copy_dir(&source_dir, &scenario_dir)?;
        let receipt_path = default_lineage_bundle_intake_receipt_path(&scenario_dir);
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
        value["manifest_sha256"] = serde_json::Value::String("0".repeat(64));
        fs::write(&receipt_path, serde_json::to_vec_pretty(&value)?)?;
        let result = build_lineage_rehydrate_receipt(&scenario_dir).and_then(|receipt| {
            publish_rehydrated_lineage_state(&scenario_dir, &scenario_dir, &receipt)
        });
        (
            result.is_err(),
            !default_lineage_rehydrate_receipt_path(&scenario_dir).exists(),
        )
    };

    let (corrupted_supersession_rejected, no_receipt_publication_on_corrupted_supersession) = {
        let scenario_dir = scenario_root.join("corrupted_supersession");
        copy_dir(&source_dir, &scenario_dir)?;
        fs::write(
            scenario_dir.join("bundle/supersession_chain_receipt.json"),
            b"{\"tampered\":true}",
        )?;
        let result = build_lineage_rehydrate_receipt(&scenario_dir).and_then(|receipt| {
            publish_rehydrated_lineage_state(&scenario_dir, &scenario_dir, &receipt)
        });
        (
            result.is_err(),
            !default_lineage_rehydrate_receipt_path(&scenario_dir).exists(),
        )
    };

    let report = LineageRehydrateReport {
        schema_version: "proof.lineage-rehydrate-report.v1".to_string(),
        rehydrate_receipt_file_name: file_name_string(&rehydrate_receipt_path)?,
        rehydrate_receipt_sha256: rehydrate_receipt_sha256.clone(),
        repeated_rehydrate_receipt_sha256: repeated_rehydrate_receipt_sha256.clone(),
        stable_repeated_rehydrate_receipt: rehydrate_receipt_sha256
            == repeated_rehydrate_receipt_sha256,
        missing_intake_receipt_rejected,
        manifest_hash_mismatch_rejected,
        corrupted_supersession_rejected,
        no_receipt_publication_on_missing_intake_receipt,
        no_receipt_publication_on_manifest_hash_mismatch,
        no_receipt_publication_on_corrupted_supersession,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice27_lineage_rehydrate/lineage_rehydrate_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_lineage_rehydrate_receipt(&rehydrate_receipt_path)?;
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

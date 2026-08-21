use precomputed_context_core::lineage_bundle_intake::{
    build_lineage_bundle_intake_receipt, default_intaken_bundle_dir,
    default_lineage_bundle_intake_receipt_path, default_lineage_bundle_intake_source_dir,
    default_lineage_bundle_intake_workspace_current, load_lineage_bundle_intake_receipt,
    publish_intaken_lineage_bundle,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct LineageBundleIntakeReport {
    schema_version: String,
    intake_receipt_file_name: String,
    intake_receipt_sha256: String,
    repeated_intake_receipt_sha256: String,
    stable_repeated_intake_receipt: bool,
    missing_envelope_rejected: bool,
    extra_member_rejected: bool,
    member_sha_mismatch_rejected: bool,
    no_receipt_publication_on_missing_envelope: bool,
    no_receipt_publication_on_extra_member: bool,
    no_receipt_publication_on_member_sha_mismatch: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let source_bundle_dir = default_lineage_bundle_intake_source_dir();
    let workspace_current = default_lineage_bundle_intake_workspace_current();

    reset_dir(&workspace_current)?;
    let receipt = build_lineage_bundle_intake_receipt(&source_bundle_dir)?;
    publish_intaken_lineage_bundle(&source_bundle_dir, &workspace_current, &receipt)?;

    let intake_receipt_path = default_lineage_bundle_intake_receipt_path(&workspace_current);
    let intake_receipt_sha256 = sha256_file(&intake_receipt_path)?;

    let repeated_receipt = build_lineage_bundle_intake_receipt(&source_bundle_dir)?;
    publish_intaken_lineage_bundle(&source_bundle_dir, &workspace_current, &repeated_receipt)?;
    let repeated_intake_receipt_sha256 = sha256_file(&intake_receipt_path)?;

    let scenario_root =
        PathBuf::from("target/proof_artifacts/slice26_lineage_bundle_intake/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_envelope_rejected, no_receipt_publication_on_missing_envelope) = {
        let scenario_dir = scenario_root.join("missing_envelope");
        copy_dir(&source_bundle_dir, &scenario_dir)?;
        fs::remove_file(scenario_dir.join("lineage_bundle_envelope.json"))?;
        let receipt_path = default_lineage_bundle_intake_receipt_path(&scenario_dir);
        let result = build_lineage_bundle_intake_receipt(&scenario_dir).and_then(|receipt| {
            publish_intaken_lineage_bundle(&scenario_dir, &scenario_dir, &receipt)
        });
        (result.is_err(), !receipt_path.exists())
    };

    let (extra_member_rejected, no_receipt_publication_on_extra_member) = {
        let scenario_dir = scenario_root.join("extra_member");
        copy_dir(&source_bundle_dir, &scenario_dir)?;
        fs::write(scenario_dir.join("rogue.txt"), b"rogue")?;
        let receipt_path = default_lineage_bundle_intake_receipt_path(&scenario_dir);
        let result = build_lineage_bundle_intake_receipt(&scenario_dir).and_then(|receipt| {
            publish_intaken_lineage_bundle(&scenario_dir, &scenario_dir, &receipt)
        });
        (result.is_err(), !receipt_path.exists())
    };

    let (member_sha_mismatch_rejected, no_receipt_publication_on_member_sha_mismatch) = {
        let scenario_dir = scenario_root.join("member_sha_mismatch");
        copy_dir(&source_bundle_dir, &scenario_dir)?;
        fs::write(scenario_dir.join("promotion_receipt.json"), b"tampered")?;
        let receipt_path = default_lineage_bundle_intake_receipt_path(&scenario_dir);
        let result = build_lineage_bundle_intake_receipt(&scenario_dir).and_then(|receipt| {
            publish_intaken_lineage_bundle(&scenario_dir, &scenario_dir, &receipt)
        });
        (result.is_err(), !receipt_path.exists())
    };

    let report = LineageBundleIntakeReport {
        schema_version: "proof.lineage-bundle-intake-report.v1".to_string(),
        intake_receipt_file_name: file_name_string(&intake_receipt_path)?,
        intake_receipt_sha256: intake_receipt_sha256.clone(),
        repeated_intake_receipt_sha256: repeated_intake_receipt_sha256.clone(),
        stable_repeated_intake_receipt: intake_receipt_sha256 == repeated_intake_receipt_sha256,
        missing_envelope_rejected,
        extra_member_rejected,
        member_sha_mismatch_rejected,
        no_receipt_publication_on_missing_envelope,
        no_receipt_publication_on_extra_member,
        no_receipt_publication_on_member_sha_mismatch,
    };

    let report_path = PathBuf::from(
        "target/proof_artifacts/slice26_lineage_bundle_intake/lineage_bundle_intake_report.json",
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_lineage_bundle_intake_receipt(&intake_receipt_path)?;
    let _ = default_intaken_bundle_dir(&workspace_current);
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

use precomputed_context_core::consumer_acknowledgment::{
    build_consumer_acknowledgment_receipt, build_return_channel_closure_receipt,
    default_consumer_acknowledgment_receipt_path, default_consumer_acknowledgment_source_dir,
    default_consumer_acknowledgment_workspace_current, default_return_channel_closure_receipt_path,
    load_consumer_acknowledgment_receipt, load_return_channel_closure_receipt,
    publish_consumer_acknowledgment, publish_return_channel_closure,
};
use precomputed_context_core::consumer_handoff::default_bounded_consumer_handoff_package_dir;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct ConsumerAcknowledgmentReport {
    schema_version: String,
    acknowledgment_receipt_sha256: String,
    repeated_acknowledgment_receipt_sha256: String,
    stable_repeated_acknowledgment_receipt: bool,
    closure_receipt_sha256: String,
    repeated_closure_receipt_sha256: String,
    stable_repeated_closure_receipt: bool,
    missing_handoff_receipt_rejected: bool,
    bounded_package_tamper_rejected: bool,
    acknowledgment_continuity_tamper_rejected: bool,
    no_ack_publication_on_missing_handoff_receipt: bool,
    no_ack_publication_on_package_tamper: bool,
    no_closure_publication_on_acknowledgment_continuity_tamper: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let handoff_source = default_consumer_acknowledgment_source_dir();
    let workspace_current = default_consumer_acknowledgment_workspace_current();
    reset_dir(&workspace_current)?;

    let acknowledgment = build_consumer_acknowledgment_receipt(&handoff_source)?;
    publish_consumer_acknowledgment(&workspace_current, &acknowledgment)?;
    let closure = build_return_channel_closure_receipt(&handoff_source, &workspace_current)?;
    publish_return_channel_closure(&workspace_current, &closure)?;

    let acknowledgment_receipt_path =
        default_consumer_acknowledgment_receipt_path(&workspace_current);
    let closure_receipt_path = default_return_channel_closure_receipt_path(&workspace_current);
    let acknowledgment_receipt_sha256 = sha256_file(&acknowledgment_receipt_path)?;
    let closure_receipt_sha256 = sha256_file(&closure_receipt_path)?;

    let repeated_ack = build_consumer_acknowledgment_receipt(&handoff_source)?;
    publish_consumer_acknowledgment(&workspace_current, &repeated_ack)?;
    let repeated_closure =
        build_return_channel_closure_receipt(&handoff_source, &workspace_current)?;
    publish_return_channel_closure(&workspace_current, &repeated_closure)?;
    let repeated_acknowledgment_receipt_sha256 = sha256_file(&acknowledgment_receipt_path)?;
    let repeated_closure_receipt_sha256 = sha256_file(&closure_receipt_path)?;

    let scenario_root =
        PathBuf::from("target/proof_artifacts/slice31_consumer_acknowledgment/scenarios");
    reset_dir(&scenario_root)?;

    let (missing_handoff_receipt_rejected, no_ack_publication_on_missing_handoff_receipt) = {
        let scenario_handoff = scenario_root.join("missing_handoff_receipt_source");
        copy_dir(&handoff_source, &scenario_handoff)?;
        fs::remove_file(scenario_handoff.join("handoff_receipt.json"))?;
        let scenario_output = scenario_root.join("missing_handoff_receipt_output");
        reset_dir(&scenario_output)?;
        let result = build_consumer_acknowledgment_receipt(&scenario_handoff)
            .and_then(|ack| publish_consumer_acknowledgment(&scenario_output, &ack));
        (
            result.is_err(),
            !default_consumer_acknowledgment_receipt_path(&scenario_output).exists(),
        )
    };

    let (bounded_package_tamper_rejected, no_ack_publication_on_package_tamper) = {
        let scenario_handoff = scenario_root.join("package_tamper_source");
        copy_dir(&handoff_source, &scenario_handoff)?;
        fs::write(
            default_bounded_consumer_handoff_package_dir(&scenario_handoff).join("extra.txt"),
            b"tamper",
        )?;
        let scenario_output = scenario_root.join("package_tamper_output");
        reset_dir(&scenario_output)?;
        let result = build_consumer_acknowledgment_receipt(&scenario_handoff)
            .and_then(|ack| publish_consumer_acknowledgment(&scenario_output, &ack));
        (
            result.is_err(),
            !default_consumer_acknowledgment_receipt_path(&scenario_output).exists(),
        )
    };

    let (
        acknowledgment_continuity_tamper_rejected,
        no_closure_publication_on_acknowledgment_continuity_tamper,
    ) = {
        let scenario_output = scenario_root.join("acknowledgment_continuity_tamper_output");
        reset_dir(&scenario_output)?;
        let ack = build_consumer_acknowledgment_receipt(&handoff_source)?;
        publish_consumer_acknowledgment(&scenario_output, &ack)?;
        let ack_path = default_consumer_acknowledgment_receipt_path(&scenario_output);
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&ack_path)?)?;
        value["handoff_receipt_sha256"] = serde_json::Value::String("tampered".to_string());
        fs::write(&ack_path, serde_json::to_vec_pretty(&value)?)?;
        let result = build_return_channel_closure_receipt(&handoff_source, &scenario_output)
            .and_then(|closure| publish_return_channel_closure(&scenario_output, &closure));
        (
            result.is_err(),
            !default_return_channel_closure_receipt_path(&scenario_output).exists(),
        )
    };

    let report = ConsumerAcknowledgmentReport {
        schema_version: "proof.consumer-acknowledgment-report.v1".to_string(),
        acknowledgment_receipt_sha256: acknowledgment_receipt_sha256.clone(),
        repeated_acknowledgment_receipt_sha256: repeated_acknowledgment_receipt_sha256.clone(),
        stable_repeated_acknowledgment_receipt: acknowledgment_receipt_sha256
            == repeated_acknowledgment_receipt_sha256,
        closure_receipt_sha256: closure_receipt_sha256.clone(),
        repeated_closure_receipt_sha256: repeated_closure_receipt_sha256.clone(),
        stable_repeated_closure_receipt: closure_receipt_sha256 == repeated_closure_receipt_sha256,
        missing_handoff_receipt_rejected,
        bounded_package_tamper_rejected,
        acknowledgment_continuity_tamper_rejected,
        no_ack_publication_on_missing_handoff_receipt,
        no_ack_publication_on_package_tamper,
        no_closure_publication_on_acknowledgment_continuity_tamper,
    };

    let report_path =
        PathBuf::from("target/proof_artifacts/slice31_consumer_acknowledgment/consumer_acknowledgment_report.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    let _ = load_consumer_acknowledgment_receipt(&acknowledgment_receipt_path)?;
    let _ = load_return_channel_closure_receipt(&closure_receipt_path)?;
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

use precomputed_context_core::import_policy::{
    default_import_policy_path, hash_import_authorization_policy_file,
    write_default_import_authorization_policy,
};
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct PolicySurfaceReport {
    schema_version: String,
    policy_file_name: String,
    policy_sha256: String,
    repeated_policy_sha256: String,
    stable_repeated_policy: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let policy_path = write_default_import_authorization_policy()?;
    let first_hash = hash_import_authorization_policy_file(&policy_path)?;

    write_default_import_authorization_policy()?;
    let second_hash = hash_import_authorization_policy_file(&policy_path)?;

    let report = PolicySurfaceReport {
        schema_version: "proof.policy-surface-report.v1".to_string(),
        policy_file_name: file_name_string(&policy_path)?,
        policy_sha256: first_hash.clone(),
        repeated_policy_sha256: second_hash.clone(),
        stable_repeated_policy: first_hash == second_hash,
    };

    let report_path =
        PathBuf::from("target/proof_artifacts/slice19_policy/policy_surface_report.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;

    println!("{}", default_import_policy_path().display());
    Ok(())
}

fn file_name_string(path: &std::path::Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .file_name()
        .ok_or("path has no file name")?
        .to_string_lossy()
        .to_string())
}

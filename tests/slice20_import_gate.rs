use precomputed_context_core::authorization_evidence::{
    build_authorization_evidence_link, write_authorization_evidence_link,
};
use precomputed_context_core::import_authorization::{
    authorize_zip_import_from_policy_file, write_authorization_receipt,
};
use precomputed_context_core::import_gate::validate_rehydrate_gate;
use precomputed_context_core::import_policy::{
    default_import_authorization_policy, write_import_authorization_policy,
};
use precomputed_context_core::trust_envelope::{
    build_signed_trust_envelope_for_zip, default_proof_signer, write_signed_trust_envelope,
};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn valid_authorization_artifacts_allow_rehydrate_gate() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice20_gate_ok")?;
    let bundle = make_valid_bundle(&root)?;

    let receipt = validate_rehydrate_gate(
        &bundle.zip_path,
        &bundle.policy_path,
        &bundle.envelope_path,
        &bundle.authorization_receipt_path,
        &bundle.evidence_link_path,
        &bundle.import_receipt_path,
    )?;

    assert!(receipt.authorized);
    assert_eq!(receipt.decision, "authorized_rehydrate_publication");
    Ok(())
}

#[test]
fn evidence_hash_mismatch_fails_closed() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice20_gate_bad_evidence")?;
    let bundle = make_valid_bundle(&root)?;

    let bad_evidence_path = root.join("bad_authorization_evidence_link.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&bundle.evidence_link_path)?)?;
    value["import_receipt_sha256"] = serde_json::Value::String("0".repeat(64));
    fs::write(&bad_evidence_path, serde_json::to_vec_pretty(&value)?)?;

    let error = validate_rehydrate_gate(
        &bundle.zip_path,
        &bundle.policy_path,
        &bundle.envelope_path,
        &bundle.authorization_receipt_path,
        &bad_evidence_path,
        &bundle.import_receipt_path,
    )
    .expect_err("tampered evidence must fail closed");

    let message = error.to_string();
    assert!(message.contains("mismatch"));
    Ok(())
}

struct ValidBundlePaths {
    zip_path: PathBuf,
    policy_path: PathBuf,
    envelope_path: PathBuf,
    authorization_receipt_path: PathBuf,
    evidence_link_path: PathBuf,
    import_receipt_path: PathBuf,
}

fn make_valid_bundle(root: &Path) -> Result<ValidBundlePaths, Box<dyn Error>> {
    let zip_path = root.join("package.zip");
    let sha_path = root.join("package.zip.sha256");
    let policy_path = root.join("import_authorization_policy.json");
    let envelope_path = root.join("package.zip.trust_envelope.json");
    let authorization_receipt_path = root.join("authorization_receipt.json");
    let evidence_link_path = root.join("authorization_evidence_link.json");
    let import_receipt_path = root.join("import_receipt.json");

    fs::write(&zip_path, b"slice20 gated package bytes")?;
    fs::write(
        &sha_path,
        format!("{}  package.zip\n", sha256_hex(fs::read(&zip_path)?)),
    )?;
    fs::write(&import_receipt_path, b"{\"import\":\"ok\"}")?;

    let policy = default_import_authorization_policy();
    write_import_authorization_policy(&policy_path, &policy)?;

    let envelope =
        build_signed_trust_envelope_for_zip(&zip_path, &sha_path, default_proof_signer())?;
    write_signed_trust_envelope(&envelope_path, &envelope)?;

    let receipt =
        authorize_zip_import_from_policy_file(&zip_path, Some(&envelope_path), &policy_path)?;
    write_authorization_receipt(&authorization_receipt_path, &receipt)?;

    let evidence = build_authorization_evidence_link(
        &policy_path,
        &envelope_path,
        &authorization_receipt_path,
        Some(&import_receipt_path),
    )?;
    write_authorization_evidence_link(&evidence_link_path, &evidence)?;

    Ok(ValidBundlePaths {
        zip_path,
        policy_path,
        envelope_path,
        authorization_receipt_path,
        evidence_link_path,
        import_receipt_path,
    })
}

fn unique_temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn Error>> {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    path.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn sha256_hex(bytes: Vec<u8>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{:02x}", byte);
    }
    output
}

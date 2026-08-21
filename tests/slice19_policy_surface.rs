use precomputed_context_core::authorization_evidence::build_authorization_evidence_link;
use precomputed_context_core::import_authorization::authorize_zip_import_from_policy_file;
use precomputed_context_core::import_policy::{
    default_import_authorization_policy, hash_import_authorization_policy_file,
    write_import_authorization_policy,
};
use precomputed_context_core::trust_envelope::{
    build_signed_trust_envelope_for_zip, default_proof_signer, write_signed_trust_envelope,
};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[test]
fn policy_file_hash_is_stable_after_rewrite() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice19_policy_hash")?;
    let policy_path = root.join("import_authorization_policy.json");
    let policy = default_import_authorization_policy();

    write_import_authorization_policy(&policy_path, &policy)?;
    let first = hash_import_authorization_policy_file(&policy_path)?;
    write_import_authorization_policy(&policy_path, &policy)?;
    let second = hash_import_authorization_policy_file(&policy_path)?;

    assert_eq!(first, second);
    Ok(())
}

#[test]
fn policy_without_allowed_signer_fails_closed() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice19_policy_reject")?;
    let zip_path = root.join("package.zip");
    let sha_path = root.join("package.zip.sha256");
    let envelope_path = root.join("package.zip.trust_envelope.json");
    let policy_path = root.join("policy_without_signer.json");

    fs::write(&zip_path, b"slice19 package bytes")?;
    fs::write(
        &sha_path,
        format!("{}  package.zip\n", sha256_hex(fs::read(&zip_path)?)),
    )?;

    let envelope =
        build_signed_trust_envelope_for_zip(&zip_path, &sha_path, default_proof_signer())?;
    write_signed_trust_envelope(&envelope_path, &envelope)?;

    let mut policy = default_import_authorization_policy();
    policy.signer_rules.clear();
    write_import_authorization_policy(&policy_path, &policy)?;

    let error =
        authorize_zip_import_from_policy_file(&zip_path, Some(&envelope_path), &policy_path)
            .expect_err("policy without signer must fail closed");
    assert!(error.to_string().contains("not authorized"));
    Ok(())
}

#[test]
fn evidence_link_binds_policy_envelope_and_receipt() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice19_evidence")?;
    let policy_path = root.join("policy.json");
    let envelope_path = root.join("envelope.json");
    let authorization_receipt_path = root.join("authorization_receipt.json");
    let import_receipt_path = root.join("import_receipt.json");

    fs::write(&policy_path, b"{\"policy\":\"ok\"}")?;
    fs::write(&envelope_path, b"{\"envelope\":\"ok\"}")?;
    fs::write(&authorization_receipt_path, b"{\"receipt\":\"ok\"}")?;
    fs::write(&import_receipt_path, b"{\"import\":\"ok\"}")?;

    let evidence = build_authorization_evidence_link(
        &policy_path,
        &envelope_path,
        &authorization_receipt_path,
        Some(&import_receipt_path),
    )?;

    assert!(evidence.import_receipt_present);
    assert_eq!(evidence.policy_file_name, "policy.json");
    assert_eq!(
        evidence.authorization_receipt_file_name,
        "authorization_receipt.json"
    );
    Ok(())
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

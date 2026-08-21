use precomputed_context_core::import_authorization::{
    authorize_zip_import, default_import_authorization_policy,
};
use precomputed_context_core::trust_envelope::{
    build_signed_trust_envelope_for_zip, default_proof_signer, rogue_proof_signer,
    verify_signed_trust_envelope, write_signed_trust_envelope,
};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[test]
fn signed_trust_envelope_verifies_and_authorizes_for_trusted_signer() -> Result<(), Box<dyn Error>>
{
    let root = unique_temp_dir("slice18_trust_ok")?;
    let zip_path = root.join("package.zip");
    let sha_path = root.join("package.zip.sha256");
    let envelope_path = root.join("package.zip.trust_envelope.json");

    fs::write(&zip_path, b"slice18 package bytes")?;
    fs::write(
        &sha_path,
        format!("{}  package.zip\n", sha256_hex(fs::read(&zip_path)?)),
    )?;

    let envelope =
        build_signed_trust_envelope_for_zip(&zip_path, &sha_path, default_proof_signer())?;
    verify_signed_trust_envelope(&envelope)?;
    write_signed_trust_envelope(&envelope_path, &envelope)?;

    let receipt = authorize_zip_import(
        &zip_path,
        Some(&envelope_path),
        &default_import_authorization_policy(),
    )?;
    assert!(receipt.authorized);
    assert_eq!(receipt.signer_id, "slice18-proof-signer-v1");
    Ok(())
}

#[test]
fn rogue_signer_is_rejected_by_import_policy() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice18_trust_rogue")?;
    let zip_path = root.join("package.zip");
    let sha_path = root.join("package.zip.sha256");
    let envelope_path = root.join("package.zip.trust_envelope.json");

    fs::write(&zip_path, b"slice18 package bytes")?;
    fs::write(
        &sha_path,
        format!("{}  package.zip\n", sha256_hex(fs::read(&zip_path)?)),
    )?;

    let envelope = build_signed_trust_envelope_for_zip(&zip_path, &sha_path, rogue_proof_signer())?;
    write_signed_trust_envelope(&envelope_path, &envelope)?;

    let error = authorize_zip_import(
        &zip_path,
        Some(&envelope_path),
        &default_import_authorization_policy(),
    )
    .expect_err("rogue signer must be rejected");
    assert!(error.to_string().contains("not authorized"));
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

use precomputed_context_core::authorization_evidence::{
    build_authorization_evidence_link, write_authorization_evidence_link,
};
use precomputed_context_core::import_authorization::{
    authorize_zip_import_from_policy_file, write_authorization_receipt,
};
use precomputed_context_core::import_gate::{
    publish_authorized_import_receipt, validate_rehydrate_gate,
};
use precomputed_context_core::import_policy::{
    default_import_authorization_policy, write_import_authorization_policy,
};
use precomputed_context_core::promotion_gate::{
    build_promotion_approval, validate_promotion_approval, write_promotion_approval,
};
use precomputed_context_core::trust_envelope::{
    build_signed_trust_envelope_for_zip, default_proof_signer, write_signed_trust_envelope,
};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[test]
fn valid_approval_allows_promotion() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice21_promotion_ok")?;
    let prepared = make_prepared_gate_surface(&root)?;

    let approval_path = root.join("operator_approval.json");
    let approval = build_promotion_approval(
        &prepared.gate_receipt_path,
        &prepared.gated_import_receipt_path,
    )?;
    write_promotion_approval(&approval_path, &approval)?;

    let receipt = validate_promotion_approval(
        &prepared.gate_receipt_path,
        &prepared.gated_import_receipt_path,
        &approval_path,
    )?;

    assert!(receipt.approved);
    assert_eq!(receipt.decision, "approved_promotion");
    Ok(())
}

#[test]
fn gate_hash_mismatch_fails_closed() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice21_promotion_bad_hash")?;
    let prepared = make_prepared_gate_surface(&root)?;

    let approval_path = root.join("operator_approval.json");
    let mut approval = build_promotion_approval(
        &prepared.gate_receipt_path,
        &prepared.gated_import_receipt_path,
    )?;
    approval.gate_receipt_sha256 = "0".repeat(64);
    write_promotion_approval(&approval_path, &approval)?;

    let error = validate_promotion_approval(
        &prepared.gate_receipt_path,
        &prepared.gated_import_receipt_path,
        &approval_path,
    )
    .expect_err("gate hash mismatch must fail closed");

    assert!(error.to_string().contains("mismatch"));
    Ok(())
}

struct PreparedGateSurface {
    gate_receipt_path: PathBuf,
    gated_import_receipt_path: PathBuf,
}

fn make_prepared_gate_surface(root: &PathBuf) -> Result<PreparedGateSurface, Box<dyn Error>> {
    let zip_path = root.join("package.zip");
    let sha_path = root.join("package.zip.sha256");
    let policy_path = root.join("import_authorization_policy.json");
    let envelope_path = root.join("package.zip.trust_envelope.json");
    let authorization_receipt_path = root.join("authorization_receipt.json");
    let evidence_link_path = root.join("authorization_evidence_link.json");
    let import_receipt_path = root.join("import_receipt.json");
    let gated_workspace = root.join("gated_workspace");

    fs::write(&zip_path, b"slice21 promotion package bytes")?;
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

    let authorization_receipt =
        authorize_zip_import_from_policy_file(&zip_path, Some(&envelope_path), &policy_path)?;
    write_authorization_receipt(&authorization_receipt_path, &authorization_receipt)?;

    let evidence = build_authorization_evidence_link(
        &policy_path,
        &envelope_path,
        &authorization_receipt_path,
        Some(&import_receipt_path),
    )?;
    write_authorization_evidence_link(&evidence_link_path, &evidence)?;

    let gate_receipt = validate_rehydrate_gate(
        &zip_path,
        &policy_path,
        &envelope_path,
        &authorization_receipt_path,
        &evidence_link_path,
        &import_receipt_path,
    )?;
    publish_authorized_import_receipt(&import_receipt_path, &gated_workspace, &gate_receipt)?;

    Ok(PreparedGateSurface {
        gate_receipt_path: gated_workspace.join("gate_receipt.json"),
        gated_import_receipt_path: gated_workspace.join("import_receipt.json"),
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

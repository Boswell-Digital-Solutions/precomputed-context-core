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
    build_promotion_approval, publish_promoted_import_receipt, validate_promotion_approval,
    write_promotion_approval,
};
use precomputed_context_core::promotion_revocation::{
    build_revocation_request, validate_revocation_request, write_revocation_request,
};
use precomputed_context_core::trust_envelope::{
    build_signed_trust_envelope_for_zip, default_proof_signer, write_signed_trust_envelope,
};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn valid_revocation_allows_rollback() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice22_revocation_ok")?;
    let prepared = make_prepared_promotion_surface(&root)?;

    let revocation_request_path = root.join("operator_revocation.json");
    let request = build_revocation_request(
        &prepared.promotion_receipt_path,
        &prepared.promoted_import_receipt_path,
    )?;
    write_revocation_request(&revocation_request_path, &request)?;

    let receipt = validate_revocation_request(
        &prepared.promotion_receipt_path,
        &prepared.promoted_import_receipt_path,
        &revocation_request_path,
    )?;

    assert!(receipt.revoked);
    assert_eq!(receipt.decision, "revoked_and_rolled_back");
    Ok(())
}

#[test]
fn promotion_hash_mismatch_fails_closed() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice22_revocation_bad_hash")?;
    let prepared = make_prepared_promotion_surface(&root)?;

    let revocation_request_path = root.join("operator_revocation.json");
    let mut request = build_revocation_request(
        &prepared.promotion_receipt_path,
        &prepared.promoted_import_receipt_path,
    )?;
    request.promotion_receipt_sha256 = "0".repeat(64);
    write_revocation_request(&revocation_request_path, &request)?;

    let error = validate_revocation_request(
        &prepared.promotion_receipt_path,
        &prepared.promoted_import_receipt_path,
        &revocation_request_path,
    )
    .expect_err("promotion hash mismatch must fail closed");

    assert!(error.to_string().contains("mismatch"));
    Ok(())
}

struct PreparedPromotionSurface {
    promotion_receipt_path: PathBuf,
    promoted_import_receipt_path: PathBuf,
}

fn make_prepared_promotion_surface(
    root: &Path,
) -> Result<PreparedPromotionSurface, Box<dyn Error>> {
    let zip_path = root.join("package.zip");
    let sha_path = root.join("package.zip.sha256");
    let policy_path = root.join("import_authorization_policy.json");
    let envelope_path = root.join("package.zip.trust_envelope.json");
    let authorization_receipt_path = root.join("authorization_receipt.json");
    let evidence_link_path = root.join("authorization_evidence_link.json");
    let import_receipt_path = root.join("import_receipt.json");
    let gated_workspace = root.join("gated_workspace");
    let promotion_workspace = root.join("promotion_workspace");
    let approval_path = root.join("operator_approval.json");

    fs::write(&zip_path, b"slice22 revocation package bytes")?;
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

    let approval = build_promotion_approval(
        &gated_workspace.join("gate_receipt.json"),
        &gated_workspace.join("import_receipt.json"),
    )?;
    write_promotion_approval(&approval_path, &approval)?;

    let promotion_receipt = validate_promotion_approval(
        &gated_workspace.join("gate_receipt.json"),
        &gated_workspace.join("import_receipt.json"),
        &approval_path,
    )?;
    publish_promoted_import_receipt(
        &gated_workspace.join("import_receipt.json"),
        &promotion_workspace,
        &promotion_receipt,
    )?;

    Ok(PreparedPromotionSurface {
        promotion_receipt_path: promotion_workspace.join("promotion_receipt.json"),
        promoted_import_receipt_path: promotion_workspace.join("import_receipt.json"),
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

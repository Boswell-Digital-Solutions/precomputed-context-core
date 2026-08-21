use precomputed_context_core::authorization_evidence::{
    build_authorization_evidence_link, write_authorization_evidence_link,
};
use precomputed_context_core::consumer_acknowledgment::{
    build_consumer_acknowledgment_receipt, build_return_channel_closure_receipt,
    publish_consumer_acknowledgment, publish_return_channel_closure,
};
use precomputed_context_core::consumer_handoff::{
    build_bounded_consumer_handoff_receipt, publish_bounded_consumer_handoff,
};
use precomputed_context_core::downstream_release::{
    build_downstream_release_receipt, publish_downstream_release_package,
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
use precomputed_context_core::lineage_activation::{
    build_lineage_activation_receipt, publish_activated_lineage_state,
};
use precomputed_context_core::lineage_bundle::{publish_lineage_bundle, LineageBundleSourcePaths};
use precomputed_context_core::lineage_bundle_intake::{
    build_lineage_bundle_intake_receipt, publish_intaken_lineage_bundle,
};
use precomputed_context_core::lineage_bundle_rehydrate::{
    build_lineage_rehydrate_receipt, publish_rehydrated_lineage_state,
};
use precomputed_context_core::lineage_consumption::{
    build_active_lineage_attestation_receipt, default_active_lineage_consumer_contract,
    publish_active_lineage_attestation, write_active_lineage_consumer_contract,
};
use precomputed_context_core::promotion_gate::{
    build_promotion_approval, publish_promoted_import_receipt, validate_promotion_approval,
    write_promotion_approval,
};
use precomputed_context_core::promotion_revocation::{
    build_revocation_request, publish_rollback_state, validate_revocation_request,
    write_revocation_request,
};
use precomputed_context_core::re_promotion::{
    build_repromotion_approval, publish_repromoted_import_receipt, validate_repromotion_approval,
    write_repromotion_approval,
};
use precomputed_context_core::release_readiness::{
    build_operator_release_summary, build_release_readiness_receipt, publish_release_readiness,
};
use precomputed_context_core::supersession_chain::{
    build_supersession_chain_receipt, publish_supersession_chain,
};
use precomputed_context_core::trust_envelope::{
    build_signed_trust_envelope_for_zip, default_proof_signer, write_signed_trust_envelope,
};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn valid_release_readiness_builds_and_publishes() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice33_release_readiness_ok")?;
    let release_dir = make_prepared_release_workspace(&root)?;
    let output_dir = root.join("readiness_workspace");

    let receipt = build_release_readiness_receipt(&release_dir)?;
    let summary = build_operator_release_summary(&release_dir)?;
    publish_release_readiness(&output_dir, &receipt, &summary)?;

    assert!(output_dir.join("release_readiness_receipt.json").exists());
    assert!(output_dir.join("operator_release_summary.json").exists());
    Ok(())
}

#[test]
fn release_receipt_continuity_tamper_fails_closed() -> Result<(), Box<dyn Error>> {
    let root = unique_temp_dir("slice33_release_readiness_tamper")?;
    let release_dir = make_prepared_release_workspace(&root)?;

    let receipt_path = release_dir.join("downstream_release_receipt.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
    value["closure_receipt_sha256"] = serde_json::Value::String("tampered".to_string());
    fs::write(&receipt_path, serde_json::to_vec_pretty(&value)?)?;

    let error = build_release_readiness_receipt(&release_dir)
        .expect_err("receipt continuity tamper must fail closed");
    assert!(
        error
            .to_string()
            .contains("downstream release receipt closure continuity mismatch"),
        "unexpected error message: {}",
        error
    );
    Ok(())
}

fn make_prepared_release_workspace(root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let (handoff_dir, ack_dir) = make_prepared_ack_workspace(root)?;
    let release_dir = root.join("downstream_release/current");

    let receipt = build_downstream_release_receipt(&handoff_dir, &ack_dir)?;
    publish_downstream_release_package(&handoff_dir, &ack_dir, &release_dir, &receipt)?;
    Ok(release_dir)
}

fn make_prepared_ack_workspace(root: &Path) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let (activation_workspace, consumption_workspace) = make_prepared_consumption_workspace(root)?;
    let handoff_workspace = root.join("handoff_workspace/current");
    let ack_workspace = root.join("ack_workspace/current");

    let handoff_receipt =
        build_bounded_consumer_handoff_receipt(&activation_workspace, &consumption_workspace)?;
    publish_bounded_consumer_handoff(
        &activation_workspace,
        &consumption_workspace,
        &handoff_workspace,
        &handoff_receipt,
    )?;

    let acknowledgment = build_consumer_acknowledgment_receipt(&handoff_workspace)?;
    publish_consumer_acknowledgment(&ack_workspace, &acknowledgment)?;
    let closure = build_return_channel_closure_receipt(&handoff_workspace, &ack_workspace)?;
    publish_return_channel_closure(&ack_workspace, &closure)?;

    Ok((handoff_workspace, ack_workspace))
}

fn make_prepared_consumption_workspace(root: &Path) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let zip_path = root.join("package.zip");
    let sha_path = root.join("package.zip.sha256");
    let policy_path = root.join("import_authorization_policy.json");
    let envelope_path = root.join("package.zip.trust_envelope.json");
    let authorization_receipt_path = root.join("authorization_receipt.json");
    let evidence_link_path = root.join("authorization_evidence_link.json");
    let import_receipt_path = root.join("import_receipt.json");
    let gated_workspace = root.join("gated_workspace");
    let promotion_workspace = root.join("promotion_workspace");
    let revocation_workspace = root.join("revocation_workspace");
    let repromotion_workspace = root.join("repromotion_workspace");
    let supersession_workspace = root.join("supersession_workspace");
    let bundle_workspace = root.join("bundle_workspace/current");
    let intake_workspace = root.join("intake_workspace/current");
    let rehydrate_workspace = root.join("rehydrate_workspace/current");
    let activation_workspace = root.join("activation_workspace/current");
    let consumption_workspace = root.join("consumption_workspace/current");
    let approval_path = root.join("operator_approval.json");
    let revocation_request_path = root.join("operator_revocation.json");
    let reapproval_path = root.join("operator_reapproval.json");
    let contract_path = consumption_workspace.join("consumer_contract.json");

    fs::write(&zip_path, b"slice33 release readiness package bytes")?;
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

    let revocation_request = build_revocation_request(
        &promotion_workspace.join("promotion_receipt.json"),
        &promotion_workspace.join("import_receipt.json"),
    )?;
    write_revocation_request(&revocation_request_path, &revocation_request)?;

    let rollback_receipt = validate_revocation_request(
        &promotion_workspace.join("promotion_receipt.json"),
        &promotion_workspace.join("import_receipt.json"),
        &revocation_request_path,
    )?;
    publish_rollback_state(&revocation_workspace, &rollback_receipt)?;

    let reapproval = build_repromotion_approval(
        &revocation_workspace.join("rollback_receipt.json"),
        &promotion_workspace.join("promotion_receipt.json"),
        &promotion_workspace.join("import_receipt.json"),
    )?;
    write_repromotion_approval(&reapproval_path, &reapproval)?;

    let repromotion_receipt = validate_repromotion_approval(
        &revocation_workspace.join("rollback_receipt.json"),
        &promotion_workspace.join("promotion_receipt.json"),
        &promotion_workspace.join("import_receipt.json"),
        &reapproval_path,
    )?;
    publish_repromoted_import_receipt(
        &promotion_workspace.join("import_receipt.json"),
        &repromotion_workspace,
        &repromotion_receipt,
    )?;

    let supersession_receipt = build_supersession_chain_receipt(
        &promotion_workspace.join("promotion_receipt.json"),
        &revocation_workspace.join("rollback_receipt.json"),
        &repromotion_workspace.join("re_promotion_receipt.json"),
    )?;
    publish_supersession_chain(&supersession_workspace, &supersession_receipt)?;

    let sources = LineageBundleSourcePaths {
        promotion_receipt_path: promotion_workspace.join("promotion_receipt.json"),
        rollback_receipt_path: revocation_workspace.join("rollback_receipt.json"),
        repromotion_receipt_path: repromotion_workspace.join("re_promotion_receipt.json"),
        supersession_chain_receipt_path: supersession_workspace
            .join("supersession_chain_receipt.json"),
    };
    let _ = publish_lineage_bundle(&bundle_workspace, &sources)?;

    let intake_receipt = build_lineage_bundle_intake_receipt(&bundle_workspace)?;
    publish_intaken_lineage_bundle(&bundle_workspace, &intake_workspace, &intake_receipt)?;

    let rehydrate_receipt = build_lineage_rehydrate_receipt(&intake_workspace)?;
    publish_rehydrated_lineage_state(&intake_workspace, &rehydrate_workspace, &rehydrate_receipt)?;

    let activation_receipt = build_lineage_activation_receipt(&rehydrate_workspace)?;
    publish_activated_lineage_state(
        &rehydrate_workspace,
        &activation_workspace,
        &activation_receipt,
    )?;

    fs::create_dir_all(&consumption_workspace)?;
    let contract = default_active_lineage_consumer_contract();
    write_active_lineage_consumer_contract(&contract_path, &contract)?;
    let attestation =
        build_active_lineage_attestation_receipt(&activation_workspace, &contract_path)?;
    publish_active_lineage_attestation(&consumption_workspace, &contract, &attestation)?;

    Ok((activation_workspace, consumption_workspace))
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

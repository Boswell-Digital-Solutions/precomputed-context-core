use crate::promotion_gate::load_promotion_receipt;
use crate::promotion_revocation::load_rollback_receipt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const RE_PROMOTION_APPROVAL_SCHEMA_VERSION: &str = "proof.re-promotion-approval.v1";
pub const RE_PROMOTION_RECEIPT_SCHEMA_VERSION: &str = "proof.re-promotion-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RePromotionApproval {
    pub schema_version: String,
    pub operator_id: String,
    pub action: String,
    pub rollback_receipt_sha256: String,
    pub source_promotion_receipt_sha256: String,
    pub source_promoted_import_receipt_sha256: String,
    pub approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RePromotionReceipt {
    pub schema_version: String,
    pub operator_id: String,
    pub action: String,
    pub rollback_receipt_sha256: String,
    pub source_promotion_receipt_sha256: String,
    pub source_promoted_import_receipt_sha256: String,
    pub approved: bool,
    pub decision: String,
}

pub fn default_operator_id() -> &'static str {
    "charlie"
}

pub fn build_repromotion_approval(
    rollback_receipt_path: &Path,
    source_promotion_receipt_path: &Path,
    source_promoted_import_receipt_path: &Path,
) -> Result<RePromotionApproval, Box<dyn Error>> {
    Ok(RePromotionApproval {
        schema_version: RE_PROMOTION_APPROVAL_SCHEMA_VERSION.to_string(),
        operator_id: default_operator_id().to_string(),
        action: "re_promote_revoked_import_receipt".to_string(),
        rollback_receipt_sha256: sha256_hex_file(rollback_receipt_path)?,
        source_promotion_receipt_sha256: sha256_hex_file(source_promotion_receipt_path)?,
        source_promoted_import_receipt_sha256: sha256_hex_file(
            source_promoted_import_receipt_path,
        )?,
        approved: true,
    })
}

pub fn validate_repromotion_approval(
    rollback_receipt_path: &Path,
    source_promotion_receipt_path: &Path,
    source_promoted_import_receipt_path: &Path,
    repromotion_approval_path: &Path,
) -> Result<RePromotionReceipt, Box<dyn Error>> {
    if !rollback_receipt_path.exists() {
        return Err(format!(
            "rollback receipt missing: {}",
            rollback_receipt_path.display()
        )
        .into());
    }
    if !source_promotion_receipt_path.exists() {
        return Err(format!(
            "source promotion receipt missing: {}",
            source_promotion_receipt_path.display()
        )
        .into());
    }
    if !source_promoted_import_receipt_path.exists() {
        return Err(format!(
            "source promoted import receipt missing: {}",
            source_promoted_import_receipt_path.display()
        )
        .into());
    }
    if !repromotion_approval_path.exists() {
        return Err(format!(
            "re-promotion approval missing: {}",
            repromotion_approval_path.display()
        )
        .into());
    }

    let approval = load_repromotion_approval(repromotion_approval_path)?;
    if !approval.approved {
        return Err("re-promotion approval is not approved".into());
    }

    let rollback_receipt = load_rollback_receipt(rollback_receipt_path)?;
    if !rollback_receipt.revoked {
        return Err("rollback receipt is not revoked".into());
    }

    let source_promotion_receipt = load_promotion_receipt(source_promotion_receipt_path)?;
    if !source_promotion_receipt.approved {
        return Err("source promotion receipt is not approved".into());
    }

    let expected_rollback_receipt_sha256 = sha256_hex_file(rollback_receipt_path)?;
    if approval.rollback_receipt_sha256 != expected_rollback_receipt_sha256 {
        return Err(format!(
            "re-promotion approval rollback hash mismatch: expected={} actual={}",
            expected_rollback_receipt_sha256, approval.rollback_receipt_sha256
        )
        .into());
    }

    let expected_source_promotion_receipt_sha256 = sha256_hex_file(source_promotion_receipt_path)?;
    if approval.source_promotion_receipt_sha256 != expected_source_promotion_receipt_sha256 {
        return Err(format!(
            "re-promotion approval source promotion hash mismatch: expected={} actual={}",
            expected_source_promotion_receipt_sha256, approval.source_promotion_receipt_sha256
        )
        .into());
    }

    let expected_source_promoted_import_receipt_sha256 =
        sha256_hex_file(source_promoted_import_receipt_path)?;
    if approval.source_promoted_import_receipt_sha256
        != expected_source_promoted_import_receipt_sha256
    {
        return Err(format!(
            "re-promotion approval source import hash mismatch: expected={} actual={}",
            expected_source_promoted_import_receipt_sha256,
            approval.source_promoted_import_receipt_sha256
        )
        .into());
    }

    Ok(RePromotionReceipt {
        schema_version: RE_PROMOTION_RECEIPT_SCHEMA_VERSION.to_string(),
        operator_id: approval.operator_id,
        action: approval.action,
        rollback_receipt_sha256: expected_rollback_receipt_sha256,
        source_promotion_receipt_sha256: expected_source_promotion_receipt_sha256,
        source_promoted_import_receipt_sha256: expected_source_promoted_import_receipt_sha256,
        approved: true,
        decision: "approved_re_promotion".to_string(),
    })
}

pub fn default_repromotion_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice23_repromotion/current")
}

pub fn default_repromotion_approval_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice23_repromotion/operator_reapproval.json")
}

pub fn default_repromotion_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("re_promotion_receipt.json")
}

pub fn default_repromoted_import_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("import_receipt.json")
}

pub fn publish_repromoted_import_receipt(
    source_promoted_import_receipt_path: &Path,
    workspace_current_dir: &Path,
    receipt: &RePromotionReceipt,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(workspace_current_dir)?;
    let destination = default_repromoted_import_receipt_path(workspace_current_dir);
    fs::copy(source_promoted_import_receipt_path, &destination)?;
    write_repromotion_receipt(
        &default_repromotion_receipt_path(workspace_current_dir),
        receipt,
    )?;
    Ok(())
}

pub fn write_repromotion_approval(
    path: &Path,
    approval: &RePromotionApproval,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(approval)?)?;
    Ok(())
}

pub fn load_repromotion_approval(path: &Path) -> Result<RePromotionApproval, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let approval: RePromotionApproval = serde_json::from_slice(&bytes)?;
    Ok(approval)
}

pub fn write_repromotion_receipt(
    path: &Path,
    receipt: &RePromotionReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_repromotion_receipt(path: &Path) -> Result<RePromotionReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: RePromotionReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

pub fn default_slice22_rollback_receipt_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice22_revocation/current/rollback_receipt.json")
}

pub fn default_slice21_source_promotion_receipt_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice21_promotion/current/promotion_receipt.json")
}

pub fn default_slice21_source_promoted_import_receipt_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice21_promotion/current/import_receipt.json")
}

pub fn sha256_hex_file(path: &Path) -> Result<String, Box<dyn Error>> {
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

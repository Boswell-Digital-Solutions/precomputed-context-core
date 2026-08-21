use crate::promotion_gate::load_promotion_receipt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const PROMOTION_REVOCATION_REQUEST_SCHEMA_VERSION: &str =
    "proof.promotion-revocation-request.v1";
pub const PROMOTION_ROLLBACK_RECEIPT_SCHEMA_VERSION: &str = "proof.promotion-rollback-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromotionRevocationRequest {
    pub schema_version: String,
    pub operator_id: String,
    pub action: String,
    pub promotion_receipt_sha256: String,
    pub promoted_import_receipt_sha256: String,
    pub revoke: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromotionRollbackReceipt {
    pub schema_version: String,
    pub operator_id: String,
    pub action: String,
    pub promotion_receipt_sha256: String,
    pub promoted_import_receipt_sha256: String,
    pub revoked: bool,
    pub decision: String,
}

pub fn default_operator_id() -> &'static str {
    "charlie"
}

pub fn build_revocation_request(
    promotion_receipt_path: &Path,
    promoted_import_receipt_path: &Path,
) -> Result<PromotionRevocationRequest, Box<dyn Error>> {
    Ok(PromotionRevocationRequest {
        schema_version: PROMOTION_REVOCATION_REQUEST_SCHEMA_VERSION.to_string(),
        operator_id: default_operator_id().to_string(),
        action: "revoke_promoted_import_receipt".to_string(),
        promotion_receipt_sha256: sha256_hex_file(promotion_receipt_path)?,
        promoted_import_receipt_sha256: sha256_hex_file(promoted_import_receipt_path)?,
        revoke: true,
    })
}

pub fn validate_revocation_request(
    promotion_receipt_path: &Path,
    promoted_import_receipt_path: &Path,
    revocation_request_path: &Path,
) -> Result<PromotionRollbackReceipt, Box<dyn Error>> {
    if !promotion_receipt_path.exists() {
        return Err(format!(
            "promotion receipt missing: {}",
            promotion_receipt_path.display()
        )
        .into());
    }

    if !promoted_import_receipt_path.exists() {
        return Err(format!(
            "promoted import receipt missing: {}",
            promoted_import_receipt_path.display()
        )
        .into());
    }

    if !revocation_request_path.exists() {
        return Err(format!(
            "promotion revocation request missing: {}",
            revocation_request_path.display()
        )
        .into());
    }

    let request = load_revocation_request(revocation_request_path)?;
    if !request.revoke {
        return Err("promotion revocation request is not marked for revoke".into());
    }

    let promotion_receipt = load_promotion_receipt(promotion_receipt_path)?;
    if !promotion_receipt.approved {
        return Err("promotion receipt is not approved".into());
    }

    let expected_promotion_receipt_sha256 = sha256_hex_file(promotion_receipt_path)?;
    if request.promotion_receipt_sha256 != expected_promotion_receipt_sha256 {
        return Err(format!(
            "promotion revocation request receipt hash mismatch: expected={} actual={}",
            expected_promotion_receipt_sha256, request.promotion_receipt_sha256
        )
        .into());
    }

    let expected_promoted_import_receipt_sha256 = sha256_hex_file(promoted_import_receipt_path)?;
    if request.promoted_import_receipt_sha256 != expected_promoted_import_receipt_sha256 {
        return Err(format!(
            "promotion revocation request promoted import hash mismatch: expected={} actual={}",
            expected_promoted_import_receipt_sha256, request.promoted_import_receipt_sha256
        )
        .into());
    }

    Ok(PromotionRollbackReceipt {
        schema_version: PROMOTION_ROLLBACK_RECEIPT_SCHEMA_VERSION.to_string(),
        operator_id: request.operator_id,
        action: request.action,
        promotion_receipt_sha256: expected_promotion_receipt_sha256,
        promoted_import_receipt_sha256: expected_promoted_import_receipt_sha256,
        revoked: true,
        decision: "revoked_and_rolled_back".to_string(),
    })
}

pub fn default_revocation_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice22_revocation/current")
}

pub fn default_revocation_request_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice22_revocation/operator_revocation.json")
}

pub fn default_rollback_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("rollback_receipt.json")
}

pub fn publish_rollback_state(
    workspace_current_dir: &Path,
    rollback_receipt: &PromotionRollbackReceipt,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(workspace_current_dir)?;
    let promoted_path = workspace_current_dir.join("import_receipt.json");
    if promoted_path.exists() {
        fs::remove_file(promoted_path)?;
    }
    write_rollback_receipt(
        &default_rollback_receipt_path(workspace_current_dir),
        rollback_receipt,
    )?;
    Ok(())
}

pub fn write_revocation_request(
    path: &Path,
    request: &PromotionRevocationRequest,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(request)?)?;
    Ok(())
}

pub fn load_revocation_request(path: &Path) -> Result<PromotionRevocationRequest, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let request: PromotionRevocationRequest = serde_json::from_slice(&bytes)?;
    Ok(request)
}

pub fn write_rollback_receipt(
    path: &Path,
    receipt: &PromotionRollbackReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_rollback_receipt(path: &Path) -> Result<PromotionRollbackReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: PromotionRollbackReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

pub fn default_slice21_promotion_receipt_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice21_promotion/current/promotion_receipt.json")
}

pub fn default_slice21_promoted_import_receipt_path() -> PathBuf {
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

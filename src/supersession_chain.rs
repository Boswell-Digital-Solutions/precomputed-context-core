use crate::promotion_gate::load_promotion_receipt;
use crate::promotion_revocation::load_rollback_receipt;
use crate::re_promotion::load_repromotion_receipt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const SUPERSESSION_CHAIN_RECEIPT_SCHEMA_VERSION: &str = "proof.supersession-chain-receipt.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupersessionLink {
    pub relation: String,
    pub from_stage: String,
    pub from_receipt_sha256: String,
    pub to_stage: String,
    pub to_receipt_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupersessionChainReceipt {
    pub schema_version: String,
    pub promotion_receipt_sha256: String,
    pub rollback_receipt_sha256: String,
    pub repromotion_receipt_sha256: String,
    pub lineage_state: String,
    pub complete: bool,
    pub links: Vec<SupersessionLink>,
}

pub fn build_supersession_chain_receipt(
    promotion_receipt_path: &Path,
    rollback_receipt_path: &Path,
    repromotion_receipt_path: &Path,
) -> Result<SupersessionChainReceipt, Box<dyn Error>> {
    if !promotion_receipt_path.exists() {
        return Err(format!(
            "promotion receipt missing: {}",
            promotion_receipt_path.display()
        )
        .into());
    }
    if !rollback_receipt_path.exists() {
        return Err(format!(
            "rollback receipt missing: {}",
            rollback_receipt_path.display()
        )
        .into());
    }
    if !repromotion_receipt_path.exists() {
        return Err(format!(
            "re-promotion receipt missing: {}",
            repromotion_receipt_path.display()
        )
        .into());
    }

    let promotion_receipt = load_promotion_receipt(promotion_receipt_path)?;
    if !promotion_receipt.approved {
        return Err("promotion receipt is not approved".into());
    }

    let rollback_receipt = load_rollback_receipt(rollback_receipt_path)?;
    if !rollback_receipt.revoked {
        return Err("rollback receipt is not revoked".into());
    }

    let repromotion_receipt = load_repromotion_receipt(repromotion_receipt_path)?;
    if !repromotion_receipt.approved {
        return Err("re-promotion receipt is not approved".into());
    }

    let promotion_receipt_sha256 = sha256_hex_file(promotion_receipt_path)?;
    let rollback_receipt_sha256 = sha256_hex_file(rollback_receipt_path)?;
    let repromotion_receipt_sha256 = sha256_hex_file(repromotion_receipt_path)?;

    if rollback_receipt.promotion_receipt_sha256 != promotion_receipt_sha256 {
        return Err(format!(
            "rollback receipt promotion hash mismatch: expected={} actual={}",
            promotion_receipt_sha256, rollback_receipt.promotion_receipt_sha256
        )
        .into());
    }

    if repromotion_receipt.rollback_receipt_sha256 != rollback_receipt_sha256 {
        return Err(format!(
            "re-promotion receipt rollback hash mismatch: expected={} actual={}",
            rollback_receipt_sha256, repromotion_receipt.rollback_receipt_sha256
        )
        .into());
    }

    if repromotion_receipt.source_promotion_receipt_sha256 != promotion_receipt_sha256 {
        return Err(format!(
            "re-promotion receipt source promotion hash mismatch: expected={} actual={}",
            promotion_receipt_sha256, repromotion_receipt.source_promotion_receipt_sha256
        )
        .into());
    }

    Ok(SupersessionChainReceipt {
        schema_version: SUPERSESSION_CHAIN_RECEIPT_SCHEMA_VERSION.to_string(),
        promotion_receipt_sha256: promotion_receipt_sha256.clone(),
        rollback_receipt_sha256: rollback_receipt_sha256.clone(),
        repromotion_receipt_sha256: repromotion_receipt_sha256.clone(),
        lineage_state: "promotion_rolled_back_then_repromoted".to_string(),
        complete: true,
        links: vec![
            SupersessionLink {
                relation: "superseded_by".to_string(),
                from_stage: "slice21_promotion".to_string(),
                from_receipt_sha256: promotion_receipt_sha256.clone(),
                to_stage: "slice22_revocation".to_string(),
                to_receipt_sha256: rollback_receipt_sha256.clone(),
            },
            SupersessionLink {
                relation: "reinstated_by".to_string(),
                from_stage: "slice22_revocation".to_string(),
                from_receipt_sha256: rollback_receipt_sha256,
                to_stage: "slice23_repromotion".to_string(),
                to_receipt_sha256: repromotion_receipt_sha256,
            },
        ],
    })
}

pub fn default_supersession_workspace_current() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice24_supersession/current")
}

pub fn default_supersession_chain_receipt_path(workspace_current_dir: &Path) -> PathBuf {
    workspace_current_dir.join("supersession_chain_receipt.json")
}

pub fn publish_supersession_chain(
    workspace_current_dir: &Path,
    receipt: &SupersessionChainReceipt,
) -> Result<(), Box<dyn Error>> {
    write_supersession_chain_receipt(
        &default_supersession_chain_receipt_path(workspace_current_dir),
        receipt,
    )
}

pub fn write_supersession_chain_receipt(
    path: &Path,
    receipt: &SupersessionChainReceipt,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_supersession_chain_receipt(
    path: &Path,
) -> Result<SupersessionChainReceipt, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let receipt: SupersessionChainReceipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

pub fn default_slice21_promotion_receipt_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice21_promotion/current/promotion_receipt.json")
}

pub fn default_slice22_rollback_receipt_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice22_revocation/current/rollback_receipt.json")
}

pub fn default_slice23_repromotion_receipt_path() -> PathBuf {
    PathBuf::from("target/proof_artifacts/slice23_repromotion/current/re_promotion_receipt.json")
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

use std::fs;
use std::path::{Path, PathBuf};

use schemars::schema_for;
use serde_json::Value;

use crate::authority::AuthorityResolutionRecord;
use crate::context_assembly::{ContextAssemblyRequest, ContextBundleManifest};
use crate::contracts::{
    KeyFilePacketContract, RepoNavigationAssistPacketContract, RepoNavigationMapContract,
    ValidationCommandPacketContract,
};
use crate::events::EventRecord;
use crate::models::{OverrideRecord, RemediationItem};

pub fn schema_catalog() -> Vec<(&'static str, Value)> {
    vec![
        (
            "authority_resolution_record.schema.json",
            serde_json::to_value(schema_for!(AuthorityResolutionRecord))
                .expect("authority schema to serialize"),
        ),
        (
            "repo_navigation_map_contract.schema.json",
            serde_json::to_value(schema_for!(RepoNavigationMapContract))
                .expect("repo navigation map schema to serialize"),
        ),
        (
            "key_file_packet_contract.schema.json",
            serde_json::to_value(schema_for!(KeyFilePacketContract))
                .expect("key file packet schema to serialize"),
        ),
        (
            "validation_command_packet_contract.schema.json",
            serde_json::to_value(schema_for!(ValidationCommandPacketContract))
                .expect("validation command packet schema to serialize"),
        ),
        (
            "repo_navigation_assist_packet_contract.schema.json",
            serde_json::to_value(schema_for!(RepoNavigationAssistPacketContract))
                .expect("repo navigation assist packet schema to serialize"),
        ),
        (
            "context_assembly_request.schema.json",
            serde_json::to_value(schema_for!(ContextAssemblyRequest))
                .expect("context assembly request schema to serialize"),
        ),
        (
            "context_bundle_manifest.schema.json",
            serde_json::to_value(schema_for!(ContextBundleManifest))
                .expect("context bundle manifest schema to serialize"),
        ),
        (
            "event_record.schema.json",
            serde_json::to_value(schema_for!(EventRecord))
                .expect("event record schema to serialize"),
        ),
        (
            "remediation_item.schema.json",
            serde_json::to_value(schema_for!(RemediationItem))
                .expect("remediation item schema to serialize"),
        ),
        (
            "override_record.schema.json",
            serde_json::to_value(schema_for!(OverrideRecord))
                .expect("override record schema to serialize"),
        ),
    ]
}

pub fn export_schemas(root: &Path) -> Result<Vec<PathBuf>, String> {
    let dir = root.join("schemas");
    fs::create_dir_all(&dir)
        .map_err(|err| format!("failed to create {}: {}", dir.display(), err))?;

    let mut written = Vec::new();

    for (name, schema) in schema_catalog() {
        let path = dir.join(name);
        let content = serde_json::to_string_pretty(&schema)
            .map_err(|err| format!("failed to serialize schema {}: {}", name, err))?;
        fs::write(&path, content)
            .map_err(|err| format!("failed to write {}: {}", path.display(), err))?;
        written.push(path);
    }

    Ok(written)
}

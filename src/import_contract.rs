use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED_ARCHIVE_MEMBERS: &[&str] = &[
    "README.txt",
    "governed_flow_report.json",
    "package_index.json",
    "replay_bundle_manifest.json",
    "replay_report.json",
];

const REQUIRED_REHYDRATED_FILES: &[&str] = &[
    "rehydrated/governed_flow/governed_flow_report.json",
    "rehydrated/package_meta/README.txt",
    "rehydrated/package_meta/package_index.json",
    "rehydrated/replay/replay_bundle_manifest.json",
    "rehydrated/replay/replay_report.json",
];

const ROUNDTRIP_FILE_MAPPINGS: &[(&str, &str)] = &[
    (
        "package/governed_flow_report.json",
        "rehydrated/governed_flow/governed_flow_report.json",
    ),
    ("package/README.txt", "rehydrated/package_meta/README.txt"),
    (
        "package/package_index.json",
        "rehydrated/package_meta/package_index.json",
    ),
    (
        "package/replay_bundle_manifest.json",
        "rehydrated/replay/replay_bundle_manifest.json",
    ),
    (
        "package/replay_report.json",
        "rehydrated/replay/replay_report.json",
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReceipt {
    pub slice: u32,
    pub source_zip: String,
    pub source_sha256_file: String,
    pub source_digest: String,
    pub archive_members: Vec<String>,
    pub rehydrated_files: Vec<String>,
    pub replay_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundtripReport {
    pub workspace: String,
    pub source_digest: String,
    pub validated_pairs: Vec<String>,
}

pub fn load_import_receipt(workspace_path: &Path) -> Result<ImportReceipt, String> {
    let receipt_path = workspace_path.join("import_receipt.json");
    let receipt_text = fs::read_to_string(&receipt_path)
        .map_err(|e| format!("failed to read receipt {}: {e}", receipt_path.display()))?;

    Ok(ImportReceipt {
        slice: extract_u32_field(&receipt_text, "slice")?,
        source_zip: extract_string_field(&receipt_text, "source_zip")?,
        source_sha256_file: extract_string_field(&receipt_text, "source_sha256_file")?,
        source_digest: extract_string_field(&receipt_text, "source_digest")?,
        archive_members: extract_array_field(&receipt_text, "archive_members")?,
        rehydrated_files: extract_array_field(&receipt_text, "rehydrated_files")?,
        replay_ready: extract_bool_field(&receipt_text, "replay_ready")?,
    })
}

pub fn validate_import_workspace(workspace_path: &Path) -> Result<RoundtripReport, String> {
    let receipt = load_import_receipt(workspace_path)?;

    if receipt.slice != 16 {
        return Err(format!(
            "unexpected import receipt slice: expected 16, got {}",
            receipt.slice
        ));
    }

    if !receipt.replay_ready {
        return Err("import receipt is not replay_ready".to_string());
    }

    validate_digest(&receipt.source_digest)?;
    validate_string_list_exact(
        &receipt.archive_members,
        REQUIRED_ARCHIVE_MEMBERS,
        "archive_members",
    )?;
    validate_string_list_exact(
        &receipt.rehydrated_files,
        REQUIRED_REHYDRATED_FILES,
        "rehydrated_files",
    )?;

    let receipt_path = workspace_path.join("import_receipt.json");
    if !receipt_path.is_file() {
        return Err(format!("missing receipt file: {}", receipt_path.display()));
    }

    for member in REQUIRED_ARCHIVE_MEMBERS {
        let path = workspace_path.join("package").join(member);
        if !path.is_file() {
            return Err(format!("missing required package file: {}", path.display()));
        }
    }

    for relative in REQUIRED_REHYDRATED_FILES {
        let path = workspace_path.join(relative);
        if !path.is_file() {
            return Err(format!(
                "missing required rehydrated file: {}",
                path.display()
            ));
        }
    }

    let mut validated_pairs = Vec::new();
    for (package_relative, rehydrated_relative) in ROUNDTRIP_FILE_MAPPINGS {
        let package_path = workspace_path.join(package_relative);
        let rehydrated_path = workspace_path.join(rehydrated_relative);
        assert_file_bytes_equal(&package_path, &rehydrated_path)?;
        validated_pairs.push(format!("{} => {}", package_relative, rehydrated_relative));
    }

    Ok(RoundtripReport {
        workspace: workspace_path.to_string_lossy().to_string(),
        source_digest: receipt.source_digest,
        validated_pairs,
    })
}

pub fn render_roundtrip_report(report: &RoundtripReport) -> String {
    let pairs_json = json_array(&report.validated_pairs);

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"slice\": 17,\n");
    out.push_str(&format!(
        "  \"workspace\": \"{}\",\n",
        json_escape(&report.workspace)
    ));
    out.push_str(&format!(
        "  \"source_digest\": \"{}\",\n",
        json_escape(&report.source_digest)
    ));
    out.push_str(&format!("  \"validated_pairs\": {},\n", pairs_json));
    out.push_str("  \"roundtrip_ready\": true\n");
    out.push_str("}\n");
    out
}

fn validate_digest(digest: &str) -> Result<(), String> {
    if digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!("invalid source digest: {}", digest))
    }
}

fn validate_string_list_exact(
    actual: &[String],
    expected: &[&str],
    field_name: &str,
) -> Result<(), String> {
    let expected_strings: Vec<String> = expected.iter().map(|value| value.to_string()).collect();
    if actual == expected_strings {
        Ok(())
    } else {
        Err(format!(
            "{} mismatch: expected {:?}, got {:?}",
            field_name, expected_strings, actual
        ))
    }
}

fn assert_file_bytes_equal(left: &Path, right: &Path) -> Result<(), String> {
    let left_bytes =
        fs::read(left).map_err(|e| format!("failed to read file {}: {e}", left.display()))?;
    let right_bytes =
        fs::read(right).map_err(|e| format!("failed to read file {}: {e}", right.display()))?;

    if left_bytes == right_bytes {
        Ok(())
    } else {
        Err(format!(
            "file content mismatch: {} vs {}",
            left.display(),
            right.display()
        ))
    }
}

fn extract_u32_field(text: &str, key: &str) -> Result<u32, String> {
    let raw = extract_scalar_value(text, key)?;
    raw.parse::<u32>()
        .map_err(|e| format!("failed to parse {} as u32: {e}", key))
}

fn extract_bool_field(text: &str, key: &str) -> Result<bool, String> {
    let raw = extract_scalar_value(text, key)?;
    match raw {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("failed to parse {} as bool: {}", key, raw)),
    }
}

fn extract_string_field(text: &str, key: &str) -> Result<String, String> {
    let raw = extract_scalar_value(text, key)?;
    let stripped = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| format!("field {} was not a JSON string", key))?;
    Ok(json_unescape(stripped))
}

fn extract_array_field(text: &str, key: &str) -> Result<Vec<String>, String> {
    let raw = extract_scalar_value(text, key)?;
    let inner = raw
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("field {} was not a JSON array", key))?
        .trim();

    if inner.is_empty() {
        return Ok(Vec::new());
    }

    let mut values = Vec::new();
    for part in inner.split(", ") {
        let stripped = part
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .ok_or_else(|| format!("field {} contained a non-string array item", key))?;
        values.push(json_unescape(stripped));
    }
    Ok(values)
}

fn extract_scalar_value<'a>(text: &'a str, key: &str) -> Result<&'a str, String> {
    let needle = format!("\"{}\": ", key);
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with(&needle))
        .ok_or_else(|| format!("missing field: {}", key))?;

    let value = line
        .trim()
        .strip_prefix(&needle)
        .ok_or_else(|| format!("field {} had an unexpected shape", key))?
        .trim_end_matches(',');

    Ok(value)
}

fn json_array(values: &[String]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push('"');
        out.push_str(&json_escape(value));
        out.push('"');
    }
    out.push(']');
    out
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn json_unescape(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn default_roundtrip_report_path(workspace_path: &Path) -> PathBuf {
    workspace_path.join("roundtrip_report.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_fields_from_receipt_text() {
        let text = concat!(
            "{\n",
            "  \"slice\": 16,\n",
            "  \"source_zip\": \"target/proof_artifacts/slice14_export.zip\",\n",
            "  \"source_sha256_file\": \"target/proof_artifacts/slice14_export.zip.sha256\",\n",
            "  \"source_digest\": \"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\n",
            "  \"archive_members\": [\"README.txt\", \"governed_flow_report.json\", \"package_index.json\", \"replay_bundle_manifest.json\", \"replay_report.json\"],\n",
            "  \"rehydrated_files\": [\"rehydrated/governed_flow/governed_flow_report.json\", \"rehydrated/package_meta/README.txt\", \"rehydrated/package_meta/package_index.json\", \"rehydrated/replay/replay_bundle_manifest.json\", \"rehydrated/replay/replay_report.json\"],\n",
            "  \"replay_ready\": true\n",
            "}\n"
        );

        assert_eq!(extract_u32_field(text, "slice").unwrap(), 16);
        assert!(extract_bool_field(text, "replay_ready").unwrap());
        assert_eq!(
            extract_string_field(text, "source_zip").unwrap(),
            "target/proof_artifacts/slice14_export.zip"
        );
        assert_eq!(
            extract_array_field(text, "archive_members").unwrap().len(),
            5
        );
    }

    #[test]
    fn validate_string_list_exact_rejects_drift() {
        let actual = vec!["README.txt".to_string()];
        let error =
            validate_string_list_exact(&actual, REQUIRED_ARCHIVE_MEMBERS, "archive_members")
                .expect_err("drift should fail");
        assert!(error.contains("archive_members mismatch"));
    }

    #[test]
    fn render_roundtrip_report_contains_expected_fields() {
        let report = RoundtripReport {
            workspace: "target/proof_artifacts/slice16_import/current".to_string(),
            source_digest: "abc123".to_string(),
            validated_pairs: vec!["left => right".to_string()],
        };

        let text = render_roundtrip_report(&report);
        assert!(text.contains("\"slice\": 17"));
        assert!(text.contains("\"source_digest\": \"abc123\""));
        assert!(text.contains("left => right"));
    }
}

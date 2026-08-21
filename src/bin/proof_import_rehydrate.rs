use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

const DEFAULT_ZIP_PATH: &str = "target/proof_artifacts/slice14_export.zip";
const DEFAULT_SHA_PATH: &str = "target/proof_artifacts/slice14_export.zip.sha256";
const DEFAULT_WORKSPACE_PATH: &str = "target/proof_artifacts/slice16_import/current";
const REQUIRED_ARCHIVE_MEMBERS: &[&str] = &[
    "README.txt",
    "governed_flow_report.json",
    "package_index.json",
    "replay_bundle_manifest.json",
    "replay_report.json",
];
const INDEX_REQUIRED_REFERENCES: &[&str] = &[
    "README.txt",
    "governed_flow_report.json",
    "replay_bundle_manifest.json",
    "replay_report.json",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Config {
    zip_path: PathBuf,
    sha_path: PathBuf,
    workspace_path: PathBuf,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("slice16 import rehydrate failed: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = parse_args(env::args().skip(1).collect())?;

    if !config.zip_path.is_file() {
        return Err(format!("missing zip file: {}", config.zip_path.display()));
    }

    if !config.sha_path.is_file() {
        return Err(format!(
            "missing sha256 file: {}",
            config.sha_path.display()
        ));
    }

    let stage_path = stage_path_for(&config.workspace_path);
    cleanup_dir(&stage_path)?;

    let result = run_inner(&config, &stage_path);
    if let Err(error) = result {
        let _ = cleanup_dir(&stage_path);
        return Err(error);
    }

    cleanup_dir(&config.workspace_path)?;
    if let Some(parent) = config.workspace_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create workspace parent {}: {e}",
                parent.display()
            )
        })?;
    }

    fs::rename(&stage_path, &config.workspace_path).map_err(|e| {
        format!(
            "failed to publish workspace {} from {}: {e}",
            config.workspace_path.display(),
            stage_path.display()
        )
    })?;

    println!("slice16 import rehydrate passed");
    println!("  workspace: {}", config.workspace_path.display());
    println!(
        "  receipt: {}",
        config.workspace_path.join("import_receipt.json").display()
    );
    Ok(())
}

fn run_inner(config: &Config, stage_path: &Path) -> Result<(), String> {
    fs::create_dir_all(stage_path)
        .map_err(|e| format!("failed to create stage path {}: {e}", stage_path.display()))?;

    let expected_digest = read_expected_digest(&config.sha_path, &config.zip_path)?;
    let actual_digest = compute_sha256(&config.zip_path)?;

    if expected_digest != actual_digest {
        return Err(format!(
            "sha256 mismatch for {}: expected {}, got {}",
            config.zip_path.display(),
            expected_digest,
            actual_digest
        ));
    }

    let archive_members = list_archive_members(&config.zip_path)?;
    validate_archive_member_set(&archive_members)?;

    let package_root = stage_path.join("package");
    extract_archive(&config.zip_path, &package_root)?;

    let package_index_text =
        fs::read_to_string(package_root.join("package_index.json")).map_err(|e| {
            format!(
                "failed to read extracted package_index.json from {}: {e}",
                package_root.display()
            )
        })?;
    validate_package_index_text(&package_index_text)?;

    rehydrate_workspace(&package_root, &stage_path.join("rehydrated"))?;
    write_import_receipt(stage_path, config, &actual_digest, &archive_members)?;
    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<Config, String> {
    let mut zip_path = PathBuf::from(DEFAULT_ZIP_PATH);
    let mut sha_path = PathBuf::from(DEFAULT_SHA_PATH);
    let mut workspace_path = PathBuf::from(DEFAULT_WORKSPACE_PATH);

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--zip" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value after --zip".to_string())?;
                zip_path = PathBuf::from(value);
                index += 2;
            }
            "--sha" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value after --sha".to_string())?;
                sha_path = PathBuf::from(value);
                index += 2;
            }
            "--workspace" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value after --workspace".to_string())?;
                workspace_path = PathBuf::from(value);
                index += 2;
            }
            "--help" | "-h" => {
                return Err(
                    "usage: cargo run --bin proof_import_rehydrate -- [--zip PATH] [--sha PATH] [--workspace PATH]"
                        .to_string(),
                );
            }
            other => {
                return Err(format!("unexpected argument: {other}"));
            }
        }
    }

    Ok(Config {
        zip_path,
        sha_path,
        workspace_path,
    })
}

fn read_expected_digest(sha_path: &Path, zip_path: &Path) -> Result<String, String> {
    let content = fs::read_to_string(sha_path)
        .map_err(|e| format!("failed to read sha256 file {}: {e}", sha_path.display()))?;
    parse_sha256_file(&content, zip_path)
}

fn parse_sha256_file(content: &str, zip_path: &Path) -> Result<String, String> {
    let mut parts = content.split_whitespace();
    let digest = parts
        .next()
        .ok_or_else(|| "sha256 file did not contain a digest".to_string())?;
    let file_name = parts
        .next()
        .ok_or_else(|| "sha256 file did not contain a file name".to_string())?;

    if digest.len() != 64 || !digest.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("invalid sha256 digest: {digest}"));
    }

    let expected_name = zip_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid zip file name: {}", zip_path.display()))?;

    if file_name != expected_name {
        return Err(format!(
            "sha256 file name mismatch: expected {}, got {}",
            expected_name, file_name
        ));
    }

    Ok(digest.to_ascii_lowercase())
}

fn compute_sha256(zip_path: &Path) -> Result<String, String> {
    let output = Command::new("sha256sum")
        .arg(zip_path)
        .output()
        .map_err(|e| {
            format!(
                "failed to execute sha256sum for {}: {e}",
                zip_path.display()
            )
        })?;

    if !output.status.success() {
        return Err(format!(
            "sha256sum returned non-zero status for {}",
            zip_path.display()
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("sha256sum produced invalid UTF-8: {e}"))?;
    let digest = stdout.split_whitespace().next().ok_or_else(|| {
        format!(
            "sha256sum did not return a digest for {}",
            zip_path.display()
        )
    })?;

    Ok(digest.to_ascii_lowercase())
}

fn list_archive_members(zip_path: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("unzip")
        .arg("-Z1")
        .arg(zip_path)
        .output()
        .map_err(|e| {
            format!(
                "failed to list archive members for {}: {e}",
                zip_path.display()
            )
        })?;

    if !output.status.success() {
        return Err(format!(
            "unzip -Z1 returned non-zero status for {}",
            zip_path.display()
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("unzip -Z1 produced invalid UTF-8: {e}"))?;

    let mut members: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    members.sort();
    members.dedup();
    Ok(members)
}

fn validate_archive_member_set(members: &[String]) -> Result<(), String> {
    let expected: Vec<String> = REQUIRED_ARCHIVE_MEMBERS
        .iter()
        .map(|member| member.to_string())
        .collect();

    if members == expected {
        Ok(())
    } else {
        Err(format!(
            "archive member set mismatch: expected {:?}, got {:?}",
            expected, members
        ))
    }
}

fn extract_archive(zip_path: &Path, package_root: &Path) -> Result<(), String> {
    fs::create_dir_all(package_root).map_err(|e| {
        format!(
            "failed to create package root {}: {e}",
            package_root.display()
        )
    })?;

    let status = Command::new("unzip")
        .arg("-q")
        .arg(zip_path)
        .arg("-d")
        .arg(package_root)
        .status()
        .map_err(|e| format!("failed to extract {}: {e}", zip_path.display()))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "unzip extraction returned non-zero status for {}",
            zip_path.display()
        ))
    }
}

fn validate_package_index_text(package_index_text: &str) -> Result<(), String> {
    for member in INDEX_REQUIRED_REFERENCES {
        let quoted = format!("\"{member}\"");
        if !package_index_text.contains(&quoted) {
            return Err(format!(
                "package_index.json is missing required member reference {}",
                member
            ));
        }
    }

    Ok(())
}

fn rehydrate_workspace(package_root: &Path, rehydrated_root: &Path) -> Result<(), String> {
    let replay_root = rehydrated_root.join("replay");
    let flow_root = rehydrated_root.join("governed_flow");
    let package_meta_root = rehydrated_root.join("package_meta");

    fs::create_dir_all(&replay_root).map_err(|e| {
        format!(
            "failed to create replay root {}: {e}",
            replay_root.display()
        )
    })?;
    fs::create_dir_all(&flow_root)
        .map_err(|e| format!("failed to create flow root {}: {e}", flow_root.display()))?;
    fs::create_dir_all(&package_meta_root).map_err(|e| {
        format!(
            "failed to create package metadata root {}: {e}",
            package_meta_root.display()
        )
    })?;

    copy_file(
        &package_root.join("replay_report.json"),
        &replay_root.join("replay_report.json"),
    )?;
    copy_file(
        &package_root.join("replay_bundle_manifest.json"),
        &replay_root.join("replay_bundle_manifest.json"),
    )?;
    copy_file(
        &package_root.join("governed_flow_report.json"),
        &flow_root.join("governed_flow_report.json"),
    )?;
    copy_file(
        &package_root.join("package_index.json"),
        &package_meta_root.join("package_index.json"),
    )?;
    copy_file(
        &package_root.join("README.txt"),
        &package_meta_root.join("README.txt"),
    )?;

    Ok(())
}

fn copy_file(source: &Path, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create destination parent {}: {e}",
                parent.display()
            )
        })?;
    }

    fs::copy(source, destination).map_err(|e| {
        format!(
            "failed to copy {} to {}: {e}",
            source.display(),
            destination.display()
        )
    })?;

    Ok(())
}

fn write_import_receipt(
    stage_path: &Path,
    config: &Config,
    digest: &str,
    archive_members: &[String],
) -> Result<(), String> {
    let receipt_path = stage_path.join("import_receipt.json");
    let receipt = render_import_receipt(config, digest, archive_members);
    fs::write(&receipt_path, receipt)
        .map_err(|e| format!("failed to write receipt {}: {e}", receipt_path.display()))
}

fn render_import_receipt(config: &Config, digest: &str, archive_members: &[String]) -> String {
    let rehydrated_files = vec![
        "rehydrated/governed_flow/governed_flow_report.json".to_string(),
        "rehydrated/package_meta/README.txt".to_string(),
        "rehydrated/package_meta/package_index.json".to_string(),
        "rehydrated/replay/replay_bundle_manifest.json".to_string(),
        "rehydrated/replay/replay_report.json".to_string(),
    ];

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"slice\": 16,\n");
    out.push_str(&format!(
        "  \"source_zip\": \"{}\",\n",
        json_escape(&config.zip_path.to_string_lossy())
    ));
    out.push_str(&format!(
        "  \"source_sha256_file\": \"{}\",\n",
        json_escape(&config.sha_path.to_string_lossy())
    ));
    out.push_str(&format!("  \"source_digest\": \"{}\",\n", digest));
    out.push_str(&format!(
        "  \"archive_members\": {},\n",
        json_array(archive_members)
    ));
    out.push_str(&format!(
        "  \"rehydrated_files\": {},\n",
        json_array(&rehydrated_files)
    ));
    out.push_str("  \"replay_ready\": true\n");
    out.push_str("}\n");
    out
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

fn stage_path_for(workspace_path: &Path) -> PathBuf {
    let mut as_text = workspace_path.to_string_lossy().to_string();
    as_text.push_str(".stage");
    PathBuf::from(as_text)
}

fn cleanup_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("failed to remove directory {}: {e}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_args_uses_defaults() {
        let config = parse_args(vec![]).expect("defaults should parse");
        assert_eq!(
            config,
            Config {
                zip_path: PathBuf::from(DEFAULT_ZIP_PATH),
                sha_path: PathBuf::from(DEFAULT_SHA_PATH),
                workspace_path: PathBuf::from(DEFAULT_WORKSPACE_PATH),
            }
        );
    }

    #[test]
    fn parse_args_accepts_overrides() {
        let config = parse_args(vec![
            "--zip".to_string(),
            "custom.zip".to_string(),
            "--sha".to_string(),
            "custom.sha256".to_string(),
            "--workspace".to_string(),
            "workspace/current".to_string(),
        ])
        .expect("override args should parse");

        assert_eq!(
            config,
            Config {
                zip_path: PathBuf::from("custom.zip"),
                sha_path: PathBuf::from("custom.sha256"),
                workspace_path: PathBuf::from("workspace/current"),
            }
        );
    }

    #[test]
    fn validate_archive_member_set_rejects_missing_member() {
        let members = vec![
            "README.txt".to_string(),
            "governed_flow_report.json".to_string(),
            "package_index.json".to_string(),
            "replay_report.json".to_string(),
        ];

        let error = validate_archive_member_set(&members).expect_err("missing member should fail");
        assert!(error.contains("archive member set mismatch"));
    }

    #[test]
    fn validate_package_index_text_rejects_missing_reference() {
        let error = validate_package_index_text(
            r#"{"files":["governed_flow_report.json","replay_report.json","README.txt"]}"#,
        )
        .expect_err("missing replay bundle manifest should fail");

        assert!(error.contains("replay_bundle_manifest.json"));
    }

    #[test]
    fn render_import_receipt_contains_expected_fields() {
        let receipt = render_import_receipt(
            &Config {
                zip_path: PathBuf::from("target/proof_artifacts/slice14_export.zip"),
                sha_path: PathBuf::from("target/proof_artifacts/slice14_export.zip.sha256"),
                workspace_path: PathBuf::from(DEFAULT_WORKSPACE_PATH),
            },
            "abc123",
            &[
                "README.txt".to_string(),
                "governed_flow_report.json".to_string(),
                "package_index.json".to_string(),
                "replay_bundle_manifest.json".to_string(),
                "replay_report.json".to_string(),
            ],
        );

        assert!(receipt.contains("\"slice\": 16"));
        assert!(receipt.contains("\"source_digest\": \"abc123\""));
        assert!(receipt.contains("rehydrated/replay/replay_report.json"));
    }
}

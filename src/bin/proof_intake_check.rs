use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

const DEFAULT_ZIP_PATH: &str = "target/proof_artifacts/slice14_export.zip";
const DEFAULT_SHA_PATH: &str = "target/proof_artifacts/slice14_export.zip.sha256";
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
}

fn main() {
    if let Err(error) = run() {
        eprintln!("slice15 intake check failed: {error}");
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

    let package_index_text = read_archive_member_text(&config.zip_path, "package_index.json")?;
    validate_package_index_text(&package_index_text)?;

    println!("slice15 intake proof passed");
    println!("  zip: {}", config.zip_path.display());
    println!("  sha256: {}", config.sha_path.display());
    println!("  digest: {}", actual_digest);
    println!("  members: {}", archive_members.join(","));
    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<Config, String> {
    let mut zip_path = PathBuf::from(DEFAULT_ZIP_PATH);
    let mut sha_path = PathBuf::from(DEFAULT_SHA_PATH);

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
            "--help" | "-h" => {
                return Err(
                    "usage: cargo run --bin proof_intake_check -- [--zip PATH] [--sha PATH]"
                        .to_string(),
                );
            }
            other => {
                return Err(format!("unexpected argument: {other}"));
            }
        }
    }

    Ok(Config { zip_path, sha_path })
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

fn read_archive_member_text(zip_path: &Path, member_name: &str) -> Result<String, String> {
    let output = Command::new("unzip")
        .arg("-p")
        .arg(zip_path)
        .arg(member_name)
        .output()
        .map_err(|e| {
            format!(
                "failed to read archive member {} from {}: {e}",
                member_name,
                zip_path.display()
            )
        })?;

    if !output.status.success() {
        return Err(format!(
            "unzip -p returned non-zero status for member {} in {}",
            member_name,
            zip_path.display()
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("archive member {} produced invalid UTF-8: {e}", member_name))
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
        ])
        .expect("override args should parse");

        assert_eq!(
            config,
            Config {
                zip_path: PathBuf::from("custom.zip"),
                sha_path: PathBuf::from("custom.sha256"),
            }
        );
    }

    #[test]
    fn parse_sha256_file_rejects_wrong_file_name() {
        let error = parse_sha256_file(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  wrong.zip\n",
            Path::new("right.zip"),
        )
        .expect_err("wrong file name should fail");

        assert!(error.contains("file name mismatch"));
    }

    #[test]
    fn validate_archive_member_set_rejects_extra_member() {
        let members = vec![
            "README.txt".to_string(),
            "governed_flow_report.json".to_string(),
            "package_index.json".to_string(),
            "replay_bundle_manifest.json".to_string(),
            "replay_report.json".to_string(),
            "rogue.txt".to_string(),
        ];

        let error = validate_archive_member_set(&members).expect_err("extra member should fail");
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
}

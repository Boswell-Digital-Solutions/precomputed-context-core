use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const EXPORT_DIR: &str = "target/proof_artifacts/slice13_export";
const OUTPUT_ZIP: &str = "target/proof_artifacts/slice14_export.zip";
const OUTPUT_SHA256: &str = "target/proof_artifacts/slice14_export.zip.sha256";
const FIXED_TOUCH_TIMESTAMP: &str = "198001010000";
const REQUIRED_FILES: &[&str] = &[
    "governed_flow_report.json",
    "replay_report.json",
    "replay_bundle_manifest.json",
    "package_index.json",
    "README.txt",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("slice14 proof zip failed: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    ensure_command_available("zip")?;
    ensure_command_available("touch")?;
    ensure_command_available("sha256sum")?;

    let repo_root =
        env::current_dir().map_err(|e| format!("failed to read current directory: {e}"))?;
    let export_dir = repo_root.join(EXPORT_DIR);
    validate_export_dir(&export_dir)?;

    let relative_files = collect_relative_files(&export_dir)?;
    if relative_files.is_empty() {
        return Err(format!(
            "export directory is empty: {}",
            export_dir.display()
        ));
    }

    let stage_root = make_stage_root(&repo_root)?;
    copy_stage_files(&export_dir, &stage_root, &relative_files)?;
    normalize_stage_timestamps(&stage_root, &relative_files)?;

    let zip_path = repo_root.join(OUTPUT_ZIP);
    if zip_path.exists() {
        fs::remove_file(&zip_path)
            .map_err(|e| format!("failed to remove existing zip {}: {e}", zip_path.display()))?;
    }

    create_zip_archive(&stage_root, &relative_files, &zip_path)?;

    let sha_path = repo_root.join(OUTPUT_SHA256);
    write_sha256_file(&zip_path, &sha_path)?;

    cleanup_stage_root(&stage_root);

    println!("slice14 zip archive created");
    println!("  zip: {}", zip_path.display());
    println!("  sha256: {}", sha_path.display());
    Ok(())
}

fn ensure_command_available(command_name: &str) -> Result<(), String> {
    let status = Command::new(command_name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("required command '{command_name}' is unavailable: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "required command '{command_name}' returned a non-zero status"
        ))
    }
}

fn validate_export_dir(export_dir: &Path) -> Result<(), String> {
    if !export_dir.is_dir() {
        return Err(format!(
            "missing export directory: {}. Run 'cargo run --bin proof_check -- --export-package' first.",
            export_dir.display()
        ));
    }

    for required in REQUIRED_FILES {
        let required_path = export_dir.join(required);
        if !required_path.is_file() {
            return Err(format!(
                "required export member missing: {}",
                required_path.display()
            ));
        }
    }

    Ok(())
}

fn collect_relative_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .map_err(|e| format!("failed to read directory {}: {e}", dir.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("failed to enumerate directory {}: {e}", dir.display()))?;

        entries.sort_by_key(|entry| entry.path());

        for entry in entries {
            let path = entry.path();
            let metadata = entry
                .metadata()
                .map_err(|e| format!("failed to read metadata {}: {e}", path.display()))?;

            if metadata.is_dir() {
                walk(root, &path, out)?;
            } else if metadata.is_file() {
                let relative = path.strip_prefix(root).map_err(|e| {
                    format!("failed to derive relative path for {}: {e}", path.display())
                })?;
                out.push(relative.to_path_buf());
            }
        }

        Ok(())
    }

    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn make_stage_root(repo_root: &Path) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("system clock error: {e}"))?
        .as_nanos();

    let stage_root = repo_root
        .join("target")
        .join("proof_artifacts")
        .join(format!(".slice14_stage_{}_{}", process::id(), stamp));

    fs::create_dir_all(&stage_root).map_err(|e| {
        format!(
            "failed to create stage directory {}: {e}",
            stage_root.display()
        )
    })?;
    Ok(stage_root)
}

fn copy_stage_files(
    export_dir: &Path,
    stage_root: &Path,
    relative_files: &[PathBuf],
) -> Result<(), String> {
    for relative in relative_files {
        let source_path = export_dir.join(relative);
        let destination_path = stage_root.join(relative);

        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create stage parent {}: {e}", parent.display()))?;
        }

        fs::copy(&source_path, &destination_path).map_err(|e| {
            format!(
                "failed to copy {} to {}: {e}",
                source_path.display(),
                destination_path.display()
            )
        })?;
    }

    Ok(())
}

fn normalize_stage_timestamps(stage_root: &Path, relative_files: &[PathBuf]) -> Result<(), String> {
    for relative in relative_files {
        let status = Command::new("touch")
            .arg("-t")
            .arg(FIXED_TOUCH_TIMESTAMP)
            .arg(stage_root.join(relative))
            .status()
            .map_err(|e| format!("failed to execute touch for {}: {e}", relative.display()))?;

        if !status.success() {
            return Err(format!(
                "touch returned non-zero status for staged file {}",
                relative.display()
            ));
        }
    }

    Ok(())
}

fn create_zip_archive(
    stage_root: &Path,
    relative_files: &[PathBuf],
    zip_path: &Path,
) -> Result<(), String> {
    let mut command = Command::new("zip");
    command.current_dir(stage_root);
    command.arg("-q");
    command.arg("-X");
    command.arg(zip_path);

    for relative in relative_files {
        command.arg(relative);
    }

    let status = command
        .status()
        .map_err(|e| format!("failed to execute zip for {}: {e}", zip_path.display()))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "zip returned non-zero status for {}",
            zip_path.display()
        ))
    }
}

fn write_sha256_file(zip_path: &Path, sha_path: &Path) -> Result<(), String> {
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

    let file_name = zip_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid zip file name for {}", zip_path.display()))?;

    let content = format!("{digest}  {file_name}\n");
    fs::write(sha_path, content)
        .map_err(|e| format!("failed to write sha256 file {}: {e}", sha_path.display()))
}

fn cleanup_stage_root(stage_root: &Path) {
    let _ = fs::remove_dir_all(stage_root);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("valid clock")
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "precomputed_context_core_{}_{}_{}",
            label,
            process::id(),
            stamp
        ));
        fs::create_dir_all(&root).expect("temp dir should create");
        root
    }

    fn create_file(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent should create");
        }
        fs::write(path, b"ok").expect("file should write");
    }

    fn create_required_members(root: &Path) {
        for member in REQUIRED_FILES {
            create_file(&root.join(member));
        }
    }

    #[test]
    fn collect_relative_files_returns_sorted_paths() {
        let root = temp_dir("sorted_paths");
        create_required_members(&root);
        create_file(&root.join("z_last.txt"));
        create_file(&root.join("nested/a_first.txt"));
        create_file(&root.join("nested/z_last.txt"));

        let files = collect_relative_files(&root).expect("file collection should succeed");
        let display: Vec<String> = files
            .iter()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .collect();

        assert_eq!(
            display,
            vec![
                "README.txt",
                "governed_flow_report.json",
                "nested/a_first.txt",
                "nested/z_last.txt",
                "package_index.json",
                "replay_bundle_manifest.json",
                "replay_report.json",
                "z_last.txt",
            ]
        );

        fs::remove_dir_all(root).expect("temp dir should clean up");
    }

    #[test]
    fn validate_export_dir_fails_closed_when_required_member_is_missing() {
        let root = temp_dir("missing_member");
        create_required_members(&root);
        fs::remove_file(root.join("package_index.json")).expect("fixture should remove");

        let error = validate_export_dir(&root).expect_err("validation should fail");
        assert!(error.contains("package_index.json"));

        fs::remove_dir_all(root).expect("temp dir should clean up");
    }

    #[test]
    fn validate_export_dir_accepts_complete_required_members() {
        let root = temp_dir("complete_members");
        create_required_members(&root);

        validate_export_dir(&root).expect("validation should succeed");

        fs::remove_dir_all(root).expect("temp dir should clean up");
    }
}

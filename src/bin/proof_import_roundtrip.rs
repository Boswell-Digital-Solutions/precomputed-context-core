use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

use precomputed_context_core::import_contract::{
    default_roundtrip_report_path, render_roundtrip_report, validate_import_workspace,
};

const DEFAULT_WORKSPACE_PATH: &str = "target/proof_artifacts/slice16_import/current";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Config {
    workspace_path: PathBuf,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("slice17 import roundtrip failed: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = parse_args(env::args().skip(1).collect())?;

    if !config.workspace_path.is_dir() {
        return Err(format!(
            "missing workspace directory: {}",
            config.workspace_path.display()
        ));
    }

    let report = validate_import_workspace(&config.workspace_path)?;
    let report_path = default_roundtrip_report_path(&config.workspace_path);
    let report_text = render_roundtrip_report(&report);

    fs::write(&report_path, report_text).map_err(|e| {
        format!(
            "failed to write roundtrip report {}: {e}",
            report_path.display()
        )
    })?;

    println!("slice17 import roundtrip passed");
    println!("  workspace: {}", config.workspace_path.display());
    println!("  report: {}", report_path.display());
    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<Config, String> {
    let mut workspace_path = PathBuf::from(DEFAULT_WORKSPACE_PATH);

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value after --workspace".to_string())?;
                workspace_path = PathBuf::from(value);
                index += 2;
            }
            "--help" | "-h" => {
                return Err(
                    "usage: cargo run --bin proof_import_roundtrip -- [--workspace PATH]"
                        .to_string(),
                );
            }
            other => {
                return Err(format!("unexpected argument: {other}"));
            }
        }
    }

    Ok(Config { workspace_path })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_uses_default_workspace() {
        let config = parse_args(vec![]).expect("defaults should parse");
        assert_eq!(
            config,
            Config {
                workspace_path: PathBuf::from(DEFAULT_WORKSPACE_PATH),
            }
        );
    }

    #[test]
    fn parse_args_accepts_override() {
        let config = parse_args(vec!["--workspace".to_string(), "tmp/workspace".to_string()])
            .expect("override should parse");
        assert_eq!(
            config,
            Config {
                workspace_path: PathBuf::from("tmp/workspace"),
            }
        );
    }
}

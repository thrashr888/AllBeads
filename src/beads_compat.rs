use crate::config::BossContext;
use crate::graph::Bead;
use crate::storage::issues_to_beads;
use crate::{AllBeadsError, Result};
use beads::Beads;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BdContextInfo {
    pub beads_dir: Option<PathBuf>,
    pub backend: Option<String>,
    pub dolt_mode: Option<String>,
    pub database: Option<String>,
    pub bd_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsCapabilityReport {
    pub context_name: String,
    pub path: PathBuf,
    pub has_beads_dir: bool,
    pub has_issues_jsonl: bool,
    pub has_legacy_beads_db: bool,
    pub bd_available: bool,
    pub bd_context_ok: bool,
    pub backend: Option<String>,
    pub dolt_mode: Option<String>,
    pub bd_version: Option<String>,
    pub has_dolt_remote: bool,
    pub bd_sync_available: bool,
    pub issues_jsonl_is_legacy_mirror: bool,
    pub problems: Vec<String>,
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsSyncOutcome {
    pub context_name: String,
    pub path: PathBuf,
    pub pulled: bool,
    pub push_configured: bool,
    pub pull_message: String,
    pub push_message: String,
}

fn run_bd(path: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("bd").args(args).current_dir(path).output()
}

pub fn is_bd_available() -> bool {
    Command::new("bd").arg("--version").output().is_ok()
}

pub fn bd_context(path: &Path) -> Result<BdContextInfo> {
    let output = run_bd(path, &["context", "--json"])
        .map_err(|e| AllBeadsError::Config(format!("Failed to run 'bd context --json': {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AllBeadsError::Config(format!(
            "bd context failed in {}: {}",
            path.display(),
            stderr.trim()
        )));
    }

    #[derive(Debug, Deserialize)]
    struct RawBdContextInfo {
        beads_dir: Option<PathBuf>,
        backend: Option<String>,
        dolt_mode: Option<String>,
        database: Option<String>,
        bd_version: Option<String>,
    }

    let parsed: RawBdContextInfo = serde_json::from_slice(&output.stdout)?;
    Ok(BdContextInfo {
        beads_dir: parsed.beads_dir,
        backend: parsed.backend,
        dolt_mode: parsed.dolt_mode,
        database: parsed.database,
        bd_version: parsed.bd_version,
    })
}

pub fn has_dolt_remote(path: &Path) -> bool {
    match run_bd(path, &["dolt", "remote", "list"]) {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .any(|line| line.trim_start().starts_with("origin"))
        }
        _ => false,
    }
}

pub fn supports_bd_sync(path: &Path) -> bool {
    match run_bd(path, &["sync", "--help"]) {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

pub fn list_all_beads(path: &Path) -> Result<Vec<Bead>> {
    let bd = Beads::with_workdir(path);
    let output = bd
        .run(&["list", "--status", "all", "--json"])
        .map_err(|e| {
            AllBeadsError::Storage(format!("Failed to list beads in {}: {}", path.display(), e))
        })?;

    match serde_json::from_str::<Vec<beads::Issue>>(&output.stdout) {
        Ok(issues) => issues_to_beads(issues),
        Err(issue_parse_error) => serde_json::from_str::<Vec<Bead>>(&output.stdout).map_err(|bead_parse_error| {
            AllBeadsError::Parse(format!(
                "Failed to parse 'bd list --status all --json' output in {} as either beads::Issue ({}) or graph::Bead ({})",
                path.display(),
                issue_parse_error,
                bead_parse_error
            ))
        }),
    }
}

pub fn sync_context(path: &Path, context_name: &str) -> Result<BeadsSyncOutcome> {
    let remote_configured = has_dolt_remote(path);

    if !remote_configured {
        return Ok(BeadsSyncOutcome {
            context_name: context_name.to_string(),
            path: path.to_path_buf(),
            pulled: false,
            push_configured: false,
            pull_message: "No Dolt remote configured; skipped bd dolt pull".to_string(),
            push_message: "No Dolt remote configured".to_string(),
        });
    }

    let pull = run_bd(path, &["dolt", "pull"]).map_err(|e| {
        AllBeadsError::Config(format!(
            "Failed to run 'bd dolt pull' in {}: {}",
            path.display(),
            e
        ))
    })?;

    let pulled = pull.status.success();
    let pull_message = if pulled {
        let stdout = String::from_utf8_lossy(&pull.stdout).trim().to_string();
        if stdout.is_empty() {
            "Pulled latest beads state from Dolt remote".to_string()
        } else {
            stdout
        }
    } else {
        let stderr = String::from_utf8_lossy(&pull.stderr).trim().to_string();
        if stderr.is_empty() {
            "bd dolt pull failed".to_string()
        } else {
            stderr
        }
    };

    Ok(BeadsSyncOutcome {
        context_name: context_name.to_string(),
        path: path.to_path_buf(),
        pulled,
        push_configured: true,
        pull_message,
        push_message: "Use 'bd dolt push' after local writes when a remote is configured"
            .to_string(),
    })
}

pub fn inspect_context(context: &BossContext) -> BeadsCapabilityReport {
    let path = context.path.clone().unwrap_or_else(|| context.get_path());
    let beads_dir = path.join(".beads");
    let has_beads_dir = beads_dir.exists();
    let has_issues_jsonl = beads_dir.join("issues.jsonl").exists();
    let has_legacy_beads_db = beads_dir.join("beads.db").exists();
    let bd_available = is_bd_available();

    let (bd_context_ok, backend, dolt_mode, bd_version) = if bd_available && has_beads_dir {
        match bd_context(&path) {
            Ok(info) => (true, info.backend, info.dolt_mode, info.bd_version),
            Err(_) => (false, None, None, None),
        }
    } else {
        (false, None, None, None)
    };

    let has_dolt_remote = if bd_context_ok {
        has_dolt_remote(&path)
    } else {
        false
    };
    let bd_sync_available = if bd_available && has_beads_dir {
        supports_bd_sync(&path)
    } else {
        false
    };

    let mut problems = Vec::new();
    let mut recommended_actions = Vec::new();

    if !has_beads_dir {
        problems.push("No .beads directory found".to_string());
        recommended_actions.push(
            "Run 'bd init' in this context before relying on AllBeads aggregation".to_string(),
        );
    }

    if has_legacy_beads_db {
        problems.push("Legacy .beads/beads.db detected; official beads now uses Dolt".to_string());
        recommended_actions.push(
            "Treat legacy SQLite artifacts as migration signals, not source-of-truth".to_string(),
        );
    }

    if has_issues_jsonl {
        problems.push(".beads/issues.jsonl present; JSONL is now a mirror/export format, not authoritative storage".to_string());
        recommended_actions.push(
            "Stop reading issues.jsonl as the primary data source; query 'bd ... --json' instead"
                .to_string(),
        );
    }

    if bd_available && !bd_sync_available {
        problems.push("'bd sync' is unavailable in the installed beads CLI".to_string());
        recommended_actions.push(
            "Replace 'bd sync' flows with 'bd dolt pull'/'bd dolt push' semantics".to_string(),
        );
    }

    if has_beads_dir && bd_available && !bd_context_ok {
        problems.push("'bd context --json' failed for this context".to_string());
        recommended_actions.push(
            "Run 'bd doctor' in the context and verify the Dolt-backed beads installation"
                .to_string(),
        );
    }

    if bd_context_ok && backend.as_deref() != Some("dolt") {
        problems.push(format!(
            "Unexpected backend reported by bd: {}",
            backend.as_deref().unwrap_or("unknown")
        ));
        recommended_actions
            .push("Verify compatibility with the official Dolt-backed beads CLI".to_string());
    }

    if bd_context_ok && !has_dolt_remote {
        recommended_actions.push("No Dolt remote configured; AllBeads can read local state but cannot pull/push shared beads state".to_string());
    }

    BeadsCapabilityReport {
        context_name: context.name.clone(),
        path,
        has_beads_dir,
        has_issues_jsonl,
        has_legacy_beads_db,
        bd_available,
        bd_context_ok,
        backend,
        dolt_mode,
        bd_version,
        has_dolt_remote,
        bd_sync_available,
        issues_jsonl_is_legacy_mirror: has_issues_jsonl,
        problems,
        recommended_actions,
    }
}

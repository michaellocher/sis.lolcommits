//! Google Drive setup without a terminal:
//! 1. Google Drive for desktop: shared drives are plain folders under
//!    ~/Library/CloudStorage/GoogleDrive-<account>/<Shared drives>/ – no login needed.
//! 2. Otherwise rclone: browser login, then pick a shared drive from a list.
use crate::config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// "Shared drives" in the languages Drive for desktop uses most.
const SHARED_NAMES: [&str; 9] = [
    "Shared drives", "Geteilte Ablagen", "Drive partagés", "Unidades compartidas", "Drive condivisi",
    "Gedeelde drives", "Drives compartilhados", "Udostępnione dyski", "Delade enheter",
];
const MY_NAMES: [&str; 8] = ["My Drive", "Meine Ablage", "Mon Drive", "Mi unidad", "Il mio Drive", "Mijn Drive", "Meu Drive", "Other computers"];

#[derive(Serialize)]
pub struct DriveFolder {
    pub account: String,
    pub name: String,
    pub path: String,
}

fn cloud_storage() -> PathBuf {
    std::env::var_os("LOLCOMMITS_CLOUDSTORAGE")
        .map(PathBuf::from)
        .unwrap_or_else(|| config::home().join("Library/CloudStorage"))
}

fn dirs(p: &PathBuf) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = fs::read_dir(p)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| (e.file_name().to_string_lossy().to_string(), e.path()))
                .filter(|(n, _)| !n.starts_with('.'))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Shared drives that Google Drive for desktop has mounted.
pub fn folders() -> Vec<DriveFolder> {
    let mut out = Vec::new();
    for (acct_dir, acct_path) in dirs(&cloud_storage()) {
        let Some(account) = acct_dir.strip_prefix("GoogleDrive-") else { continue };
        let top = dirs(&acct_path);
        let containers: Vec<PathBuf> = {
            let known: Vec<PathBuf> = top.iter().filter(|(n, _)| SHARED_NAMES.contains(&n.as_str())).map(|(_, p)| p.clone()).collect();
            if !known.is_empty() {
                known
            } else {
                // unknown language: anything that isn't "My Drive" or a shortcut store
                top.iter()
                    .filter(|(n, _)| !MY_NAMES.contains(&n.as_str()) && !n.contains("shortcut-targets"))
                    .map(|(_, p)| p.clone())
                    .collect()
            }
        };
        for c in containers {
            for (name, path) in dirs(&c) {
                out.push(DriveFolder { account: account.to_string(), name, path: path.to_string_lossy().to_string() });
            }
        }
    }
    out
}

pub const REMOTE: &str = "lolcommits-gdrive";

pub fn rclone_installed() -> bool {
    config::which("rclone").is_some()
}

fn rclone(args: &[&str]) -> Result<String, String> {
    let bin = config::which("rclone").ok_or("rclone is not installed")?;
    let out = config::cmd(bin.to_str().unwrap()).args(args).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("rclone failed").to_string())
    }
}

pub fn remote_exists() -> bool {
    rclone(&["listremotes"]).map(|o| o.lines().any(|l| l.trim() == format!("{REMOTE}:"))).unwrap_or(false)
}

#[derive(Serialize, Deserialize)]
pub struct SharedDrive {
    pub id: String,
    pub name: String,
}

/// Browser login (rclone opens it and waits for Google's redirect on localhost).
pub fn connect() -> Result<Vec<SharedDrive>, String> {
    if !remote_exists() {
        rclone(&["config", "create", REMOTE, "drive", "scope=drive", "config_is_local=true", "config_change_team_drive=false"])?;
    }
    shared_drives()
}

pub fn shared_drives() -> Result<Vec<SharedDrive>, String> {
    let out = rclone(&["backend", "drives", &format!("{REMOTE}:")])?;
    parse_drives(&out)
}

pub fn parse_drives(json: &str) -> Result<Vec<SharedDrive>, String> {
    #[derive(Deserialize)]
    struct Raw {
        id: String,
        name: String,
    }
    let raw: Vec<Raw> = serde_json::from_str(json).map_err(|e| format!("unexpected rclone output: {e}"))?;
    Ok(raw.into_iter().map(|r| SharedDrive { id: r.id, name: r.name }).collect())
}

/// Point the remote at one shared drive; returns the share target.
pub fn use_shared_drive(id: &str) -> Result<String, String> {
    rclone(&["config", "update", REMOTE, &format!("team_drive={id}"), "root_folder_id="])?;
    Ok(format!("rclone:{REMOTE}:lolcommits"))
}

pub fn disconnect() {
    let _ = rclone(&["config", "delete", REMOTE]);
}

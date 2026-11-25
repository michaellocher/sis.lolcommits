//! The team gallery. Same layout as the CLI: <target>/<person>/<YYYYmmdd>-<rand>.jpg
//! plus <person>/meta.tsv (date, name, repo or "-", subject, file).
//! Targets: a folder (Google Drive for desktop, NAS) or an rclone remote.
use crate::store::{self, Shot};
use crate::{config, render};
use rand::Rng;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const RCLONE_FLAGS: [&str; 1] = ["--drive-use-trash=false"];

#[derive(Debug, Clone)]
pub enum Target {
    Folder(PathBuf),
    Rclone(String),
}

pub fn target() -> Result<Target, String> {
    let t = config::get("share.target").ok_or("sharing is not set up")?;
    if let Some(r) = t.strip_prefix("rclone:") {
        return Ok(Target::Rclone(r.to_string()));
    }
    let expanded = if let Some(rest) = t.strip_prefix("~/") { config::home().join(rest) } else { PathBuf::from(&t) };
    if t.contains(':') && !t.starts_with('/') && !t.starts_with('~') {
        return Err(format!("{t} is an ssh target – those work with the lolcommits command line tool only"));
    }
    Ok(Target::Folder(expanded))
}

pub fn configured() -> bool {
    target().is_ok() && config::get("share.name").is_some()
}

/// Folder name for a display name, the same as slug() in the CLI.
pub fn slug(name: &str) -> String {
    let mut s = name.to_string();
    for (a, b) in [("ä", "ae"), ("Ä", "ae"), ("ö", "oe"), ("Ö", "oe"), ("ü", "ue"), ("Ü", "ue"), ("ß", "ss")] {
        s = s.replace(a, b);
    }
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn me() -> Result<(String, String), String> {
    let name = config::get("share.name").ok_or("set your name first")?;
    let s = slug(&name);
    if s.is_empty() {
        return Err("your name needs at least one letter or digit".into());
    }
    Ok((name, s))
}

fn rclone(args: &[&str]) -> Result<String, String> {
    let bin = config::which("rclone").ok_or("rclone is not installed")?;
    let out = config::cmd(bin.to_str().unwrap()).args(args).args(RCLONE_FLAGS).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("rclone failed").to_string())
    }
}

fn put(t: &Target, src: &Path, rel: &str) -> Result<(), String> {
    match t {
        Target::Folder(base) => {
            let dst = base.join(rel);
            fs::create_dir_all(dst.parent().unwrap()).map_err(|e| format!("{}: {e}", base.display()))?;
            fs::copy(src, &dst).map(|_| ()).map_err(|e| format!("{}: {e}", dst.display()))
        }
        Target::Rclone(base) => rclone(&["copyto", src.to_str().unwrap(), &format!("{base}/{rel}")]).map(|_| ()),
    }
}

fn cat(t: &Target, rel: &str) -> String {
    match t {
        Target::Folder(base) => fs::read_to_string(base.join(rel)).unwrap_or_default(),
        Target::Rclone(base) => rclone(&["cat", &format!("{base}/{rel}")]).unwrap_or_default(),
    }
}

fn rm(t: &Target, rel: &str) -> Result<(), String> {
    match t {
        Target::Folder(base) => match fs::remove_file(base.join(rel)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        },
        Target::Rclone(base) => rclone(&["deletefile", &format!("{base}/{rel}")]).map(|_| ()),
    }
}

fn purge(t: &Target, rel: &str) -> Result<(), String> {
    match t {
        Target::Folder(base) => match fs::remove_dir_all(base.join(rel)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        },
        Target::Rclone(base) => {
            if rclone(&["lsf", &format!("{base}/{rel}")]).is_err() {
                return Ok(()); // already gone
            }
            rclone(&["purge", &format!("{base}/{rel}")]).map(|_| ())
        }
    }
}

fn tmp_file(name: &str) -> PathBuf {
    let d = config::lol_dir().join(".preview");
    let _ = fs::create_dir_all(&d);
    d.join(name)
}

/// The picture as it will be uploaded: no metadata, repo band blurred unless share.repo=true.
pub fn prepare(file: &str) -> Result<PathBuf, String> {
    let img = render::load(Path::new(file))?;
    let img = if config::get_or("share.repo", "false") == "true" { img } else { render::blur_band(&img) };
    let out = tmp_file(&format!("{:08x}.jpg", rand::thread_rng().gen::<u32>()));
    render::save_jpeg(&img, &out)?;
    Ok(out)
}

pub fn clear_previews() {
    let _ = fs::remove_dir_all(config::lol_dir().join(".preview"));
}

pub fn upload(shot: &Shot) -> Result<(), String> {
    let t = target()?;
    let (name, me) = me()?;
    if store::shared_rows().iter().any(|(l, _)| l == &shot.file) {
        return Err("already shared".into());
    }
    let img = prepare(&shot.file)?;
    let base = Path::new(&shot.file).file_name().unwrap().to_string_lossy().to_string();
    let d = base.get(..8).unwrap_or("00000000").to_string();
    let rel = format!("{me}/{d}-{:08x}.jpg", rand::thread_rng().gen::<u32>());
    let repo = if config::get_or("share.repo", "false") == "true" { shot.repo.clone() } else { "-".into() };
    let mut meta = cat(&t, &format!("{me}/meta.tsv"));
    meta.push_str(&format!(
        "{}-{}-{}\t{}\t{}\t{}\t{}\n",
        &d[..4],
        &d[4..6],
        &d[6..8],
        name.replace('\t', " "),
        repo,
        shot.subject.replace('\t', " "),
        rel.rsplit('/').next().unwrap()
    ));
    let meta_tmp = tmp_file("meta.tsv");
    fs::write(&meta_tmp, meta).map_err(|e| e.to_string())?;
    let res = put(&t, &img, &rel).and_then(|_| put(&t, &meta_tmp, &format!("{me}/meta.tsv")));
    let _ = fs::remove_file(&img);
    let _ = fs::remove_file(&meta_tmp);
    res?;
    store::add_shared(&shot.file, &rel).map_err(|e| e.to_string())?;
    prune();
    Ok(())
}

/// lolcommits.share.retention days – off (0) unless the user sets it.
fn prune() {
    let days: i64 = config::get("share.retention").and_then(|d| d.parse().ok()).unwrap_or(0);
    if days <= 0 {
        return;
    }
    let (Ok(t), Ok((_, me))) = (target(), me()) else { return };
    let cutoff = (chrono::Local::now() - chrono::Duration::days(days)).format("%Y-%m-%d").to_string();
    let meta = cat(&t, &format!("{me}/meta.tsv"));
    let (old, keep): (Vec<&str>, Vec<&str>) = meta.lines().partition(|l| l.split('\t').next().unwrap_or("") < cutoff.as_str());
    if old.is_empty() {
        return;
    }
    for l in &old {
        if let Some(f) = l.split('\t').nth(4) {
            let rel = format!("{me}/{f}");
            let _ = rm(&t, &rel);
            let _ = store::remove_shared(|_, r| r == rel);
        }
    }
    let tmp = tmp_file("meta.tsv");
    let body: String = keep.iter().map(|l| format!("{l}\n")).collect();
    if fs::write(&tmp, body).is_ok() {
        let _ = put(&t, &tmp, &format!("{me}/meta.tsv"));
    }
    let _ = fs::remove_file(tmp);
}

pub fn unshare(file: &str) -> Result<(), String> {
    let t = target()?;
    let (_, me) = me()?;
    let rel = store::shared_rows().into_iter().find(|(l, _)| l == file).map(|r| r.1).ok_or("this picture is not shared")?;
    rm(&t, &rel)?;
    let fname = rel.rsplit('/').next().unwrap_or("").to_string();
    let meta: String = cat(&t, &format!("{me}/meta.tsv"))
        .lines()
        .filter(|l| l.split('\t').nth(4) != Some(fname.as_str()))
        .map(|l| format!("{l}\n"))
        .collect();
    let tmp = tmp_file("meta.tsv");
    fs::write(&tmp, meta).map_err(|e| e.to_string())?;
    put(&t, &tmp, &format!("{me}/meta.tsv"))?;
    let _ = fs::remove_file(tmp);
    store::remove_shared(|l, _| l == file).map_err(|e| e.to_string())
}

pub fn unshare_all() -> Result<(), String> {
    let t = target()?;
    let (_, me) = me()?;
    purge(&t, &me)?;
    let _ = fs::remove_file(store::shared_path());
    Ok(())
}

#[derive(Serialize)]
pub struct TeamRow {
    pub date: String,
    pub name: String,
    pub repo: String,
    pub subject: String,
    pub file: String,
    pub mine: bool,
}

/// Folder to read team pictures from: the target itself for folders,
/// a downloaded copy for rclone remotes.
pub fn team_dir() -> Result<PathBuf, String> {
    Ok(match target()? {
        Target::Folder(p) => p,
        Target::Rclone(_) => config::lol_dir().join(".shared"),
    })
}

pub fn sync() -> Result<(), String> {
    if let Target::Rclone(base) = target()? {
        let dir = team_dir()?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        rclone(&["sync", &base, dir.to_str().unwrap()])?;
    }
    Ok(())
}

pub fn team() -> Result<Vec<TeamRow>, String> {
    let dir = team_dir()?;
    let me = config::get("share.name").map(|n| slug(&n)).unwrap_or_default();
    let days: i64 = config::get("share.retention").and_then(|d| d.parse().ok()).unwrap_or(0);
    let cutoff = if days > 0 { (chrono::Local::now() - chrono::Duration::days(days)).format("%Y-%m-%d").to_string() } else { String::new() };
    let mut rows = Vec::new();
    for e in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten() {
        let person = e.path();
        for r in store::tsv(&person.join("meta.tsv"), 5) {
            let f = person.join(&r[4]);
            if r[0] >= cutoff && f.is_file() {
                rows.push(TeamRow {
                    date: r[0].clone(),
                    name: r[1].clone(),
                    repo: if r[2] == "-" { String::new() } else { r[2].clone() },
                    subject: r[3].clone(),
                    file: f.to_string_lossy().to_string(),
                    mine: e.file_name().to_string_lossy() == me,
                });
            }
        }
    }
    rows.sort_by(|a, b| (&b.date, &b.file).cmp(&(&a.date, &a.file)));
    Ok(rows)
}

/// Can we write to the target? Writes and deletes a small file.
pub fn probe(t: &Target) -> Result<(), String> {
    let tmp = tmp_file("probe.txt");
    fs::write(&tmp, "lolcommits connection test\n").map_err(|e| e.to_string())?;
    let rel = format!(".probe-{:08x}", rand::thread_rng().gen::<u32>());
    let res = put(t, &tmp, &rel).and_then(|_| rm(t, &rel));
    let _ = fs::remove_file(tmp);
    res.map_err(|e| format!("can't write there: {e}"))
}

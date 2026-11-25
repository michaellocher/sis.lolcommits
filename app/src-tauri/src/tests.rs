//! End-to-end tests of the app logic against a temporary home: fake Google
//! Drive for desktop folder, own git config, test pattern instead of the camera.
use crate::store::Shot;
use crate::{capture, config, drive, render, share, store};
use std::fs;
use std::path::PathBuf;

fn sandbox() -> PathBuf {
    let root = std::env::temp_dir().join(format!("lolcommits-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("cloud/GoogleDrive-kim@example.com/Geteilte Ablagen/Team Fun")).unwrap();
    fs::create_dir_all(root.join("cloud/GoogleDrive-kim@example.com/Meine Ablage/private")).unwrap();
    std::env::set_var("LOLCOMMITS_DIR", root.join("lol"));
    std::env::set_var("GIT_CONFIG_GLOBAL", root.join("gitconfig"));
    std::env::set_var("LOLCOMMITS_TEST_PATTERN", "1");
    std::env::set_var("LOLCOMMITS_CLOUDSTORAGE", root.join("cloud"));
    root
}

fn shoot(subject: &str, stem: &str) -> Shot {
    let img = render::caption(render::fit(capture::grab().unwrap()), "abc1234567", subject, "repo", "main");
    let out = config::lol_dir().join("repo").join(format!("{stem}.jpg"));
    render::save_jpeg(&img, &out).unwrap();
    let shot = Shot {
        ts: "2026-11-25T10:00:00+01:00".into(),
        repo: "repo".into(),
        branch: "main".into(),
        sha: "abc1234567".into(),
        subject: subject.into(),
        file: out.to_string_lossy().into(),
        shared: false,
    };
    store::append(&shot).unwrap();
    shot
}

#[test]
fn full_flow() {
    let root = sandbox();

    // Drive for desktop: only shared drives are offered, not "Meine Ablage"
    let folders = drive::folders();
    assert_eq!(folders.len(), 1, "{:?}", folders.iter().map(|f| &f.name).collect::<Vec<_>>());
    assert_eq!(folders[0].name, "Team Fun");
    assert_eq!(folders[0].account, "kim@example.com");

    // capture → local store, no EXIF
    let a = shoot("first: tabs\tand stuff", "20261125-100000-abc1234567");
    let b = shoot("second", "20261125-100100-abc1234567");
    assert_eq!(store::shots().len(), 2);
    let bytes = fs::read(&a.file).unwrap();
    assert!(!bytes.windows(4).any(|w| w == b"Exif"));

    // connect the shared drive folder, umlaut name
    let target = PathBuf::from(&folders[0].path).join("lolcommits");
    fs::create_dir_all(&target).unwrap();
    share::probe(&share::Target::Folder(target.clone())).unwrap();
    config::set("share.target", target.to_str().unwrap()).unwrap();
    config::set("share.name", "Jürgen Groß").unwrap();
    assert!(share::configured());

    // share two, one is refused twice
    share::upload(&a).unwrap();
    share::upload(&b).unwrap();
    assert!(share::upload(&a).is_err());
    let me = target.join("juergen-gross");
    let meta = fs::read_to_string(me.join("meta.tsv")).unwrap();
    assert_eq!(meta.lines().count(), 2, "{meta}");
    assert!(meta.lines().all(|l| l.split('\t').nth(2) == Some("-")), "repo hidden by default: {meta}");
    assert!(meta.contains("first: tabs and stuff"));
    let team = share::team().unwrap();
    assert_eq!(team.len(), 2);
    assert!(team.iter().all(|r| r.mine && r.name == "Jürgen Groß"));
    assert!(store::shots().iter().all(|s| s.shared));

    // no expiry by default: an old row survives the next upload
    fs::write(me.join("old.jpg"), b"x").unwrap();
    let mut m = fs::read_to_string(me.join("meta.tsv")).unwrap();
    m.push_str("2020-01-01\tJürgen Groß\t-\told one\told.jpg\n");
    fs::write(me.join("meta.tsv"), m).unwrap();
    let c = shoot("third", "20261125-100200-abc1234567");
    share::upload(&c).unwrap();
    assert!(me.join("old.jpg").exists(), "default must not expire shared pictures");

    // with an expiry set it is removed
    config::set("share.retention", "30").unwrap();
    let d = shoot("fourth", "20261125-100300-abc1234567");
    share::upload(&d).unwrap();
    assert!(!me.join("old.jpg").exists());
    assert!(!fs::read_to_string(me.join("meta.tsv")).unwrap().contains("old one"));

    // unshare one
    share::unshare(&a.file).unwrap();
    assert_eq!(share::team().unwrap().len(), 3);
    assert!(!store::find(&a.file).unwrap().shared);

    // unshare all removes the person folder
    share::unshare_all().unwrap();
    assert!(!me.exists());
    assert!(store::shared_rows().is_empty());

    // contact sheet, then forget everything
    let files: Vec<String> = store::shots().into_iter().map(|s| s.file).collect();
    let sheet = render::contact_sheet(&files).unwrap();
    assert!(sheet.width() > 320);
    store::forget_all();
    assert!(store::shots().is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn parses_rclone_drives() {
    let d = drive::parse_drives(r#"[{"id":"0AB","kind":"drive#teamDrive","name":"lolcommits"},{"id":"0CD","kind":"drive#teamDrive","name":"Eng"}]"#).unwrap();
    assert_eq!(d.len(), 2);
    assert_eq!(d[0].name, "lolcommits");
    assert!(drive::parse_drives("not json").is_err());
}

#[test]
fn slugs_match_the_cli() {
    assert_eq!(share::slug("Jürgen Groß"), "juergen-gross");
    assert_eq!(share::slug("Ä Person"), "ae-person");
    assert_eq!(share::slug("  --Kim!! "), "kim");
}

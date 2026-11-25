# lolcommits (2027 edition)

A webcam selfie for every git commit, with the commit message on it in big meme letters.
Remake of the old [lolcommits](https://github.com/lolcommits/lolcommits) Ruby gem as a menu bar app for macOS (and Linux).

- **Download, open, done:** a setup assistant handles the camera, the git hook and the team gallery. No terminal, no Homebrew.
- **Lives in the menu bar:** the icon flashes when it takes a photo. From the menu you can pause (1 hour, until tomorrow, until you resume), take a test photo, or share your latest one.
- **Doesn't slow down commits:** `git commit` returns immediately and the app takes the photo in the background.
- **One camera permission:** the app owns the camera, so your terminal, IDE and git client don't each ask for it.
- **Team gallery, privacy first:** share to a Google shared drive. You pick each picture after a preview, and one click takes it all back ([details](#team-gallery)).
- **Extras:** gallery with search, timelapse video, year-in-review contact sheet ("wrapped").

## Install

1. Download `lolcommits.dmg` from the [latest release](https://github.com/simon-scherzinger/sis.lolcommits/releases) and move lolcommits to Applications.
2. Open it. The assistant walks you through:
   - **Fair use:** what stays on your Mac and what is shared (GDPR versus team spirit, in short)
   - **Camera:** allow it once and take a test photo
   - **On:** take a photo on every commit, in all repos (needs git 2.54+, which ships with current macOS)
   - **Team gallery (optional):** pick a shared drive from Google Drive for desktop, or sign in with Google
3. Commit something.

On **Linux** use the `.AppImage` or `.deb` from the release.
Without Google Drive for desktop, "Sign in with Google" works through [rclone](https://rclone.org), and the app offers to install it.

**With Claude Code:** tell it

> Install lolcommits following https://github.com/simon-scherzinger/sis.lolcommits/blob/main/INSTALL.md

## Team gallery

Pictures stay on your computer unless you share them. Sharing is opt-in per person.

**Setup for the team (once):** create a Google shared drive, e.g. "lolcommits", and add the team's Google group.
Use a shared drive rather than someone's My Drive, so the pictures belong to the company and not to one person.

**Setup for each person:** in the app, open Team › Set up team gallery and pick the shared drive.
- **With Google Drive for desktop:** its shared drives show up right away, with no sign-in.
- **Without it:** "Sign in with Google" opens the browser. If your Workspace blocks rclone's sign-in, an admin has to allow it ([rclone docs](https://rclone.org/drive/#making-your-own-client-id)).

**What gets uploaded:**
- the picture without metadata, with the repo name and commit hash blurred
- the date (no time of day), your name and the commit message

Each person writes only to their own folder.
Shared pictures stay until you take them back, unless you set an expiry in Settings.
Your team should agree on a period, see [PRIVACY.md](PRIVACY.md).

### Privacy

Faces plus commit times are personal data, and in a company they are employee data.
lolcommits is built so that the team gallery stays fun and doesn't turn into monitoring:

- opt-in per person, a preview with two checks before every upload, nicknames allowed
- withdrawal is one click (Privacy › Unshare all)
- no metadata, repo names blurred by default
- dates only, no times, no statistics per person
- no face or emotion recognition, ever

Before rolling it out in a company, read [PRIVACY.md](PRIVACY.md). It has a checklist and a notice template for your team.

## Command line

The app bundles the `lolcommits` script. Settings › App › "Install `lolcommits` command" puts it on your PATH.
It also works on its own, without the app, on macOS and Linux with `ffmpeg` (plus `imagemagick` on macOS for captions).

```bash
lolcommits enable --global   # every repo (git 2.54+)
lolcommits enable            # just this repo (any git version)
lolcommits disable [--global]
lolcommits status | doctor

lolcommits last              # open the latest shot
lolcommits gallery           # HTML gallery
lolcommits timelapse [repo]  # mp4 (needs ffmpeg)
lolcommits wrapped [year]    # contact sheet of the year

lolcommits share [file]      # preview, then upload
lolcommits unshare [file|--all]
lolcommits forget <file|last|--all>
lolcommits ui                # open the app
```

- **Skip one commit:** `LOLCOMMITS_SKIP=1 git commit …`
- **Pause in one repo:** `git config lolcommits.enabled false`
- **With the app installed:** the hook only queues a job in `~/.lolcommits/queue/`. The app takes the photo.

### Config

Everything is plain git config, shared by the app and the command line:

| key | default | |
|---|---|---|
| `lolcommits.device` | `0` (macOS) / `/dev/video0` (Linux) | camera |
| `lolcommits.delay` | `1.5` | seconds of camera warm-up |
| `lolcommits.dir` | `~/.lolcommits` | where pictures go |
| `lolcommits.enabled` | `true` | `false` pauses without removing the hook |
| `lolcommits.pausedUntil` | – | unix time, set by "Pause" in the menu bar |
| `lolcommits.retention` | keep | delete local pictures after N days |
| `lolcommits.share.target` | off | a folder (Drive for desktop, NAS) or `rclone:<remote>:<path>` |
| `lolcommits.share.name` | – | your name in the gallery, also the folder name |
| `lolcommits.share.repo` | `false` | `true` shows repo names in shared pictures |
| `lolcommits.share.retention` | `0` (keep) | delete your shared pictures after N days |
| `lolcommits.share.auto` | `false` | upload every photo without preview |

Pictures land in `~/.lolcommits/<repo>/<timestamp>-<sha>.jpg`, along with an `index.tsv` (time, repo, branch, sha, subject, file).
Failures are logged to `~/.lolcommits/lolcommits.log`.

### Advanced share targets (command line only)

- **S3 or any other rclone remote:** `share.target rclone:<remote>:<bucket>/<path>`. Pick an EU region.
- **A server over ssh:** `share.target user@host:/srv/lolcommits` (rsync).

## Development

```bash
cd app
npm install
npm run dev          # run the app; LOLCOMMITS_TEST_PATTERN=1 uses test bars instead of the camera
cd src-tauri && cargo test
npm run build        # lolcommits.app + dmg in src-tauri/target/release/bundle
```

**Releases:** pushing a tag like `0.3.0` builds signed and notarized macOS and Linux packages ([release.yml](.github/workflows/release.yml)).
- **Signing:** needs the secrets `APPLE_CERTIFICATE` (Developer ID Application, .p12 as base64), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` (app-specific password) and `APPLE_TEAM_ID`.
- **Without them:** the build is unsigned.
- **Version:** keep `version` in `app/src-tauri/tauri.conf.json` in line with the tag.

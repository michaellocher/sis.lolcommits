# lolcommits (2027 edition)

A webcam selfie for every git commit, with the commit message on it in big meme letters.
Remake of the old [lolcommits](https://github.com/lolcommits/lolcommits) Ruby gem as a single bash script.

- **macOS and Linux** – camera via ffmpeg (`avfoundation` / `v4l2`)
- **Does not block commits** – the photo is taken in the background, `git commit` returns right away
- **Leaves your hooks alone** – uses git's config-based hooks (git 2.54+); older git falls back to appending one line to `.git/hooks/post-commit`
- **Skips rebases and cherry-picks** – no burst of 40 selfies during `git rebase -i`
- **Extras:** HTML gallery, timelapse video, year-in-review contact sheet (`wrapped`)

## Install

**With Claude Code:** tell it

> Install lolcommits following https://github.com/simon-scherzinger/sis.lolcommits/blob/main/INSTALL.md

[INSTALL.md](INSTALL.md) has the steps for coding agents: install dependencies, clone, put it on the PATH, check, enable.
The agent asks before installing packages or enabling the hook, and leaves the camera test to you.

**By hand:**

Dependencies: `git`, `ffmpeg`, and, if your ffmpeg has no `drawtext` filter (Homebrew's doesn't), `imagemagick`.

```bash
brew install ffmpeg imagemagick            # macOS
sudo apt install ffmpeg fonts-dejavu-core  # Debian/Ubuntu
```

```bash
ln -s "$PWD/lolcommits" ~/.local/bin/lolcommits
lolcommits doctor
lolcommits test        # first run on macOS: allow camera access for your terminal
```

## Use

```bash
lolcommits enable --global   # every repo (git 2.54+)
lolcommits enable            # just this repo
lolcommits disable [--global]

lolcommits last              # open the latest shot
lolcommits gallery           # searchable HTML gallery, filter by repo
lolcommits timelapse [repo]  # mp4
lolcommits wrapped [year]    # contact sheet of the year
```

Skip one commit: `LOLCOMMITS_SKIP=1 git commit …`
Pause in one repo: `git config lolcommits.enabled false`

## Config

Everything is plain git config, so it works globally or per repo:

| key | default | |
|---|---|---|
| `lolcommits.device` | `0` (macOS) / `/dev/video0` (Linux) | see `lolcommits devices` |
| `lolcommits.delay` | `1.5` | seconds of camera warm-up before the shot |
| `lolcommits.size` | `1280x720` | capture resolution |
| `lolcommits.font` | Impact, else DejaVu Sans Bold | path to a `.ttf` |
| `lolcommits.dir` | `~/.lolcommits` | where pictures go |
| `lolcommits.enabled` | `true` | `false` pauses without removing the hook |

Pictures land in `~/.lolcommits/<repo>/<timestamp>-<sha>.jpg`, plus an `index.tsv` (time, repo, branch, sha, subject, file).
Failed captures are logged to `~/.lolcommits/lolcommits.log`.

## Notes

- **macOS camera permission** belongs to the app that runs git (Terminal, iTerm, Ghostty, your IDE, a GUI git client). Each one asks once.
- `LOLCOMMITS_INPUT="-f lavfi -i testsrc2=size=1280x720"` replaces the camera with a test pattern, useful for testing without a camera.
- `LOLCOMMITS_NO_OPEN=1` stops commands from opening files.

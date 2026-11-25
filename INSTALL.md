# Installing lolcommits with a coding agent

These steps are written for Claude Code or another coding agent installing lolcommits on the user's machine.
People can follow them too.

To start, tell Claude Code:

> Install lolcommits following https://github.com/simon-scherzinger/sis.lolcommits/blob/main/INSTALL.md

---

## Instructions for the agent

Do the steps in order. Ask the user before anything marked **(ask)**. Stop and report if a step fails.

### 1. Detect the platform

```bash
uname -s                      # Darwin = macOS, Linux = Linux
git --version                 # 2.54+ enables global, config-based hooks
command -v ffmpeg magick convert brew apt-get dnf pacman
```

### 2. Install dependencies (ask)

Tell the user which packages you will install, then install only what is missing:

| platform | command |
|---|---|
| macOS (Homebrew) | `brew install ffmpeg imagemagick` |
| Debian/Ubuntu | `sudo apt-get install -y ffmpeg fonts-dejavu-core` |
| Fedora | `sudo dnf install -y ffmpeg dejavu-sans-fonts` |
| Arch | `sudo pacman -S --needed ffmpeg ttf-dejavu` |

ImageMagick is only needed when ffmpeg has no `drawtext` filter, which is the case for Homebrew's ffmpeg.
Step 4 tells you whether it is needed.
Leave `sudo` password prompts to the user.

### 3. Get the code

Clone the repo into the user's usual code folder. Use `~/Developer` if you don't know where that is:

```bash
git clone https://github.com/simon-scherzinger/sis.lolcommits.git ~/Developer/sis.lolcommits
```

If the folder already exists, run `git -C ~/Developer/sis.lolcommits pull --ff-only` instead.

Put it on the PATH with a symlink, not a copy, so `git pull` updates it:

```bash
mkdir -p ~/.local/bin
ln -sf ~/Developer/sis.lolcommits/lolcommits ~/.local/bin/lolcommits
```

If `~/.local/bin` is not on `PATH`, tell the user which line to add to their shell rc file (`~/.zshrc` or `~/.bashrc`).
Do not edit that file without asking.

### 4. Check the setup

```bash
lolcommits doctor
```

The output must end with `all good ✓`.
If it reports `ffmpeg has no drawtext and ImageMagick is missing`, install ImageMagick: `brew install imagemagick` or `sudo apt-get install -y imagemagick`.

Then test rendering without the camera:

```bash
LOLCOMMITS_NO_OPEN=1 LOLCOMMITS_DIR="$(mktemp -d)" \
  LOLCOMMITS_INPUT="-f lavfi -i testsrc2=size=1280x720:rate=30" lolcommits test
```

This prints the path of a JPEG with a test pattern and a caption.

### 5. Camera test (the user runs this)

This step takes a photo of the user. Don't run it yourself. Ask the user to run it in their own terminal:

```bash
lolcommits test
```

On macOS, the app that runs git (Terminal, iTerm, Ghostty, an IDE) asks once for camera access.
If no picture appears, run `lolcommits devices` and set the right camera with `git config --global lolcommits.device <index or name>`.

### 6. Enable (ask)

Ask whether lolcommits should run in every repo or only some:

```bash
lolcommits enable --global   # every repo, needs git 2.54+
lolcommits enable            # current repo only, works with any git version
```

`enable` does not replace existing hooks.
With git 2.54+ it adds a `hook.lolcommits` entry to git config.
With older git it appends one marked line to `.git/hooks/post-commit`.

### 7. Report back

Tell the user:
- where the code lives and that `lolcommits` is on the PATH
- whether it is enabled globally or per repo
- useful commands: `lolcommits last`, `gallery`, `timelapse`, `wrapped`, `disable [--global]`
- how to skip one commit: `LOLCOMMITS_SKIP=1 git commit …`

## Uninstall

```bash
lolcommits disable --global          # and `lolcommits disable` in any repo enabled one by one
rm ~/.local/bin/lolcommits
rm -rf ~/Developer/sis.lolcommits    # pictures stay in ~/.lolcommits
```

---
name: biliup
description: Use the biliup command-line tool to start the WebUI server, run recording tasks from config files, inspect help, and operate upload/download/comment commands (upload, comments, reply, top-reply).
---

# biliup

Use this skill when the user wants to install or operate the `biliup` command-line tool.

`biliup` can start the WebUI server, run recording tasks from config files, log in, upload videos, append videos, inspect video information, download videos, list uploaded videos, list comments, reply to comments, and pin or unpin comments.

Always inspect `biliup <command> --help` (or `-h`) before generating a command. Prefer the local customized binary when this repository is the workspace.

## Install flow

When the user needs to install `biliup`, choose one installation path based on the user's operating system and preference.

### GitHub Releases prebuilt package

Use this path when the user wants to install a prebuilt binary from GitHub Releases.

For Linux or macOS, generate and run this script. Set `INSTALL_DIR` to the user's target directory when needed.

```bash
set -euo pipefail

REPO="biliup/biliup"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux) os="linux" ;;
  Darwin) os="macos" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) arch="x86_64" ;;
  aarch64|arm64) arch="aarch64" ;;
  armv7l|armv7*) arch="arm" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

asset="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | python3 -c 'import json,sys; data=json.load(sys.stdin); os=sys.argv[1]; arch=sys.argv[2]; assets=[a["name"] for a in data["assets"]]; matches=[n for n in assets if n.endswith(".tar.xz") and f"-{arch}-{os}.tar.xz" in n]; print(matches[0] if matches else "")' "$os" "$arch")"

if [ -z "$asset" ]; then
  echo "No matching biliup release asset for ${arch}-${os}" >&2
  exit 1
fi

url="https://github.com/${REPO}/releases/latest/download/${asset}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

curl -fL "$url" -o "$tmp/$asset"
tar -xJf "$tmp/$asset" -C "$tmp"
mkdir -p "$INSTALL_DIR"
install -m 755 "$(find "$tmp" -type f -name biliup | head -n 1)" "$INSTALL_DIR/biliup"

"$INSTALL_DIR/biliup" --help
```

For Windows PowerShell, generate and run this script. Set `$InstallDir` to the user's target directory when needed.

```powershell
$ErrorActionPreference = "Stop"

$Repo = "biliup/biliup"
$InstallDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:USERPROFILE "bin" }
$Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { throw "Unsupported Windows architecture" }

$Release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$Asset = $Release.assets | Where-Object { $_.name -like "biliupR-v*-$Arch-windows.zip" } | Select-Object -First 1
if (-not $Asset) { throw "No matching biliup release asset for $Arch-windows" }

$Tmp = New-Item -ItemType Directory -Force (Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString()))
try {
  $Zip = Join-Path $Tmp.FullName $Asset.name
  Invoke-WebRequest $Asset.browser_download_url -OutFile $Zip
  Expand-Archive $Zip -DestinationPath $Tmp.FullName -Force
  New-Item -ItemType Directory -Force $InstallDir | Out-Null
  $Exe = Get-ChildItem $Tmp.FullName -Recurse -Filter biliup.exe | Select-Object -First 1
  if (-not $Exe) { throw "biliup.exe not found in the release archive" }
  Copy-Item $Exe.FullName (Join-Path $InstallDir "biliup.exe") -Force
  & (Join-Path $InstallDir "biliup.exe") --help
} finally {
  Remove-Item $Tmp.FullName -Recurse -Force
}
```

If the target directory is not on `PATH`, tell the user to run `biliup` by its full path or add the target directory to `PATH`.

### Windows winget

```bash
winget install biliup
biliup --help
```

### Linux or macOS uv

```bash
uv tool install biliup
biliup --help
```

### Local customized source

This repository (fork `dannywsh/biliup`) adds Web v3 `--post-upload-goods` and comment commands. Build and install it when the user wants those features:

```bash
cargo build --release -p biliup-cli --bin biliup
install -m 755 target/release/biliup "$HOME/.local/bin/biliup"
```

If the workspace root is the parent of this repo, `cd` into `biliup` first. Verify:

```bash
biliup --help
biliup upload --help
biliup top-reply --help
```

`biliup --help` must list `top-reply`. `biliup upload --help` must list `--post-upload-goods`.

## Operation flow

### Web v3 视频带货投稿

For “商业推广 → 视频带货 → 投稿后再添加商品”, use `--submit web --post-upload-goods`. The flag generates the same random `adorder_id` range as the creator-center page and submits `adorder_type: 2`:

```bash
biliup upload \
  --submit web \
  --post-upload-goods \
  --cover /absolute/path/cover.jpg \
  --title "视频标题" \
  --tid 65 \
  --tag "标签1,标签2" \
  /absolute/path/video.mp4
```

Use this flag only with `--submit web`; APP and Bcut submission reject it. During review, `biliup show` may report `adorder_id: 0` and `has_porder: 0` even when the declaration was saved. For definitive verification, inspect the creator-center edit-detail response and require a nonzero `archive.adorder_id` plus `archive.new_adorder_info.adorder_type == 2`.

### 评论、回复、置顶

Cookie file defaults to `cookies.json` in the current working directory. Override with `-u/--user-cookie`.

List comments. Pinned comments print first as `top rpid=...`:

```bash
biliup comments BV1xxx
biliup comments BV1xxx --sort 2 --pn 1 --ps 20
```

Reply. Default is dry-run; add `--execute` to send. `rpid=0` posts a top-level comment; any other `rpid` replies under that comment:

```bash
biliup reply --help
biliup reply BV1xxx 0 "评论内容"
biliup reply BV1xxx 0 "评论内容" --execute
```

Pin or unpin a comment. Default is dry-run pin; add `--execute` to send, `--unpin` to cancel:

```bash
biliup top-reply --help
biliup top-reply BV1xxx <rpid>
biliup top-reply BV1xxx <rpid> --execute
biliup top-reply BV1xxx <rpid> --unpin --execute
```

After a successful pin, `biliup comments <vid>` should show `top rpid=<rpid>`. Newly posted comments may not appear immediately because of audit or cache.

### WebUI server

```bash
biliup server --auth
biliup server --bind 0.0.0.0 --port 19159 --auth
biliup server --config config.toml
```

Background on Linux or macOS:

```bash
nohup biliup server --auth &
```

## Help flow

When the user asks what commands are available:

```bash
biliup --help
```

When the user asks about a specific command:

```bash
biliup <command> --help
```

`-h` is equivalent to `--help`.

## Commands

```text
login
renew
upload
append
show
comments
reply
top-reply
dump-flv
download
server
list
```

When generating a command, include the concrete paths required by that command, such as cookie files, config files, and video file paths.

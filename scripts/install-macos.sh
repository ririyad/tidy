#!/usr/bin/env bash
# Install Tidy for macOS from the latest GitHub Release.
# Uses curl so the download is NOT quarantined by Gatekeeper
# (browser downloads of unsigned apps show “damaged and can’t be opened”).
set -euo pipefail

REPO="${TIDY_REPO:-ririyad/tidy}"
TAG="${TIDY_TAG:-}"
APP_NAME="Tidy"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This installer is for macOS only." >&2
  exit 1
fi

arch="$(uname -m)"
case "$arch" in
  arm64) pattern='Tidy_.*_aarch64\.dmg$' ;;
  x86_64) pattern='Tidy_.*_x64\.dmg$' ;;
  *)
    echo "Unsupported architecture: $arch" >&2
    exit 1
    ;;
esac

if [[ -n "$TAG" ]]; then
  api="https://api.github.com/repos/${REPO}/releases/tags/${TAG}"
else
  api="https://api.github.com/repos/${REPO}/releases/latest"
fi

echo "Fetching release metadata from ${api}…"
json="$(curl -fsSL "$api")"

dmg_url="$(
  python3 - "$json" "$pattern" <<'PY'
import json, re, sys
data = json.loads(sys.argv[1])
pattern = re.compile(sys.argv[2])
for asset in data.get("assets", []):
    name = asset.get("name") or ""
    if pattern.search(name):
        print(asset["browser_download_url"])
        raise SystemExit(0)
raise SystemExit(f"No matching DMG asset for pattern {sys.argv[2]!r}")
PY
)"

tmp="$(mktemp -d "${TMPDIR:-/tmp}/tidy-install.XXXXXX")"
cleanup() {
  if [[ -n "${mountpoint:-}" ]]; then
    hdiutil detach "$mountpoint" -quiet >/dev/null 2>&1 || true
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT

dmg_path="${tmp}/Tidy.dmg"
echo "Downloading ${dmg_url}…"
curl -fsSL -o "$dmg_path" "$dmg_url"

echo "Mounting disk image…"
attach_out="$(hdiutil attach "$dmg_path" -nobrowse -readonly)"
mountpoint="$(echo "$attach_out" | sed -n 's|.*\(/Volumes/.*\)$|\1|p' | tail -1)"
if [[ -z "$mountpoint" || ! -d "$mountpoint" ]]; then
  echo "Failed to mount DMG." >&2
  exit 1
fi

src_app="${mountpoint}/${APP_NAME}.app"
if [[ ! -d "$src_app" ]]; then
  echo "Expected ${APP_NAME}.app inside the DMG." >&2
  exit 1
fi

dest="/Applications/${APP_NAME}.app"
echo "Installing to ${dest}…"
rm -rf "$dest"
ditto "$src_app" "$dest"

# Clear any quarantine that ditto might preserve from the volume metadata.
xattr -cr "$dest" 2>/dev/null || true

echo "Launching ${APP_NAME}…"
open "$dest"
echo "Done."

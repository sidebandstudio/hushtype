#!/bin/bash
# Packages the built app for the website: zips hushtype.app into
# website/download/, writes its SHA-256 and size into website/install.sh, and
# copies THIRD_PARTY_NOTICES to website/notices.txt.
#
# Build the app first:
#   DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
#     CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo tauri build
# Then deploy the website/ directory (see website/README.md).
set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
app="$repo_dir/src-tauri/target/release/bundle/macos/hushtype.app"
version="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$repo_dir/src-tauri/tauri.conf.json")"
installer="$repo_dir/website/install.sh"
zip_name="hushtype-$version-mac.zip"

[ -d "$app" ] || { echo "Build the app first: $app is missing." >&2; exit 1; }
codesign --verify --deep --strict "$app"
app_version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist")"
[ "$app_version" = "$version" ] || { echo "Built app is $app_version, expected $version. Rebuild it." >&2; exit 1; }

mkdir -p "$repo_dir/website/download"
rm -f "$repo_dir/website/download/"*.zip
ditto -c -k --sequesterRsrc --keepParent "$app" "$repo_dir/website/download/$zip_name"

sha="$(shasum -a 256 "$repo_dir/website/download/$zip_name" | cut -d' ' -f1)"
bytes="$(stat -f %z "$repo_dir/website/download/$zip_name")"
size="$(( (bytes + 524288) / 1048576 )) MB"

python3 - "$installer" "$version" "$zip_name" "$sha" "$size" <<'PY'
import re, sys
path, version, zip_name, sha, size = sys.argv[1:]
text = open(path).read()
text = re.sub(r'^VERSION=".*"$', f'VERSION="{version}"', text, flags=re.M)
text = re.sub(r'^APP_URL="(.*/download/).*"$', lambda m: f'APP_URL="{m.group(1)}{zip_name}"', text, flags=re.M)
text = re.sub(r'^APP_SHA256=".*"$', f'APP_SHA256="{sha}"', text, flags=re.M)
text = re.sub(r'^APP_SIZE=".*"$', f'APP_SIZE="{size}"', text, flags=re.M)
open(path, "w").write(text)
PY

cp "$repo_dir/THIRD_PARTY_NOTICES" "$repo_dir/website/notices.txt"
printf 'Packaged %s (%s, sha256 %s)\n' "$zip_name" "$size" "$sha"

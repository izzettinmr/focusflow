#!/usr/bin/env bash
# User-local toolchain. Does not modify system packages or shell profiles.
set -euo pipefail
tools_dir="${FOCUSFLOW_ANDROID_TOOLS:-$HOME/.local/share/focusflow-android-tools}"
mkdir -p "$tools_dir/downloads" "$tools_dir/sdk/cmdline-tools"
if [ ! -x "$tools_dir/jdk/bin/java" ]; then
  curl --fail --location --retry 3 --silent --show-error \
    'https://api.adoptium.net/v3/assets/latest/21/hotspot?architecture=x64&image_type=jdk&os=linux&vendor=eclipse' \
    -o "$tools_dir/downloads/jdk.json"
  python3 - "$tools_dir" <<'PY'
import json, sys, subprocess, hashlib, tarfile
from pathlib import Path
p=Path(sys.argv[1]); package=json.loads((p/'downloads/jdk.json').read_text())[0]['binary']['package']
dest=p/'downloads/jdk.tar.gz'
subprocess.run(['curl','--fail','--location','--retry','3','--silent','--show-error',package['link'],'-o',str(dest)],check=True)
if hashlib.sha256(dest.read_bytes()).hexdigest()!=package['checksum']: raise SystemExit('Java SHA-256 mismatch')
with tarfile.open(dest) as archive:
    name=archive.getmembers()[0].name.split('/')[0]
    archive.extractall(p, filter='data')
(p/name).rename(p/'jdk')
PY
fi
export JAVA_HOME="$tools_dir/jdk"
export PATH="$JAVA_HOME/bin:$PATH"
export ANDROID_HOME="$tools_dir/sdk"
if [ ! -x "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" ]; then
  curl --fail --location --retry 3 --silent --show-error \
    https://dl.google.com/android/repository/commandlinetools-linux-15859902_latest.zip \
    -o "$tools_dir/downloads/commandline.zip"
  python3 - "$tools_dir" <<'PY'
import hashlib, zipfile, sys
from pathlib import Path
p=Path(sys.argv[1]); f=p/'downloads/commandline.zip'
if hashlib.sha256(f.read_bytes()).hexdigest()!='4e4c464f145a7512b57d088ac6c278c03c9eea610886b35a5e0804e74eedf583':
    raise SystemExit('Android commandline SHA-256 mismatch')
with zipfile.ZipFile(f) as z: z.extractall(p/'sdk/cmdline-tools')
(p/'sdk/cmdline-tools/cmdline-tools').rename(p/'sdk/cmdline-tools/latest')
PY
  chmod +x "$ANDROID_HOME/cmdline-tools/latest/bin/"*
fi
# SDK Manager displays the upstream licenses before accepting them.
yes | "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" --licenses > "$tools_dir/licenses.log" 2>&1 || test "${PIPESTATUS[1]}" = 0
"$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" 'platform-tools' 'platforms;android-36' 'build-tools;36.0.0' 'ndk;29.0.14206865'
source "$HOME/.cargo/env"
rustup target add aarch64-linux-android x86_64-linux-android
echo "Android toolchain ready: $tools_dir"

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ ! -f src-tauri/target/release/focusflow ]]; then
  echo 'Önce npm run tauri -- build --no-bundle komutunu çalıştırın.' >&2
  exit 1
fi
mkdir -p "$HOME/.local/bin"
install_temp="$(mktemp "$HOME/.local/bin/.focusflow-install.XXXXXX")"
trap 'if [[ -f "$install_temp" ]]; then unlink "$install_temp"; fi' EXIT
install -m755 src-tauri/target/release/focusflow "$install_temp"
mv -f "$install_temp" "$HOME/.local/bin/focusflow"
trap - EXIT
install -Dm644 src-tauri/icons/128x128.png "$HOME/.local/share/icons/hicolor/128x128/apps/focusflow.png"
mkdir -p "$HOME/.local/share/applications"
cat > "$HOME/.local/share/applications/focusflow.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=FocusFlow
Comment=Pomodoro, kronometre ve çalışma takibi
Exec="$HOME/.local/bin/focusflow"
Icon=focusflow
Terminal=false
Categories=Office;Education;
StartupNotify=true
DESKTOP
if command -v update-desktop-database >/dev/null; then
  update-desktop-database "$HOME/.local/share/applications"
fi
echo 'FocusFlow uygulama menüsüne kuruldu.'

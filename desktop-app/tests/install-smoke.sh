#!/usr/bin/env bash
set -euo pipefail
target="$1"
if [[ "$(uname -s)" == "Linux" ]]; then
  package_file=$(find "src-tauri/target/$target/release/bundle/deb" -maxdepth 1 -name '*.deb' -print -quit)
  test -n "$package_file"
  package_name=$(dpkg-deb -f "$package_file" Package)
  sudo dpkg -i "$package_file"
  trap 'sudo dpkg -r "$package_name"' EXIT
  binary=$(dpkg -L "$package_name" | awk -F/ '$2=="usr" && $3=="bin" && NF==4 {print;exit}')
  test -n "$binary"
  node tests/worker-smoke.cjs "$binary"
  echo 'PASS: DEB installation and installed worker startup.'
else
  disk_image=$(find "src-tauri/target/$target/release/bundle/dmg" -maxdepth 1 -name '*.dmg' -print -quit)
  test -n "$disk_image"
  mount_dir="$RUNNER_TEMP/the-box-dmg-check"
  mkdir -p "$mount_dir"
  hdiutil attach "$disk_image" -mountpoint "$mount_dir" -nobrowse -readonly
  trap 'hdiutil detach "$mount_dir"' EXIT
  bundle=$(find "$mount_dir" -maxdepth 1 -name '*.app' -print -quit)
  test -n "$bundle"
  executable=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$bundle/Contents/Info.plist")
  node tests/worker-smoke.cjs "$bundle/Contents/MacOS/$executable"
  echo 'PASS: DMG mounting and bundled worker startup.'
fi

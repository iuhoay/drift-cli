#!/usr/bin/env bash
# Install the latest drift CLI release into $DRIFT_BIN_DIR (default: ~/.local/bin).
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/iuhoay/drift-cli/main/scripts/install.sh | bash
set -euo pipefail

REPO="iuhoay/drift-cli"
INSTALL_DIR="${DRIFT_BIN_DIR:-$HOME/.local/bin}"

os=$(uname -s)
arch=$(uname -m)

case "${os}-${arch}" in
  Darwin-arm64) asset="drift-aarch64-apple-darwin" ;;
  Linux-x86_64) asset="drift-x86_64-unknown-linux-gnu" ;;
  *)
    echo "no published binary for ${os} ${arch}" >&2
    echo "see https://github.com/${REPO}/releases" >&2
    exit 1
    ;;
esac

latest_tag() {
  if [ -n "${DRIFT_VERSION:-}" ]; then
    echo "$DRIFT_VERSION"
    return
  fi

  # Do not follow the redirect — the Location header is the tag.
  local location
  location=$(curl -fsSI "https://github.com/${REPO}/releases/latest" \
    | awk 'tolower($1) == "location:" { print $2 }' \
    | tr -d '\r\n')
  if [ -n "$location" ]; then
    echo "${location##*/}"
    return
  fi

  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -1
}

version=$(latest_tag)
if [ -z "$version" ]; then
  echo "failed to determine latest version" >&2
  exit 1
fi

url="https://github.com/${REPO}/releases/download/${version}/${asset}"
echo "installing drift ${version} (${asset})"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

curl -fsSL "$url" -o "${tmpdir}/drift"
chmod +x "${tmpdir}/drift"

mkdir -p "$INSTALL_DIR"
install -m 0755 "${tmpdir}/drift" "${INSTALL_DIR}/drift"

echo "installed ${INSTALL_DIR}/drift"
"${INSTALL_DIR}/drift" --version

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
  echo
  echo "Add ${INSTALL_DIR} to your PATH:"
  shell_name=$(basename "${SHELL:-bash}")
  case "$shell_name" in
    zsh)  echo "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.zshrc && source ~/.zshrc" ;;
    fish) echo "  fish_add_path ${INSTALL_DIR}" ;;
    *)    echo "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc && source ~/.bashrc" ;;
  esac
fi

echo
echo "Run 'drift auth login' to sign in."

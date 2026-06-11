#!/bin/sh
set -e

# Repository info
OWNER="zhang3say"
REPO="cxc"

# Detect OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
  darwin)  OS="darwin" ;;
  linux)   OS="linux" ;;
  *)       echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Detect Architecture
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  ARCH="amd64" ;;
  arm64|aarch64) ARCH="arm64" ;;
  *)       echo "Unsupported Architecture: $ARCH"; exit 1 ;;
esac

# Fetch latest release tag from GitHub API
echo "Fetching latest release for $OWNER/$REPO..."
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/$OWNER/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_RELEASE" ]; then
  # Fallback to scraping HTML if API rate-limited
  LATEST_RELEASE=$(curl -s "https://github.com/$OWNER/$REPO/releases/latest" | grep -o 'tag/[^"]*' | head -n 1 | cut -d/ -f2)
fi

if [ -z "$LATEST_RELEASE" ]; then
  echo "Error: Could not retrieve latest release version."
  exit 1
fi

echo "Latest release: $LATEST_RELEASE"

# Construct download URL
TARBALL="${REPO}_${OS}_${ARCH}.tar.gz"
URL="https://github.com/$OWNER/$REPO/releases/download/$LATEST_RELEASE/$TARBALL"

# Temporary directory
TMP_DIR=$(mktemp -d)
clean_up() {
  rm -rf "$TMP_DIR"
}
trap clean_up EXIT

echo "Downloading $URL..."
curl -sSL -o "$TMP_DIR/$TARBALL" "$URL"

# Extract
echo "Extracting..."
tar -xzf "$TMP_DIR/$TARBALL" -C "$TMP_DIR"

# Determine install directory
if [ -w "/usr/local/bin" ]; then
  INSTALL_DIR="/usr/local/bin"
  SUDO=""
else
  INSTALL_DIR="$HOME/.local/bin"
  SUDO=""
  # Create if it doesn't exist
  mkdir -p "$INSTALL_DIR"
fi

# If neither is writable, ask for sudo to write to /usr/local/bin
if [ ! -w "$INSTALL_DIR" ]; then
  INSTALL_DIR="/usr/local/bin"
  SUDO="sudo"
fi

echo "Installing to $INSTALL_DIR/cxc..."
$SUDO cp "$TMP_DIR/cxc" "$INSTALL_DIR/cxc"
$SUDO chmod +x "$INSTALL_DIR/cxc"

echo "✓ cxc installed successfully to $INSTALL_DIR/cxc"
if [ "$INSTALL_DIR" = "$HOME/.local/bin" ]; then
  echo "Please make sure $HOME/.local/bin is in your PATH."
fi

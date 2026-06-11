#!/bin/sh
set -e

# Repository info
OWNER="zhang3say"
REPO="cxc"

# Detect OS and Architecture to form target triple
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  darwin)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-apple-darwin" ;;
      arm64)   TARGET="aarch64-apple-darwin" ;;
      *)       echo "Unsupported Architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  linux)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
      arm64|aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
      *)       echo "Unsupported Architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS"; exit 1
    ;;
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

# Check if cxc is already installed and compare version
if command -v cxc >/dev/null 2>&1; then
  CURRENT_VERSION=$(cxc --version | awk '{print $2}')
  CLEAN_LATEST=$(echo "$LATEST_RELEASE" | sed 's/^v//')
  if [ "$CURRENT_VERSION" = "$CLEAN_LATEST" ]; then
    echo "✓ cxc is already up to date (version $LATEST_RELEASE)."
    exit 0
  fi
fi

# Construct download URL
TARBALL="${REPO}-${TARGET}.tar.xz"
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
tar -xf "$TMP_DIR/$TARBALL" -C "$TMP_DIR"

# Find binary recursively inside temporary directory
BINARY_PATH=$(find "$TMP_DIR" -type f -name "cxc" | head -n 1)
if [ -z "$BINARY_PATH" ]; then
  echo "Error: cxc binary not found in the extracted archive."
  exit 1
fi

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
$SUDO cp "$BINARY_PATH" "$INSTALL_DIR/cxc"
$SUDO chmod +x "$INSTALL_DIR/cxc"

echo "✓ cxc installed successfully to $INSTALL_DIR/cxc"
if [ "$INSTALL_DIR" = "$HOME/.local/bin" ]; then
  echo "Please make sure $HOME/.local/bin is in your PATH."
fi

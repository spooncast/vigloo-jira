#!/bin/bash
set -e

REPO="spooncast/vigloo-jira"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="vj"

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
    Darwin)
        case "${ARCH}" in
            arm64) ASSET="vj-darwin-arm64" ;;
            x86_64) ASSET="vj-darwin-x86_64" ;;
            *) echo "Unsupported architecture: ${ARCH}"; exit 1 ;;
        esac
        ;;
    Linux)
        case "${ARCH}" in
            x86_64) ASSET="vj-linux-x86_64" ;;
            *) echo "Unsupported architecture: ${ARCH}"; exit 1 ;;
        esac
        ;;
    *)
        echo "Unsupported OS: ${OS}"
        exit 1
        ;;
esac

# Get latest release URL
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"

echo "Downloading ${ASSET}..."
curl -fsSL "${DOWNLOAD_URL}" -o "/tmp/${BINARY_NAME}"
chmod +x "/tmp/${BINARY_NAME}"

echo "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
sudo mv "/tmp/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"

echo "Done! Run 'vj' to start."

#!/bin/sh
set -e

REPO="jonaspauleta/laramux"
BINARY_NAME="laramux"
INSTALL_DIR="/usr/local/bin"

main() {
    OS=$(uname -s)
    ARCH=$(uname -m)

    case "$OS" in
        Linux)  os="linux" ;;
        Darwin) os="macos" ;;
        *)      echo "Error: Unsupported OS: $OS"; exit 1 ;;
    esac

    case "$ARCH" in
        x86_64|amd64)       arch="x86_64" ;;
        aarch64|arm64)       arch="aarch64" ;;
        *)                   echo "Error: Unsupported architecture: $ARCH"; exit 1 ;;
    esac

    ASSET="${BINARY_NAME}-${os}-${arch}"
    BASE_URL="https://github.com/${REPO}/releases/latest/download"

    TMPDIR=$(mktemp -d)
    trap 'rm -rf "$TMPDIR"' EXIT

    echo "Downloading ${ASSET}..."
    curl -fSL "${BASE_URL}/${ASSET}" -o "${TMPDIR}/${BINARY_NAME}"
    curl -fSL "${BASE_URL}/checksums.txt" -o "${TMPDIR}/checksums.txt"

    echo "Verifying checksum..."
    EXPECTED=$(grep "${ASSET}$" "${TMPDIR}/checksums.txt" | awk '{print $1}')
    if [ -z "$EXPECTED" ]; then
        echo "Error: Checksum not found for ${ASSET}"
        exit 1
    fi

    if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL=$(sha256sum "${TMPDIR}/${BINARY_NAME}" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        ACTUAL=$(shasum -a 256 "${TMPDIR}/${BINARY_NAME}" | awk '{print $1}')
    else
        echo "Warning: No sha256sum or shasum found, skipping verification"
        ACTUAL="$EXPECTED"
    fi

    if [ "$EXPECTED" != "$ACTUAL" ]; then
        echo "Error: Checksum verification failed"
        echo "  Expected: ${EXPECTED}"
        echo "  Actual:   ${ACTUAL}"
        exit 1
    fi
    echo "Checksum verified."

    chmod +x "${TMPDIR}/${BINARY_NAME}"

    if [ -w "$INSTALL_DIR" ]; then
        mv "${TMPDIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    else
        echo "Installing to ${INSTALL_DIR} (requires sudo)..."
        sudo mv "${TMPDIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    fi

    VERSION=$("${INSTALL_DIR}/${BINARY_NAME}" --version 2>/dev/null || echo "${BINARY_NAME}")
    echo "Successfully installed ${VERSION} to ${INSTALL_DIR}/${BINARY_NAME}"
}

main

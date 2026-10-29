#!/bin/sh
# Installs the Polaris CLI on Linux and macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/Davissss2/Polaris-Sync/main/install.sh | sh
#
# Environment variables:
#   POLARIS_VERSION      release tag to install (default: latest)
#   POLARIS_INSTALL_DIR  install directory (default: ~/.local/bin)
set -eu

REPO="Davissss2/Polaris-Sync"
VERSION="${POLARIS_VERSION:-latest}"
INSTALL_DIR="${POLARIS_INSTALL_DIR:-$HOME/.local/bin}"

step() { printf '\033[34m==>\033[0m %s\n' "$1"; }
fail() { printf '\033[31merror:\033[0m %s\n' "$1" >&2; exit 1; }

detect_target() {
	os=$(uname -s)
	case "$os" in
		Linux) os="linux" ;;
		Darwin) os="macos" ;;
		*) fail "unsupported operating system: $os" ;;
	esac

	arch=$(uname -m)
	case "$arch" in
		x86_64 | amd64) arch="x86_64" ;;
		arm64 | aarch64) arch="aarch64" ;;
		*) fail "unsupported architecture: $arch (only x86_64 and aarch64 are published)" ;;
	esac

	TARGET="$os-$arch"
}

resolve_tag() {
	if [ "$VERSION" != "latest" ]; then
		TAG="$VERSION"
		return
	fi

	# Follow the redirect of /releases/latest to get the tag without parsing JSON
	url=$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest" 2>/dev/null || true)

	case "$url" in
		*/tag/*) TAG="${url##*/tag/}" ;;
		*) fail "could not resolve the latest release. Set POLARIS_VERSION explicitly." ;;
	esac
}

download() {
	url="https://github.com/$REPO/releases/download/$TAG/polaris-$TAG-$TARGET.zip"
	step "Downloading $TAG ($TARGET)"
	curl -fsSL "$url" -o "$TMP/polaris.zip" || fail "download failed: $url"
}

extract() {
	step "Extracting"
	if command -v unzip >/dev/null 2>&1; then
		unzip -q "$TMP/polaris.zip" -d "$TMP"
	elif command -v python3 >/dev/null 2>&1; then
		python3 -m zipfile -e "$TMP/polaris.zip" "$TMP"
	else
		fail "neither unzip nor python3 is available to extract the archive"
	fi

	[ -f "$TMP/polaris" ] || fail "the downloaded archive does not contain a polaris binary"
}

install_binary() {
	step "Installing to $INSTALL_DIR"
	mkdir -p "$INSTALL_DIR"
	chmod +x "$TMP/polaris"
	mv -f "$TMP/polaris" "$INSTALL_DIR/polaris"
}

report_path() {
	case ":${PATH:-}:" in
		*":$INSTALL_DIR:"*) ;;
		*)
			echo ""
			step "Add this line to your shell profile so 'polaris' is on your PATH:"
			printf '\n    export PATH="%s:$PATH"\n\n' "$INSTALL_DIR"
			;;
	esac
}

main() {
	command -v curl >/dev/null 2>&1 || fail "curl is required"
	detect_target
	resolve_tag

	TMP=$(mktemp -d)
	trap 'rm -rf "$TMP"' EXIT INT TERM

	download
	extract
	install_binary
	report_path

	echo ""
	"$INSTALL_DIR/polaris" --version
	echo ""
	step "Done. Try 'polaris init my-game'"
}

main

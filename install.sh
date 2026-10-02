#!/bin/sh
# Install maddy, a Markdown-and-mathematics to PDF compiler.
#
#   curl -LsSf https://raw.githubusercontent.com/jeppejohansen/maddy/main/install.sh | sh
#
# Environment:
#   MADDY_VERSION      a tag to install, such as v0.1.0 (default: the latest)
#   MADDY_INSTALL_DIR  where to put the binary (default: ~/.local/bin)
#
# The binary is self-contained: nothing else is installed, and no Typst or LaTeX
# toolchain is required.

set -eu

REPO="jeppejohansen/maddy"
INSTALL_DIR="${MADDY_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*"; }
err() { printf 'error: %s\n' "$*" >&2; exit 1; }

need() {
    command -v "$1" >/dev/null 2>&1 || err "this installer needs \`$1\`"
}

# --- how to download -------------------------------------------------------

if command -v curl >/dev/null 2>&1; then
    fetch() { curl -LsSf --proto '=https' --tlsv1.2 "$1" -o "$2"; }
    fetch_stdout() { curl -LsSf --proto '=https' --tlsv1.2 "$1"; }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -qO "$2" "$1"; }
    fetch_stdout() { wget -qO- "$1"; }
else
    err "this installer needs \`curl\` or \`wget\`"
fi

need tar
need mktemp

# --- which build -----------------------------------------------------------

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
    Darwin)
        # One universal binary covers Intel and Apple Silicon.
        slug="macos-universal"
        fallback=""
        ;;
    Linux)
        case "$arch" in
            x86_64 | amd64)  slug="x86_64-linux";  fallback="x86_64-linux-static" ;;
            aarch64 | arm64) slug="aarch64-linux"; fallback="aarch64-linux-static" ;;
            *) err "unsupported architecture \`$arch\`; build from source with \`cargo install --git https://github.com/$REPO\`" ;;
        esac
        ;;
    *)
        err "unsupported system \`$os\`; on Windows use install.ps1, or build from source"
        ;;
esac

# --- which version ---------------------------------------------------------

version="${MADDY_VERSION:-}"
if [ -z "$version" ]; then
    say "Looking up the latest release..."
    version="$(fetch_stdout "https://api.github.com/repos/$REPO/releases/latest" \
        | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
        | head -n 1)"
    [ -n "$version" ] || err "could not determine the latest release; set MADDY_VERSION to install a specific one"
fi
number="${version#v}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

# Download, verify and unpack one build. Returns non-zero if it will not run.
try_install() {
    name="maddy-${number}-$1"
    archive="${name}.tar.gz"
    base="https://github.com/$REPO/releases/download/$version"

    say "Downloading $archive..."
    fetch "$base/$archive" "$tmp/$archive" || return 1

    # The release publishes one SHA256SUMS covering every archive.
    if fetch "$base/SHA256SUMS" "$tmp/SHA256SUMS" 2>/dev/null; then
        expected="$(grep " $archive\$" "$tmp/SHA256SUMS" 2>/dev/null | cut -d' ' -f1 || true)"
        if [ -n "$expected" ]; then
            if command -v sha256sum >/dev/null 2>&1; then
                actual="$(sha256sum "$tmp/$archive" | cut -d' ' -f1)"
            elif command -v shasum >/dev/null 2>&1; then
                actual="$(shasum -a 256 "$tmp/$archive" | cut -d' ' -f1)"
            else
                actual=""
            fi
            if [ -n "$actual" ] && [ "$actual" != "$expected" ]; then
                err "checksum mismatch for $archive; refusing to install"
            fi
            [ -n "$actual" ] && say "Checksum verified."
        fi
    fi

    rm -rf "$tmp/unpacked" && mkdir -p "$tmp/unpacked"
    tar xzf "$tmp/$archive" -C "$tmp/unpacked" || return 1
    [ -f "$tmp/unpacked/maddy" ] || return 1
    chmod +x "$tmp/unpacked/maddy"

    # A build can download fine and still not run — a glibc older than the one
    # it was built against, most commonly — so it is checked before installing.
    "$tmp/unpacked/maddy" --version >/dev/null 2>&1 || return 1
}

if ! try_install "$slug"; then
    if [ -n "$fallback" ]; then
        say "That build does not run here; trying the statically linked one..."
        try_install "$fallback" || err "no build for this system would run; please open an issue"
    else
        err "the download failed or the binary would not run; please open an issue"
    fi
fi

# --- install ---------------------------------------------------------------

mkdir -p "$INSTALL_DIR"
mv -f "$tmp/unpacked/maddy" "$INSTALL_DIR/maddy"

say ""
say "Installed $("$INSTALL_DIR/maddy" --version) to $INSTALL_DIR/maddy"

case ":${PATH}:" in
    *":$INSTALL_DIR:"*) say "Run \`maddy paper.md\` to compile a document." ;;
    *)
        say ""
        say "$INSTALL_DIR is not on your PATH. Add it with:"
        say ""
        say "    export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
esac

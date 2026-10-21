#!/usr/bin/env bash
# Builds a Debian .deb package for StatsCraft.
# Run on Linux (locally or in CI). Produces StatsCraft_<version>_<arch>.deb
#
# The script has to work even when it is checked out with Windows line
# endings: bash reads the carriage return as part of the command, and
# "set -euo pipefail" then fails with "invalid option name". The
# .gitattributes keeps the file LF. If a stray carriage return does
# get in, the script runs from a clean copy of itself - it cannot
# rewrite itself in place, because bash reads a script incrementally
# and would then read the rewritten bytes out of step.
#
# The guard is one line, and it must stay one line: a multi-line `if`
# block here would be parsed before it runs, and the parse would fail
# on the very carriage return the guard is meant to remove.
grep -q "$(printf '\r')" "$0" 2>/dev/null && exec bash -c 'tr -d "\r" < "$1" > "$2" && exec bash "$2" "${@:3}"' _ "$0" "$(mktemp)" "$0" "$@"
set -euo pipefail

# The original script's path, which the line-ending guard passes in as
# the first argument. Without it the clean copy would be looked for in
# the temporary directory.
ORIG="${1:-$0}"
cd "$(dirname "$ORIG")/../.." # repo root

# The version comes from Cargo.toml.
#
# It used to come from `cargo pkgid | cut -d# -f2 | cut -d: -f2`, which
# is wrong. `cargo pkgid` prints "path+file:///...#name@version" - but
# it omits the "name@" part when the package name matches the last
# directory of the path, and includes it when they differ. The CI
# checks the repository out into a directory named after the GitHub
# repository, so a crate whose name differs from the repository's
# produces "name@version", and dpkg rejects it:
#
#   dpkg-deb: error: 'Version' field value 'statecraft@0.1.0':
#   version number does not start with digit
#
# Reading the version out of Cargo.toml does not depend on the crate
# name and the directory name agreeing.
VERSION=$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' Cargo.toml | head -1)
if [ -z "$VERSION" ]; then
  echo "could not read the version from Cargo.toml" >&2
  exit 1
fi
# A Debian version may only contain alphanumerics, a dot, a plus, a
# hyphen and a tilde, so anything else is folded into a dot.
DEB_VERSION=$(printf '%s' "$VERSION" | sed 's/[^A-Za-z0-9.+~-]/-/g')
ARCH=$(dpkg --print-architecture)
PKGNAME="statscraft"
DEB_ROOT="target/deb/${PKGNAME}_${ARCH}"
INSTALL_PREFIX="/usr"

echo "== version: $VERSION  arch: $ARCH =="

# 1. Release build
cargo build --release

# 2. Assemble filesystem tree
rm -rf "$DEB_ROOT"
mkdir -p \
  "$DEB_ROOT/DEBIAN" \
  "$DEB_ROOT$INSTALL_PREFIX/bin" \
  "$DEB_ROOT$INSTALL_PREFIX/share/applications" \
  "$DEB_ROOT$INSTALL_PREFIX/share/icons/hicolor/scalable/apps" \
  "$DEB_ROOT$INSTALL_PREFIX/share/doc/$PKGNAME"

cp target/release/statscraft "$DEB_ROOT$INSTALL_PREFIX/bin/statscraft"
cp packaging/assets/statscraft.desktop "$DEB_ROOT$INSTALL_PREFIX/share/applications/"
cp packaging/assets/icon.svg "$DEB_ROOT$INSTALL_PREFIX/share/icons/hicolor/scalable/apps/statscraft.svg"
cp README.md "$DEB_ROOT$INSTALL_PREFIX/share/doc/$PKGNAME/"

# The control file: the package's metadata, with the placeholders
# filled in.
cp packaging/linux/control "$DEB_ROOT/DEBIAN/control"
sed -i "s/@VERSION@/$DEB_VERSION/g" "$DEB_ROOT/DEBIAN/control"
sed -i "s/@ARCH@/$ARCH/g" "$DEB_ROOT/DEBIAN/control"

# strip binary
strip "$DEB_ROOT$INSTALL_PREFIX/bin/statscraft" || true

# 3. Build package
dpkg-deb --build "$DEB_ROOT" "statscraft_${DEB_VERSION}_${ARCH}.deb"
echo "== produced: statscraft_${DEB_VERSION}_${ARCH}.deb =="

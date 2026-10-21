#!/usr/bin/env bash
# Builds a Debian .deb package for StateCraft.
# Run on Linux (locally or in CI). Produces StateCraft_<version>_<arch>.deb
set -euo pipefail

cd "$(dirname "$0")/../.." # repo root

VERSION=$(cargo pkgid | cut -d# -f2 | cut -d: -f2)
ARCH=$(dpkg --print-architecture)
PKGNAME="statecraft"
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

cp target/release/statecraft "$DEB_ROOT$INSTALL_PREFIX/bin/statecraft"
cp packaging/assets/statecraft.desktop "$DEB_ROOT$INSTALL_PREFIX/share/applications/"
cp packaging/assets/icon.svg "$DEB_ROOT$INSTALL_PREFIX/share/icons/hicolor/scalable/apps/statecraft.svg"
cp README.md "$DEB_ROOT$INSTALL_PREFIX/share/doc/$PKGNAME/"

# Replace placeholders
sed -i "s/@VERSION@/$VERSION/g" "$DEB_ROOT/DEBIAN/control" 2>/dev/null || true
cp packaging/linux/control "$DEB_ROOT/DEBIAN/control"
sed -i "s/@VERSION@/$VERSION/g" "$DEB_ROOT/DEBIAN/control"
sed -i "s/@ARCH@/$ARCH/g" "$DEB_ROOT/DEBIAN/control"

# strip binary
strip "$DEB_ROOT$INSTALL_PREFIX/bin/statecraft" || true

# 3. Build package
dpkg-deb --build "$DEB_ROOT" "statecraft_${VERSION}_${ARCH}.deb"
echo "== produced: statecraft_${VERSION}_${ARCH}.deb =="

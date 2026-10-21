# Builds the StatsCraft .deb package the way the CI does.
# Run it with:  docker run --rm -v "$PWD:/repo" -w /repo ubuntu:24.04 bash ci_linux.sh
set -euo pipefail

cd /repo

echo "== 1. the build dependencies =="
apt-get update -qq
apt-get install -y -qq libxkbcommon-dev libwayland-dev libx11-dev \
  libxcursor-dev libxrandr-dev libxi-dev libgl1-mesa-dev fontconfig \
  build-essential pkg-config >/dev/null
echo "   ok"

echo "== 2. the release build =="
cargo build --release
echo "   ok"

echo "== 3. the .deb =="
bash packaging/linux/build_deb.sh
echo "   ok"

echo "== 4. the package =="
ls -la statscraft_*.deb
dpkg-deb -I statscraft_*.deb
dpkg-deb -c statscraft_*.deb

echo "== 5. the tests =="
cargo test --release
echo "== ALL GREEN =="

#!/usr/bin/env bash
# Reproducible release packaging: tar.gz + dmg + SHA256SUMS for GitHub Releases.
# Usage: ./scripts/package_release.sh [version]
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="${1:-0.1.0}"
TARGET="sfai-${VERSION}-aarch64-apple-darwin"

cargo build --release
rm -rf "dist/${TARGET}" dist/dmg-root
mkdir -p "dist/${TARGET}/docs"
cp target/release/sfai "README.md" "LICENSE" "dist/${TARGET}/"
cp docs/providers.md docs/benchmark-spec.md "dist/${TARGET}/docs/" 2>/dev/null || true
cat > "dist/${TARGET}/INSTALL.md" <<'MD'
# sfai — 安装（macOS Apple Silicon）
sudo cp sfai /usr/local/bin/    # 或加入任意 PATH 目录
sfai                             # 交互会话（Decide-Act 运行时）
MD

cd dist
tar czf "${TARGET}.tar.gz" "${TARGET}"
mkdir -p dmg-root && cp -R "${TARGET}"/* dmg-root/
ln -sf /usr/local/bin dmg-root/Install-to-PATH 2>/dev/null || true
hdiutil create -volname "sfai ${VERSION}" -srcfolder dmg-root -ov -format UDZO "${TARGET}.dmg" >/dev/null
shasum -a 256 "${TARGET}.tar.gz" "${TARGET}.dmg" | tee SHA256SUMS
echo "artifacts ready in dist/ — publish with:"
echo "  gh release create v${VERSION} dist/${TARGET}.dmg dist/${TARGET}.tar.gz dist/SHA256SUMS"

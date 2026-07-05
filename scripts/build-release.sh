#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────
#  Linux 发布构建脚本
#  编译 x86_64-unknown-linux-gnu 目标，输出到 dist/ 目录
# ─────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$PROJECT_ROOT/dist"

# 从 Cargo.toml 读取包名和版本（不依赖 jq，用 grep+awk）
PACKAGE_NAME=$(grep -m1 '^name = ' "$PROJECT_ROOT/Cargo.toml" | sed 's/name = "\(.*\)"/\1/')
VERSION=$(grep -m1 '^version = ' "$PROJECT_ROOT/Cargo.toml" | sed 's/version = "\(.*\)"/\1/')

# 清空旧的发布目录
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

echo "══════════════════════════════════════════════════════════════════════════"
echo "  $PACKAGE_NAME v$VERSION  Linux Release Build"
echo "══════════════════════════════════════════════════════════════════════════"
echo ""

# ─────────────────────────────────────────────────────────────
#  编译 x86_64-unknown-linux-gnu
# ─────────────────────────────────────────────────────────────
TARGET="x86_64-unknown-linux-gnu"
echo "▶ 正在编译 $TARGET ..."

cargo build --release --target "$TARGET"

# 复制并重命名产物
SRC="$PROJECT_ROOT/target/$TARGET/release/$PACKAGE_NAME"
DST="$DIST_DIR/${PACKAGE_NAME}_v${VERSION}_${TARGET}"
cp "$SRC" "$DST"
echo "  ✔ $DST"

# ─────────────────────────────────────────────────────────────
#  输出汇总
# ─────────────────────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════════════════════════════════"
echo "  发布文件已输出到: $DIST_DIR"
echo "══════════════════════════════════════════════════════════════════════════"
echo "  $(basename "$DST")"
echo "══════════════════════════════════════════════════════════════════════════"
echo ""
echo "  💡 macOS 用户请运行: cargo build --release --target x86_64-apple-darwin"
echo "  💡 若需静态链接（musl），请运行: cargo build --release --target x86_64-unknown-linux-musl"
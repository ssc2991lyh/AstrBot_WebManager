#!/usr/bin/env bash
# 本地打包 Release zip —— 供 GitHub Release 上传，供 install_debian.sh 下载部署。
#
# 前置：
#   1) 已编译后端：      cargo build --release   （产物 src-tauri/target/release/astrbot-launcher）
#   2) 已构建前端：      pnpm build              （产物 dist/）
#
# 用法（在仓库根目录执行）：
#   bash deploy/package_release.sh              # 文件名带日期版本
#   bash deploy/package_release.sh v0.1.0       # 指定版本号
#
# 生成的 zip 结构（与 install_debian.sh 对齐）：
#   astrbot-launcher
#   astrbot-launcher.service
#   dist/
set -euo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"

VER="${1:-$(date +%Y%m%d)}"
OUT="astrbot-web-manager-linux-x64.zip"

# 校验产物存在
if [[ ! -x src-tauri/target/release/astrbot-launcher ]]; then
  echo "错误：未找到后端二进制，请先 'cd src-tauri && cargo build --release'" >&2
  exit 1
fi
if [[ ! -f dist/index.html ]]; then
  echo "错误：未找到前端产物，请先 'pnpm build'" >&2
  exit 1
fi

rm -f "$OUT"
mkdir -p _pkg
cp src-tauri/target/release/astrbot-launcher _pkg/
cp deploy/astrbot-launcher.service _pkg/
cp -r dist _pkg/dist
( cd _pkg && zip -r "../$OUT" . )
rm -rf _pkg

echo "==> 已生成 $OUT"
echo "    请作为 GitHub Release 资产上传，并在 install_debian.sh 中更新 RELEASE_URL 为："
echo "    https://github.com/ssc2991lyh/AstrBot_WebManager/releases/download/$VER/$OUT"

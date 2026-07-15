#!/usr/bin/env bash
# AstrBot Launcher HTTP 后端 —— VMware 虚拟机内一键部署脚本
# 用法（在 VM 内，以可 sudo 的用户执行）：
#   sudo -u mulq bash -c 'cd /path/to/AstrBot_WebManager && bash deploy/setup_vm.sh'
# 脚本假设源码已 scp 到 VM，且当前位于仓库根目录或其子目录。
set -euo pipefail

SRC_DIR="$(cd "$(dirname "$0")/.." && pwd)"   # 仓库根 = AstrBot_WebManager
INSTALL_DIR=/opt/astrbot-launcher
UNIT=astrbot-launcher.service
SERVICE_USER=mulq

echo "==> 仓库根: $SRC_DIR"

# 1. 系统编译依赖（native-tls/openssl、zlib 等）
if command -v apt-get >/dev/null 2>&1; then
  sudo apt-get update -y
  sudo apt-get install -y build-essential pkg-config libssl-dev
fi

# 2. 装 Rust 工具链（若未装）
if ! command -v cargo >/dev/null 2>&1; then
  echo "==> 安装 rustup ..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi
export PATH="$HOME/.cargo/bin:$PATH"

# 3. 编译 release
echo "==> cargo build --release (src-tauri) ..."
cd "$SRC_DIR/src-tauri"
cargo build --release

# 4. 安装二进制
echo "==> 安装到 $INSTALL_DIR ..."
sudo mkdir -p "$INSTALL_DIR/bin"
sudo cp "target/release/astrbot_launcher" "$INSTALL_DIR/bin/"
sudo chown -R "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR"

# 5. 注册 systemd 单元
echo "==> 安装 systemd 单元 $UNIT ..."
sudo cp "$SRC_DIR/deploy/$UNIT" /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now "$UNIT"

echo "==> 完成：astrbot-launcher 已启动，监听 :6190"
sudo systemctl status "$UNIT" --no-pager || true

#!/usr/bin/env bash
# AstrBot Web Manager —— Debian / Ubuntu 一键安装脚本（预编译 Release 模式）
#
# 适用：Debian 系发行版（Debian / Ubuntu 及衍生版），依赖 systemd + apt。
# 原理：从 GitHub Release 下载【预编译好的 zip】（含后端二进制 + 前端 dist + systemd 单元），
#       解压后安装到系统路径，并以 systemd 托管。无需在目标机编译 Rust。
#
# zip 内部结构预期（由 deploy/package_release.sh 生成）：
#   astrbot-launcher            # 后端二进制（release build）
#   astrbot-launcher.service    # systemd 单元
#   dist/                       # 前端静态产物（index.html, assets/ ...）
#
# 用法：
#   sudo bash install_debian.sh
#   # 或显式指定 Release 包地址：
#   RELEASE_URL=https://github.com/ssc2991lyh/AstrBot_WebManager/releases/download/v0.1.0/astrbot-web-manager-linux-x64.zip \
#     sudo -E bash install_debian.sh
#
set -euo pipefail

# ---------------- 可配置项 ----------------
RELEASE_URL="${RELEASE_URL:-https://github.com/ssc2991lyh/AstrBot_WebManager/releases/download/v0.1.0/astrbot-web-manager-linux-x64.zip}"
BIN_NAME=astrbot-launcher
BIN_PATH=/usr/local/bin/$BIN_NAME
WWW_ROOT=/var/www/astrbot-web
DATA_HOME=/var/lib/astrbot-launcher
SERVICE_USER=astrbot
UNIT=astrbot-launcher.service

TMP="$(mktemp -d)"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

# ---------------- 0. 必须为 root ----------------
if [[ $EUID -ne 0 ]]; then
  echo "错误：请以 root 运行（sudo bash $0）" >&2
  exit 1
fi

# ---------------- 1. 仅支持 Debian 系 ----------------
if [[ ! -f /etc/os-release ]] || ! grep -qiE '^(ID=(debian|ubuntu)|ID_LIKE=.*debian)' /etc/os-release; then
  echo "错误：本脚本仅支持 Debian 系发行版（Debian / Ubuntu 及衍生），当前系统不受支持。" >&2
  exit 1
fi

echo "==> [1/6] 安装系统依赖（unzip, curl, nginx）..."
apt-get update -y
apt-get install -y unzip curl nginx

# ---------------- 2. 下载 Release 包 ----------------
echo "==> [2/6] 下载 Release 包：$RELEASE_URL"
ZIP="$TMP/astrbot-web-manager.zip"
if command -v wget >/dev/null 2>&1; then
  wget -qO "$ZIP" "$RELEASE_URL"
else
  curl -fsSL "$RELEASE_URL" -o "$ZIP"
fi

# ---------------- 3. 解压并定位包目录 ----------------
echo "==> [3/6] 解压..."
cd "$TMP"
unzip -oq "$ZIP"
# 兼容 zip 内含或不含顶层目录：自动找到后端二进制所在目录
PKG_DIR="$(find "$TMP" -maxdepth 3 -type f -name "$BIN_NAME" | head -1 | xargs dirname)"
if [[ -z "$PKG_DIR" || ! -x "$PKG_DIR/$BIN_NAME" ]]; then
  echo "错误：zip 内未找到可执行文件 $BIN_NAME，请检查 Release 包结构。" >&2
  exit 1
fi
echo "    包目录：$PKG_DIR"

# ---------------- 4. 安装后端二进制 ----------------
echo "==> [4/6] 安装后端二进制到 $BIN_PATH"
install -m 0755 "$PKG_DIR/$BIN_NAME" "$BIN_PATH"

# ---------------- 5. 服务用户 + systemd 单元 + 前端 ----------------
echo "==> [5/6] 创建服务用户并安装 systemd 单元 / 前端"
id "$SERVICE_USER" >/dev/null 2>&1 || useradd -r -s /usr/sbin/nologin "$SERVICE_USER"
mkdir -p "$DATA_HOME" "$WWW_ROOT/dist"
cp -r "$PKG_DIR/dist/." "$WWW_ROOT/dist/"
# 确保单元里的 User/Group 与本脚本一致（zip 内单元可能写的是其他用户）
sed -e "s/^User=.*/User=$SERVICE_USER/" -e "s/^Group=.*/Group=$SERVICE_USER/" \
  "$PKG_DIR/$UNIT" > "/etc/systemd/system/$UNIT"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_HOME" "$WWW_ROOT"
systemctl daemon-reload
systemctl enable --now "$UNIT"

# ---------------- 6. nginx 反代（若尚未配置） ----------------
echo "==> [6/6] 配置 nginx 反代（若尚未配置）"
SITE=/etc/nginx/sites-available/astrbot-web
if [[ ! -f "$SITE" ]]; then
  cat > "$SITE" <<NGINX
server {
    listen 80;
    server_name _;
    root $WWW_ROOT/dist;
    index index.html;
    location / {
        try_files \$uri \$uri/ /index.html;
    }
    location /api/ {
        proxy_pass http://127.0.0.1:6190;
        proxy_set_header Host \$host;
        proxy_read_timeout 3600s;   # SSE 日志流需要较长超时
    }
}
NGINX
  ln -sf "$SITE" /etc/nginx/sites-enabled/astrbot-web
  # 关闭默认站点避免冲突（可选）
  rm -f /etc/nginx/sites-enabled/default
  nginx -t && systemctl reload nginx
fi

echo
echo "==> 完成！后端以 systemd 服务 $UNIT 运行，前端由 nginx 托管于 $WWW_ROOT/dist"
echo "    打开 http://<本机IP>/ 即可访问管理面板（默认端口 80）。"
systemctl status "$UNIT" --no-pager || true

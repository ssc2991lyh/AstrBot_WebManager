![Logo](https://github.com/user-attachments/assets/d28490ba-17f7-4c44-a27c-a9bcb22dd036)

# AstrBot Web Manager

> 把 [AstrBot Launcher](https://github.com/AstrBotDevs/AstrBot-Launcher) 的 React 前端从 Tauri 桌面壳里**解放出来**，做成一套能在浏览器里跑、部署在无图形界面的服务器上的实例管理器。后端用 Rust + [axum](https://github.com/tokio-rs/axum) 暴露 HTTP API（默认 `:6190`），所有命令、日志、文件操作都走 HTTP，不再依赖桌面环境。

**License:** [AGPL-3.0](./LICENSE) · 源码见 [GitHub](https://github.com/ssc2991lyh/AstrBot_WebManager)。按 AGPL §13，如果你把本服务架到公网给别人用，需要向用户提供对应源码。

---

## 它是什么 / 不是什么

- **是**：一个 headless（无界面、无窗口、无托盘）的 AstrBot 多实例管理面板。在一台 Linux 服务器上起一个后端进程，浏览器打开就能管理实例的创建、启停、升级、备份、日志、文件。
- **不是**：官方 AstrBot Launcher 桌面客户端。官方版是 Tauri 套壳的原生应用，本仓库是它的**网页化分支**——前端代码同源，但运行时彻底去掉了 Tauri，命令全部改为 HTTP 调用。
- 本项目与另一套基于 Node.js 的 Web Manager（`:6180`）是相互独立的两条技术路线，本仓库只涵盖 Rust + axum 这一套。

## 功能特性

- **多实例管理**：创建 / 启动 / 停止 / 重启 / 删除实例，查看运行状态与端口，编辑实例名、版本、监听地址。
- **版本管理**：从 GitHub Releases 拉取 AstrBot 与 Launcher 的可用版本，一键安装 / 卸载 / 升级。
- **组件管理**：安装、重装、卸载运行时组件（如 Node.js、Python 工具链等）。
- **备份与恢复**：按实例创建备份、列出备份、恢复、删除。
- **日志**：实时日志流（通过 SSE 推送），可查看系统日志与实例输出。
- **文件管理（移植自 MSLX）**：直接浏览和编辑实例目录下的文件——列目录、读写文本、新建目录、重命名、复制、移动、删除、改权限（chmod）、压缩 / 解压、分片上传、下载。路径被严格限制在实例目录内，禁止跨目录穿越。
- **高级设置**：代理、PyPI / npm / Node.js 镜像源、GitHub 代理、UV 依赖开关、中国大陆一键加速、开机自启（systemd）、主题偏好等。
- **关于页**：版本信息、发布说明、许可证。

## 架构

```text
┌──────────────┐      HTTP /api/* + SSE       ┌──────────────────────┐
│  浏览器前端   │ ───────────────────────────▶ │  Rust + axum 后端     │
│ React+Vite   │                              │  (headless, :6190)   │
│ 静态文件托管  │ ◀─────────────────────────── │                      │
└──────────────┘      JSON 响应 / 事件流        └──────────┬───────────┘
                                                           │ 启动/停止/读写
                                                           ▼
                                                  ┌────────────────────┐
                                                  │  AstrBot 实例进程    │
                                                  │  (core/main.py 等)  │
                                                  └────────────────────┘
```

- 前端是纯静态产物（`pnpm build` 生成的 `dist/`），用任意 Web 服务器托管即可。
- 后端是一个常驻进程，把所有原 Tauri command 注册成 HTTP 路由：
  - `POST /api/<command>`：业务命令，参数以 JSON body 传递（如 `create_instance`、`start_instance`、`fetch_releases`）。
  - `GET /api/events`：SSE 事件流，推送日志与状态变化。
  - `/api/files/instance/<id>/...`：一组文件管理路由（列表、内容读写、上传分片、压缩解压等）。
- 后端默认监听 `0.0.0.0:6190`，可用环境变量 `ASTRBOT_HTTP_PORT` 覆盖。
- 数据目录默认走系统规范路径（Linux 上约 `~/.local/share/astrbot-launcher`）。

## 技术栈

- **前端**：React 19 · Vite 7 · Ant Design 6 · TypeScript · Zustand（状态）· react-router（路由）。
- **后端**：Rust 2021 · axum 0.8（HTTP）· tokio（异步）· reqwest（网络/代理）· redb（状态库）· zip / tar / flate2（归档）· walkdir（遍历）。
- **已剥离的部分**：Tauri 运行时、桌面窗口、系统托盘、macOS 专属分支、应用自更新、开机自启 Tauri 插件。这些桌面能力要么被删除，要么改为 systemd / 纯前端（localStorage）实现。

## 构建

需要：Node.js 18+、pnpm、Rust 工具链（含 `cargo`）、系统库 `libssl-dev` / `pkg-config` / `build-essential`（Linux）。

```bash
# 前端
pnpm install
pnpm build                 # 产物输出到 dist/

# 后端（在 src-tauri 下）
cd src-tauri
cargo build --release      # 产物为 target/release/astrbot_launcher
```

本地联调时，`vite.config.ts` 已把 `/api` 代理到 `localhost:6190`（可用 `ASTRBOT_BACKEND_HOST` 指向远端后端）；前端 dev server 端口 `1420`。

## 部署（Linux 服务器）

`deploy/` 目录提供三种部署辅助：

- `setup_vm.sh`：**源码编译安装**——在目标机装 Rust 工具链并 `cargo build --release`，适合需要自定义后端或参与开发。
- `install_debian.sh`：**Debian/Ubuntu 一键安装（推荐生产环境）**——从 GitHub Release 下载**预编译好的 zip**（含后端二进制 + 前端 `dist` + systemd 单元），解压即装、免编译，适合干净的 Debian 系服务器。
- `astrbot-launcher.service`：systemd 单元文件（被上面两个脚本使用）。
- `package_release.sh`：本地打包脚本，把编译产物打成 Release 用的 zip。

预编译安装典型流程（详见 `install_debian.sh` 顶部注释）：

```bash
# 在 Debian/Ubuntu 服务器上，以 root 执行：
RELEASE_URL=https://github.com/ssc2991lyh/AstrBot_WebManager/releases/download/v0.1.0/astrbot-web-manager-linux-x64.zip \
  sudo -E bash -c 'curl -fsSL https://raw.githubusercontent.com/ssc2991lyh/AstrBot_WebManager/main/deploy/install_debian.sh | bash'
```

后端以 systemd 服务常驻，单元名 `astrbot-launcher.service`，二进制位于 `/usr/local/bin/astrbot-launcher`，监听 `:6190`，崩溃后由 `Restart=on-failure` 自动拉起。「开机自启面板」「重启 / 停止 Manager」等操作也通过 systemd 完成（对应 `set_systemd_enabled` / `restart_manager` / `stop_manager` 命令）。

nginx 反代示例：

```nginx
server {
    listen 80;
    root /var/www/astrbot-web/dist;
    index index.html;

    # 单页应用回退
    location / {
        try_files $uri $uri/ /index.html;
    }

    # 业务 API 与 SSE 事件流转发到 Rust 后端
    location /api/ {
        proxy_pass http://127.0.0.1:6190;
        proxy_set_header Host $host;
        proxy_read_timeout 3600s;   # SSE 日志流需要较长超时
    }
}
```

## 关于「中国大陆一键加速」与镜像

高级设置里开启「中国大陆一键加速」后，后端会忽略手动填写的代理和源，改用以下预设地址（这些镜像由社区维护，本项目仅内置其 URL）：

- npm 镜像：<https://npmreg.proxy.ustclug.org/>
- Node.js 下载：<https://mirrors.ustc.edu.cn/node/>
- PyPI 镜像：<https://mirrors.ustc.edu.cn/pypi/>
- python-build-standalone 发布：<https://mirrors.ustc.edu.cn/github-release/astral-sh/python-build-standalone/LatestRelease/>
- uv 发布：<https://mirrors.ustc.edu.cn/github-release/astral-sh/uv/LatestRelease/>
- AstrBot Release 加速代理：<https://gh-proxy.com>

如果某镜像的维护者不希望被内置，提出 Issue 说明即可，我们会及时处理。感谢这些基础设施的维护者。

## 已知限制 / 与官方版的差异

- **文件管理**功能说明：参考了来自[MSLTeam/MSLX](https://github.com/MSLTeam/MSLX)的部分源码改造而来。
- macOS 专属代码分支已移除，本项目以 **Linux 服务器部署**为主要场景。
- 桌面专属能力（系统托盘、窗口状态记忆、应用自身更新、双击打开文件夹等）在本网页版中不存在或改为等效的 Web / systemd 实现。
- 后端目前未做鉴权中间件；若暴露到公网，请在反向代理层自行加认证 / 访问控制。

## 附注

如果本项目对你的部署有帮助，欢迎到仓库点个 Star ❤️。问题、建议或镜像移除请求，请走 GitHub Issue。

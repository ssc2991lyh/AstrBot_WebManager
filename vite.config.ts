import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import fs from "node:fs";

// 后端 HTTP 服务地址（Rust axum 后端，端口 6190）。
// 默认连本机；在 VMware 虚拟机里跑后端时，用 ASTRBOT_BACKEND_HOST=<VM_IP> 覆盖。
const backendHost = process.env.ASTRBOT_BACKEND_HOST || "localhost";
const backendUrl = `http://${backendHost}:6190`;

// Windows 上 fs.realpathSync.native 可能把 C: 盘路径解析成 D: 盘（如容器/重定向环境），
// 导致 vite 内部路径与 config.root 不一致，触发 [vite:build-html] 报错。
// 直接把 root 设成 realpath 后的真实路径即可对齐。
const projectRoot = fs.realpathSync.native(path.resolve("./"));

// https://vite.dev/config/
export default defineConfig(async () => ({
  root: projectRoot,
  plugins: [react()],

  build: {
    rollupOptions: {
      input: "./index.html",
    },
  },

  // 纯网页端（已去 Tauri）：允许局域网 / VM 访问 dev server
  server: {
    port: 1420,
    strictPort: true,
    host: true,
    // 阶段 3 联调：把 /api/* 与 /api/events(SSE) 转发到 Rust 后端
    proxy: {
      "/api": {
        target: backendUrl,
        changeOrigin: true,
      },
    },
    watch: {
      // 不监听 Rust 后端源码
      ignored: ["**/src-tauri/**"],
    },
  },
}));

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// 后端 HTTP 服务地址（Rust axum 后端，端口 6190）。
// 默认连本机；在 VMware 虚拟机里跑后端时，用 ASTRBOT_BACKEND_HOST=<VM_IP> 覆盖。
const backendHost = process.env.ASTRBOT_BACKEND_HOST || "localhost";
const backendUrl = `http://${backendHost}:6190`;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

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

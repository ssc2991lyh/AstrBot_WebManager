import type { GitHubRelease, AppSnapshot, ThemePreference, RepairPreserveScope } from './types';
import { mockHandler } from './mock';

export interface FileItem {
  name: string;
  is_dir: boolean;
  size: number;
  modified?: number;
}

// 阶段1：真后端(Rust HTTP 层)未接时走 mock；阶段3 接真后端后置 false。
export const USE_MOCK = false;

// 前端保持官方 Launcher 的 camelCase 参数风格（1:1 复刻 Tauri invoke 契约），
// 后端 axum api_handler 统一按 snake_case 读取，故在传输层做顶层 key 转换。
// 注意：只转换「顶层」key；嵌套对象（如 release / themePreference 的值）的字段
// 属于协议内部约定，不应改动。
function snakeify(obj: Record<string, unknown> | undefined): Record<string, unknown> {
  if (!obj) return {};
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(obj)) {
    const snake = key.replace(/[A-Z]/g, (m) => '_' + m.toLowerCase());
    out[snake] = obj[key];
  }
  return out;
}

// 文件管理走独立 axum 路由（二进制/流式），不进 /api/{cmd} JSON dispatch。
async function fget<T = any>(url: string): Promise<T> {
  const res = await fetch(url);
  if (!res.ok) throw new Error((await res.text()) || `HTTP ${res.status}`);
  return (await res.json()) as T;
}
async function fpost<T = any>(url: string, body?: unknown): Promise<T> {
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error((await res.text()) || `HTTP ${res.status}`);
  return (await res.json()) as T;
}

// 统一传输层：Tauri invoke -> HTTP fetch('/api/<cmd>')
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (USE_MOCK) {
    return mockHandler<T>(cmd, args) as T;
  }
  const res = await fetch('/api/' + cmd, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(snakeify(args ?? {})),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}`);
  }
  const ct = res.headers.get('content-type') || '';
  if (!ct.includes('application/json')) return undefined as unknown as T;
  return (await res.json()) as T;
}

type LockCheckRequest =
  | {
      target: 'instance_data' | 'backup_create' | 'instance_upgrade';
      instanceId: string;
    }
  | {
      target: 'backup_restore';
      backupPath: string;
    };

export const api = {
  // ========================================
  // Snapshot
  // ========================================
  getAppSnapshot: () => call<AppSnapshot>('get_app_snapshot'),
  rebuildAppSnapshot: () => call<AppSnapshot>('rebuild_app_snapshot'),
  getVersion: () => call<string>('get_version'),

  // ========================================
  // Config
  // ========================================
  saveGithubProxy: (githubProxy: string) => call<void>('save_github_proxy', { githubProxy }),
  saveProxy: (settings: {
    proxyUrl: string;
    proxyPort: string;
    proxyUsername: string;
    proxyPassword: string;
  }) => call<void>('save_proxy', settings),
  savePypiMirror: (pypiMirror: string) => call<void>('save_pypi_mirror', { pypiMirror }),
  saveNodejsMirror: (nodejsMirror: string) => call<void>('save_nodejs_mirror', { nodejsMirror }),
  saveNpmRegistry: (npmRegistry: string) => call<void>('save_npm_registry', { npmRegistry }),
  saveUseUvForDeps: (useUvForDeps: boolean) =>
    call<void>('save_use_uv_for_deps', { useUvForDeps }),
  compareVersions: (a: string, b: string) => call<number>('compare_versions', { a, b }),
  saveCheckInstanceUpdate: (checkInstanceUpdate: boolean) =>
    call<void>('save_check_instance_update', { checkInstanceUpdate }),
  savePersistInstanceState: (persistInstanceState: boolean) =>
    call<void>('save_persist_instance_state', { persistInstanceState }),
  saveIgnoreExternalPath: (ignoreExternalPath: boolean) =>
    call<void>('save_ignore_external_path', { ignoreExternalPath }),
  saveMainlandAcceleration: (mainlandAcceleration: boolean) =>
    call<void>('save_mainland_acceleration', { mainlandAcceleration }),
  saveLockCheckExtensionWhitelist: (lockCheckExtensionWhitelist: boolean) =>
    call<void>('save_lock_check_extension_whitelist', { lockCheckExtensionWhitelist }),
  saveThemePreference: (themePreference: ThemePreference) =>
    call<void>('save_theme_preference', { themePreference }),

  // ========================================
  // Systemd (#3 开机自启，替代 Tauri autostart 插件)
  // ========================================
  getSystemdStatus: () =>
    call<{ installed: boolean; enabled: boolean }>('get_systemd_status'),
  setSystemdEnabled: (enable: boolean) => call<void>('set_systemd_enabled', { enable }),

  // ========================================
  // Manager 服务控制 (#6 停止/重启，替代 Tauri process 插件)
  // ========================================
  stopManager: () => call<void>('stop_manager'),
  restartManager: () => call<void>('restart_manager'),

  // ========================================
  // Components
  // ========================================
  installComponent: (componentId: string) => call<string>('install_component', { componentId }),
  reinstallComponent: (componentId: string) =>
    call<string>('reinstall_component', { componentId }),
  uninstallComponent: (componentId: string) =>
    call<string>('uninstall_component', { componentId }),

  // ========================================
  // GitHub
  // ========================================
  fetchReleases: (forceRefresh: boolean = false) =>
    call<GitHubRelease[]>('fetch_releases', { forceRefresh }),
  fetchLauncherReleaseNotes: (version: string) =>
    call<string | null>('fetch_launcher_release_notes', { version }),

  // ========================================
  // Version Management
  // ========================================
  installVersion: (release: GitHubRelease) => call<void>('install_version', { release }),
  uninstallVersion: (version: string) => call<void>('uninstall_version', { version }),

  // ========================================
  // Troubleshooting
  // ========================================
  clearInstanceData: (instanceId: string) => call<void>('clear_instance_data', { instanceId }),
  checkLock: (request: LockCheckRequest) =>
    call<void>('check_lock', {
      target: request.target,
      instanceId: 'instanceId' in request ? request.instanceId : null,
      backupPath: 'backupPath' in request ? request.backupPath : null,
    }),
  clearInstanceVenv: (instanceId: string) => call<void>('clear_instance_venv', { instanceId }),
  clearPycache: (instanceId: string) => call<void>('clear_pycache', { instanceId }),
  repairInstance: (instanceId: string, preserveScope: RepairPreserveScope) =>
    call<void>('repair_instance', { instanceId, preserveScope }),
  rebuildInstanceManifest: () =>
    call<{ instances: number; versions: number }>('rebuild_instance_manifest'),

  // ========================================
  // Instance Management
  // ========================================
  openInstanceCoreFolder: (instanceId: string) =>
    call<void>('open_instance_core_folder', { instanceId }),
  createInstance: (name: string, version: string, port: number = 0) =>
    call<void>('create_instance', { name, version, port }),
  deleteInstance: (instanceId: string) => call<void>('delete_instance', { instanceId }),
  updateInstance: (
    instanceId: string,
    name?: string,
    version?: string,
    host?: string,
    port?: number
  ) =>
    call<void>('update_instance', {
      instanceId,
      name: name ?? null,
      version: version ?? null,
      host: host ?? null,
      port: port ?? null,
    }),
  startInstance: (instanceId: string) => call<number>('start_instance', { instanceId }),
  stopInstance: (instanceId: string) => call<void>('stop_instance', { instanceId }),
  restartInstance: (instanceId: string) => call<number>('restart_instance', { instanceId }),
  getInstancePort: (instanceId: string) => call<number>('get_instance_port', { instanceId }),

  // ========================================
  // Backup
  // ========================================
  createBackup: (instanceId: string) => call<string>('create_backup', { instanceId }),
  restoreBackup: (backupPath: string) => call<void>('restore_backup', { backupPath }),
  deleteBackup: (backupPath: string) => call<void>('delete_backup', { backupPath }),

  // ========================================
  // File Manager (instance core/ directory)
  // ========================================
  files: {
    lists: (id: string, path = '') =>
      fget<{ path: string; items: FileItem[] }>(
        `/api/files/instance/${id}/lists?path=${encodeURIComponent(path)}`
      ),
    content: (id: string, path: string) =>
      fget<{ path: string; content: string }>(
        `/api/files/instance/${id}/content?path=${encodeURIComponent(path)}`
      ),
    write: (id: string, path: string, content: string) =>
      fpost(`/api/files/instance/${id}/content`, { path, content }),
    mkdir: (id: string, path: string) =>
      fpost(`/api/files/instance/${id}/directory`, { path }),
    rename: (id: string, old_path: string, new_path: string) =>
      fpost(`/api/files/instance/${id}/rename`, { old_path, new_path }),
    remove: (id: string, paths: string[]) =>
      fpost(`/api/files/instance/${id}/delete`, { paths }),
    copy: (id: string, sources: string[], dest_dir: string) =>
      fpost(`/api/files/instance/${id}/copy`, { sources, dest_dir }),
    move: (id: string, sources: string[], dest_dir: string) =>
      fpost(`/api/files/instance/${id}/move`, { sources, dest_dir }),
    chmod: (id: string, path: string, mode: number) =>
      fpost(`/api/files/instance/${id}/chmod`, { path, mode }),
    downloadUrl: (id: string, path: string) =>
      `/api/files/instance/${id}/download?path=${encodeURIComponent(path)}`,
    uploadInit: (id: string, dir: string, name: string) =>
      fpost<{ upload_id: string }>(`/api/files/instance/${id}/upload/init`, { dir, name }),
    uploadChunk: async (id: string, upload_id: string, bytes: Uint8Array) => {
      const res = await fetch(`/api/files/instance/${id}/upload/chunk/${upload_id}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/octet-stream' },
        body: bytes,
      });
      if (!res.ok) throw new Error((await res.text()) || `HTTP ${res.status}`);
      return (await res.json()) as { ok: boolean };
    },
    uploadFinish: (id: string, upload_id: string) =>
      fpost(`/api/files/instance/${id}/upload/finish/${upload_id}`),
    compress: (id: string, sources: string[], dest: string) =>
      fpost(`/api/files/instance/${id}/compress`, { sources, dest }),
    decompress: (id: string, path: string, dest_dir: string) =>
      fpost(`/api/files/instance/${id}/decompress`, { path, dest_dir }),
  },
};

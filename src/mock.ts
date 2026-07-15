import type {
  AppSnapshot,
  GitHubRelease,
  BackupInfo,
  InstanceStatus,
  InstalledVersion,
  ComponentStatus,
  AppConfig,
} from './types';

// 阶段1（前端解耦 + Mock）：本地无真后端时返回假数据，让 6 页 UI 在浏览器跑通。
// 阶段3 接真 Rust HTTP 后端后，api.ts 的 USE_MOCK 置 false 即可切真数据。

export const MOCK_APP_VERSION = '0.3.9';

const mockConfig: AppConfig = {
  mainland_acceleration: false,
  github_proxy: 'https://cdn.gh-proxy.org',
  proxy_url: '',
  proxy_port: '',
  proxy_username: '',
  proxy_password: '',
  pypi_mirror: 'https://pypi.tuna.tsinghua.edu.cn/simple',
  nodejs_mirror: 'https://npmmirror.com/mirrors/node',
  npm_registry: 'https://registry.npmmirror.com',
  use_uv_for_deps: true,
  close_to_tray: false,
  autostart_minimize_to_tray: false,
  check_instance_update: true,
  persist_instance_state: false,
  ignore_external_path: false,
  lock_check_extension_whitelist: false,
  theme_preference: 'system',
};

const mockInstances: InstanceStatus[] = [
  {
    id: 'accept1',
    name: 'accept1',
    state: 'running',
    port: 6185,
    version: 'v4.26.6',
    dashboard_enabled: true,
    pid_tracker_not_available: false,
    configured_host: '0.0.0.0',
    configured_port: 6185,
  },
  {
    id: '4c6d9a97-1a2b-4c3d-8e5f-000000000001',
    name: '我的 AstrBot',
    state: 'stopped',
    port: 2001,
    version: 'v4.26.5',
    dashboard_enabled: true,
    pid_tracker_not_available: false,
    configured_host: '0.0.0.0',
    configured_port: 2001,
  },
  {
    id: 'd72de757-2b3c-4d5e-9f0a-000000000002',
    name: 'AstrBot 02',
    state: 'stopped',
    port: 2002,
    version: 'v4.26.5',
    dashboard_enabled: true,
    pid_tracker_not_available: false,
    configured_host: '0.0.0.0',
    configured_port: 2002,
  },
];

const mockVersions: InstalledVersion[] = [
  { version: 'v4.26.6', zip_path: '/home/mulq/astrbot_launcher/versions/v4.26.6.zip' },
  { version: 'v4.26.5', zip_path: '/home/mulq/astrbot_launcher/versions/v4.26.5.zip' },
];

const mockBackups: BackupInfo[] = [
  {
    filename: 'backup-accept1-2026-07-14T20-30-00.zip',
    path: '/home/mulq/astrbot_launcher/backups/backup-accept1-2026-07-14T20-30-00.zip',
    metadata: {
      created_at: '2026-07-14T20:30:00',
      instance_name: 'accept1',
      instance_id: 'accept1',
      version: 'v4.26.6',
      arch_target: 'x86_64-unknown-linux-gnu',
    },
  },
];

const mockComponents = (): ComponentStatus[] => [
  { id: 'python', installed: true, display_name: 'Python', description: 'Python 3.10 / 3.12 运行时' },
  { id: 'nodejs', installed: true, display_name: 'Node.js (LTS)', description: 'Node.js 运行时' },
  { id: 'uv', installed: true, display_name: 'uv', description: 'uv / uvx 包管理工具' },
];

export const mockSnapshot: AppSnapshot = {
  instances: mockInstances,
  versions: mockVersions,
  backups: mockBackups,
  components: { components: mockComponents() },
  config: mockConfig,
};

export const mockReleases: GitHubRelease[] = [
  {
    tag_name: 'v4.26.6',
    name: 'v4.26.6',
    published_at: '2026-07-10T12:00:00Z',
    prerelease: false,
    assets: [
      {
        name: 'astrbot.zip',
        browser_download_url:
          'https://github.com/AstrBotDevs/AstrBot/releases/download/v4.26.6/astrbot.zip',
        size: 12345678,
      },
    ],
    html_url: 'https://github.com/AstrBotDevs/AstrBot/releases/tag/v4.26.6',
    body: '## Added\n- 一些新功能\n\n## Changed\n- 一些改动',
  },
  {
    tag_name: 'v4.26.5',
    name: 'v4.26.5',
    published_at: '2026-07-01T12:00:00Z',
    prerelease: false,
    assets: [
      {
        name: 'astrbot.zip',
        browser_download_url:
          'https://github.com/AstrBotDevs/AstrBot/releases/download/v4.26.5/astrbot.zip',
        size: 12300000,
      },
    ],
    html_url: 'https://github.com/AstrBotDevs/AstrBot/releases/tag/v4.26.5',
    body: '## Fixed\n- 修复若干问题',
  },
];

// 路由 mock：已实现命令返回真实假数据；其余（void/string/number 操作命令）返回 undefined 不崩。
export function mockHandler<T>(cmd: string, _args?: Record<string, unknown>): T | undefined {
  switch (cmd) {
    case 'get_app_snapshot':
    case 'rebuild_app_snapshot':
      return mockSnapshot as unknown as T;
    case 'fetch_releases':
      return mockReleases as unknown as T;
    case 'get_version':
      return MOCK_APP_VERSION as unknown as T;
    case 'compare_versions':
      return 0 as unknown as T;
    case 'rebuild_instance_manifest':
      return { instances: 3, versions: 2 } as unknown as T;
    case 'start_instance':
    case 'restart_instance':
    case 'get_instance_port':
      return 2001 as unknown as T;
    case 'get_systemd_status':
      return { installed: true, enabled: true } as unknown as T;
    default:
      return undefined as unknown as T;
  }
}

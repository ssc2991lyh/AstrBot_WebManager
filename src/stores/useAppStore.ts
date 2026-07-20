import { create } from 'zustand';
import { api } from '../api';
import type {
  InstalledVersion,
  InstanceStatus,
  AppConfig,
  AppSnapshot,
  BackupInfo,
  DeployProgress,
  DeployState,
  ComponentStatus,
  DownloadProgress,
  ThemePreference,
} from '../types';
import { handleApiError } from '../utils';
import { MODAL_CLOSE_DELAY_MS } from '../constants';
import { useLogStore } from './useLogStore';

interface AppState {
  // Data
  instances: InstanceStatus[];
  versions: InstalledVersion[];
  backups: BackupInfo[];
  components: ComponentStatus[];
  config: AppConfig | null;
  loading: boolean;
  initialized: boolean;

  // Operations loading map
  operations: Record<string, boolean>;

  // Deploy state
  deployState: DeployState | null;

  // Download progress
  downloadProgress: Record<string, DownloadProgress>;

  // Actions
  hydrateSnapshot: (snapshot: AppSnapshot) => void;
  reloadSnapshot: (options?: { throwOnError?: boolean }) => Promise<void>;
  rebuildSnapshotFromDisk: (options?: { throwOnError?: boolean }) => Promise<void>;
  startOperation: (key: string) => void;
  finishOperation: (key: string) => void;
  isOperationActive: (key: string) => boolean;
  startDeploy: (instanceName: string, type: 'start' | 'upgrade' | 'downgrade') => void;
  setDeployProgress: (progress: DeployProgress | null) => void;
  closeDeploy: () => void;
  clearDownloadProgress: (id: string) => void;
  setThemePreference: (themePreference: ThemePreference) => void;
}

const KNOWN_COMPONENTS: ReadonlyArray<
  Pick<ComponentStatus, 'id' | 'display_name' | 'description'>
> = [
  {
    id: 'python',
    display_name: 'Python',
    description: 'Python 3.10 / 3.12 运行时',
  },
  {
    id: 'nodejs',
    display_name: 'Node.js (LTS)',
    description: 'Node.js 运行时',
  },
  {
    id: 'uv',
    display_name: 'uv',
    description: 'uv / uvx 包管理工具',
  },
];

function isComponentStatus(value: unknown): value is ComponentStatus {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const item = value as Record<string, unknown>;
  return (
    typeof item.id === 'string' &&
    typeof item.installed === 'boolean' &&
    typeof item.display_name === 'string' &&
    typeof item.description === 'string'
  );
}

function readRawComponents(payload: unknown): ComponentStatus[] | null {
  if (Array.isArray(payload)) {
    return payload.filter(isComponentStatus);
  }

  if (!payload || typeof payload !== 'object') {
    return null;
  }

  const nested = (payload as { components?: unknown }).components;
  if (!Array.isArray(nested)) {
    return null;
  }

  return nested.filter(isComponentStatus);
}

function normalizeComponents(payload: unknown, previous: ComponentStatus[]): ComponentStatus[] {
  const parsed = readRawComponents(payload);
  const stableSource = parsed && parsed.length > 0 ? parsed : previous;

  const knownOrder = new Map(KNOWN_COMPONENTS.map((item, index) => [item.id, index]));
  const previousMap = new Map(previous.map((item) => [item.id, item]));
  const sourceMap = new Map(stableSource.map((item) => [item.id, item]));

  const mergedKnown = KNOWN_COMPONENTS.map((known) => {
    const fromSource = sourceMap.get(known.id);
    const fromPrevious = previousMap.get(known.id);
    return {
      id: known.id,
      installed: fromSource?.installed ?? fromPrevious?.installed ?? false,
      display_name: fromSource?.display_name ?? fromPrevious?.display_name ?? known.display_name,
      description: fromSource?.description ?? fromPrevious?.description ?? known.description,
    };
  });

  const extras = stableSource
    .filter((item) => !knownOrder.has(item.id))
    .sort((a, b) => a.id.localeCompare(b.id));

  return [...mergedKnown, ...extras];
}

export const useAppStore = create<AppState>((set, get) => {
  let snapshotRequestSeq = 0;
  let minValidSnapshotSeq = 0;
  let latestAppliedSnapshotSeq = 0;
  let inflightSnapshotLoads = 0;
  const operationCounters = new Map<string, number>();

  const applySnapshot = (snapshot: AppSnapshot) => {
    const rawComponents = (snapshot as { components?: unknown }).components;
    const nextComponents = normalizeComponents(rawComponents, get().components);

    set({
      instances: snapshot.instances,
      versions: snapshot.versions,
      backups: snapshot.backups,
      components: nextComponents,
      config: snapshot.config,
      initialized: true,
    });
  };

  const loadSnapshot = async (
    fetchSnapshot: () => Promise<AppSnapshot>,
    options?: { throwOnError?: boolean }
  ) => {
    const requestSeq = ++snapshotRequestSeq;
    inflightSnapshotLoads += 1;
    set({ loading: true });
    try {
      const snapshot = await fetchSnapshot();
      if (requestSeq < minValidSnapshotSeq || requestSeq < latestAppliedSnapshotSeq) {
        return;
      }

      latestAppliedSnapshotSeq = requestSeq;
      applySnapshot(snapshot);
    } catch (e: unknown) {
      if (options?.throwOnError) {
        throw e;
      }
      handleApiError(e);
    } finally {
      inflightSnapshotLoads = Math.max(0, inflightSnapshotLoads - 1);
      set({ loading: inflightSnapshotLoads > 0 });
    }
  };

  return {
    // Initial state
    instances: [],
    versions: [],
    backups: [],
    components: [],
    config: null,
    loading: false,
    initialized: false,
    operations: {},
    deployState: null,
    downloadProgress: {},

    hydrateSnapshot: (snapshot: AppSnapshot) => {
      // Event snapshots are authoritative at arrival time.
      // Ignore all in-flight request snapshots that started before this event.
      minValidSnapshotSeq = snapshotRequestSeq + 1;
      latestAppliedSnapshotSeq = minValidSnapshotSeq;
      applySnapshot(snapshot);
    },

    reloadSnapshot: async (options?: { throwOnError?: boolean }) => {
      await loadSnapshot(api.getAppSnapshot, options);
    },

    rebuildSnapshotFromDisk: async (options?: { throwOnError?: boolean }) => {
      await loadSnapshot(api.rebuildAppSnapshot, options);
    },

    startOperation: (key: string) => {
      set((state) => {
        const nextCount = (operationCounters.get(key) ?? 0) + 1;
        operationCounters.set(key, nextCount);

        if (state.operations[key]) {
          return state;
        }

        return { operations: { ...state.operations, [key]: true } };
      });
    },

    finishOperation: (key: string) => {
      set((state) => {
        const nextCount = (operationCounters.get(key) ?? 0) - 1;
        if (nextCount > 0) {
          operationCounters.set(key, nextCount);
          return state;
        }

        operationCounters.delete(key);
        if (!state.operations[key]) {
          return state;
        }

        const next = { ...state.operations };
        delete next[key];
        return { operations: next };
      });
    },

    isOperationActive: (key: string) => {
      return (operationCounters.get(key) ?? 0) > 0;
    },

    startDeploy: (instanceName: string, type: 'start' | 'upgrade' | 'downgrade') => {
      set({ deployState: { instanceName, deployType: type, progress: null } });
    },

    setDeployProgress: (progress: DeployProgress | null) => {
      set((state) => ({
        deployState: state.deployState ? { ...state.deployState, progress } : null,
      }));
    },

    closeDeploy: () => {
      set({ deployState: null });
    },

    clearDownloadProgress: (id: string) => {
      set((state) => {
        const next = { ...state.downloadProgress };
        delete next[id];
        return { downloadProgress: next };
      });
    },

    setThemePreference: (themePreference: ThemePreference) => {
      set((state) => {
        if (!state.config) {
          return state;
        }

        return {
          config: {
            ...state.config,
            theme_preference: themePreference,
          },
        };
      });
    },
  };
});

// 事件通道：阶段2 用 SSE 订阅 app-snapshot / deploy-progress / log-entry。
// 后端 /api/events 为 broadcast 流，事件名即 event 字段，data 为 JSON 载荷。
let eventSource: EventSource | null = null;

export function initEventListeners() {
  if (eventSource || typeof EventSource === 'undefined') return;
  const es = new EventSource('/api/events');
  eventSource = es;

  es.addEventListener('deploy-progress', (ev) => {
    try {
      const progress = JSON.parse((ev as MessageEvent).data) as DeployProgress;
      const store = useAppStore.getState();
      store.setDeployProgress(progress);
      if (progress.step === 'done' || progress.step === 'error') {
        window.setTimeout(() => useAppStore.getState().closeDeploy(), MODAL_CLOSE_DELAY_MS);
      }
    } catch {
      /* ignore malformed payload */
    }
  });

  es.addEventListener('app-snapshot', (ev) => {
    try {
      const snapshot = JSON.parse((ev as MessageEvent).data) as AppSnapshot;
      useAppStore.getState().hydrateSnapshot(snapshot);
    } catch {
      /* ignore malformed payload */
    }
  });

  // log-entry：运行实例 stdout / 系统日志实时尾随（镜像官方 Launcher 的 listen('log-entry')）
  es.addEventListener('log-entry', (ev) => {
    try {
      const entry = JSON.parse((ev as MessageEvent).data);
      useLogStore.getState().addLogEntry(entry);
    } catch {
      /* ignore malformed payload */
    }
  });

  es.onerror = () => {
    // EventSource 内置自动重连；此处仅防止未捕获异常冒泡。
  };
}

export function cleanupEventListeners() {
  if (eventSource) {
    eventSource.close();
    eventSource = null;
  }
}

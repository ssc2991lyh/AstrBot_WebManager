import { create } from 'zustand';

type CheckResult = 'found' | 'latest' | 'error';

interface UpdateState {
  hasUpdate: boolean;
  newVersion: string;
  releaseNotes: string;
  releaseNotesReady: boolean;
  checking: boolean;
  installing: boolean;
  pendingUpdate: unknown;
  checkForUpdate: () => Promise<CheckResult>;
  installUpdate: () => Promise<boolean>;
}

export const useUpdateStore = create<UpdateState>((set) => ({
  hasUpdate: false,
  newVersion: '',
  releaseNotes: '',
  releaseNotesReady: false,
  checking: false,
  installing: false,
  pendingUpdate: null,

  // #5 应用自身更新：阶段3 接真后端后改为查 GitHub releases（api.fetchReleases 对比当前版本）。
  // 阶段0(Mock) 直接返回 latest（无更新提示）。
  checkForUpdate: async () => {
    set({
      hasUpdate: false,
      newVersion: '',
      releaseNotes: '',
      releaseNotesReady: true,
      pendingUpdate: null,
      checking: false,
    });
    return 'latest';
  },

  // #5 阶段3 实装；阶段0 无更新可装，返回 false
  installUpdate: async () => {
    return false;
  },
}));

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

/** 24 小时内不重复静默检查（手动检查不受限） */
const SILENT_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;
const STORAGE_KEY_LAST_CHECK = 'updater:last_silent_check_at';
const STORAGE_KEY_SKIPPED_VERSION = 'updater:skipped_version';

export interface UpdateMeta {
  version: string;
  currentVersion: string;
  date?: string;
  body?: string;
}

type Phase = 'idle' | 'checking' | 'available' | 'downloading' | 'installing' | 'ready' | 'error' | 'up_to_date';

export const useUpdaterStore = defineStore('updater', () => {
  const phase = ref<Phase>('idle');
  const error = ref<string>('');
  const meta = ref<UpdateMeta | null>(null);
  const contentLength = ref<number>(0);
  const downloaded = ref<number>(0);

  /** 最近一次 check 拿到的 Update 句柄，用于后续 downloadAndInstall */
  let pendingUpdate: Update | null = null;

  const hasUpdate = computed(() => phase.value === 'available' || phase.value === 'downloading' || phase.value === 'installing' || phase.value === 'ready');
  const isBusy = computed(() => phase.value === 'checking' || phase.value === 'downloading' || phase.value === 'installing');
  const progressPercent = computed(() => {
    if (contentLength.value <= 0) return 0;
    return Math.min(100, Math.floor((downloaded.value / contentLength.value) * 100));
  });

  function resetProgress() {
    contentLength.value = 0;
    downloaded.value = 0;
  }

  /**
   * 检查更新。
   * @param silent true 表示启动时的静默检查，会遵循 24h 防抖 + 跳过版本；false 为用户手动触发，强制执行。
   * @returns 是否发现可用更新
   */
  async function checkUpdate(silent = false): Promise<boolean> {
    if (isBusy.value) return hasUpdate.value;

    if (silent) {
      const last = Number(localStorage.getItem(STORAGE_KEY_LAST_CHECK) || '0');
      if (last > 0 && Date.now() - last < SILENT_CHECK_INTERVAL_MS) {
        return false;
      }
    }

    phase.value = 'checking';
    error.value = '';

    try {
      const update = await check();
      localStorage.setItem(STORAGE_KEY_LAST_CHECK, String(Date.now()));

      if (!update) {
        pendingUpdate = null;
        meta.value = null;
        phase.value = 'up_to_date';
        return false;
      }

      pendingUpdate = update;
      meta.value = {
        version: update.version,
        currentVersion: update.currentVersion,
        date: update.date,
        body: update.body,
      };

      if (silent) {
        const skipped = localStorage.getItem(STORAGE_KEY_SKIPPED_VERSION);
        if (skipped === update.version) {
          phase.value = 'idle';
          return false;
        }
      }

      phase.value = 'available';
      return true;
    } catch (e: any) {
      pendingUpdate = null;
      meta.value = null;
      error.value = formatError(e);
      phase.value = 'error';
      if (!silent) {
        console.error('[Updater] check failed:', e);
      }
      return false;
    }
  }

  /** 下载并安装最近一次 check 到的 Update，完成后由调用方决定何时 relaunch */
  async function downloadAndInstall(): Promise<void> {
    if (!pendingUpdate) {
      throw new Error('没有待更新的版本，请先检查更新');
    }
    if (isBusy.value && phase.value !== 'available') return;

    resetProgress();
    phase.value = 'downloading';
    error.value = '';

    try {
      await pendingUpdate.downloadAndInstall((event: DownloadEvent) => {
        switch (event.event) {
          case 'Started':
            contentLength.value = event.data.contentLength ?? 0;
            downloaded.value = 0;
            break;
          case 'Progress':
            downloaded.value += event.data.chunkLength;
            break;
          case 'Finished':
            phase.value = 'installing';
            break;
        }
      });
      phase.value = 'ready';
    } catch (e: any) {
      error.value = formatError(e);
      phase.value = 'error';
      throw e;
    }
  }

  async function restartApp(): Promise<void> {
    await relaunch();
  }

  /** 跳过当前版本：静默检查时不再弹出此版本，直到更新版本出现或用户手动检查 */
  function skipCurrentVersion() {
    if (meta.value?.version) {
      localStorage.setItem(STORAGE_KEY_SKIPPED_VERSION, meta.value.version);
    }
    phase.value = 'idle';
  }

  function dismiss() {
    if (phase.value === 'up_to_date' || phase.value === 'error' || phase.value === 'available') {
      phase.value = 'idle';
    }
  }

  function formatError(e: unknown): string {
    if (e instanceof Error) return e.message;
    if (typeof e === 'string') return e;
    try {
      return JSON.stringify(e);
    } catch {
      return String(e);
    }
  }

  return {
    phase,
    error,
    meta,
    contentLength,
    downloaded,
    hasUpdate,
    isBusy,
    progressPercent,
    checkUpdate,
    downloadAndInstall,
    restartApp,
    skipCurrentVersion,
    dismiss,
  };
});

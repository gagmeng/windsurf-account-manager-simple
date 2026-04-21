<template>
  <el-dialog
    v-model="visible"
    :title="dialogTitle"
    width="520px"
    :close-on-click-modal="!updaterStore.isBusy"
    :close-on-press-escape="!updaterStore.isBusy"
    :show-close="!updaterStore.isBusy"
    align-center
    class="update-dialog"
    @closed="handleClosed"
  >
    <div class="update-body">
      <div v-if="phase === 'checking'" class="state-block">
        <el-icon class="is-loading spin-icon" :size="32"><Loading /></el-icon>
        <p>正在检查更新...</p>
      </div>

      <template v-else-if="phase === 'available'">
        <div class="version-row">
          <div class="version-col">
            <span class="version-label">当前版本</span>
            <span class="version-value current">v{{ updaterStore.meta?.currentVersion }}</span>
          </div>
          <el-icon :size="20" class="arrow-icon"><Right /></el-icon>
          <div class="version-col">
            <span class="version-label">新版本</span>
            <span class="version-value latest">v{{ updaterStore.meta?.version }}</span>
          </div>
        </div>

        <div v-if="updaterStore.meta?.date" class="release-date">
          发布时间：{{ formatDate(updaterStore.meta.date) }}
        </div>

        <div class="release-notes">
          <div class="notes-title">更新内容</div>
          <pre class="notes-body">{{ updaterStore.meta?.body?.trim() || '（此版本未提供更新说明）' }}</pre>
        </div>
      </template>

      <template v-else-if="phase === 'downloading' || phase === 'installing'">
        <div class="state-block">
          <p class="progress-title">
            {{ phase === 'installing' ? '正在安装...' : '正在下载新版本...' }}
          </p>
          <el-progress
            :percentage="updaterStore.progressPercent"
            :status="phase === 'installing' ? 'success' : ''"
            :stroke-width="16"
            class="progress-bar"
          />
          <div class="progress-detail" v-if="phase === 'downloading' && updaterStore.contentLength > 0">
            {{ formatBytes(updaterStore.downloaded) }} / {{ formatBytes(updaterStore.contentLength) }}
          </div>
        </div>
      </template>

      <template v-else-if="phase === 'ready'">
        <div class="state-block success">
          <el-icon :size="40" color="#67c23a"><CircleCheck /></el-icon>
          <p class="ready-title">新版本 v{{ updaterStore.meta?.version }} 已准备就绪</p>
          <p class="ready-desc">点击「立即重启」应用新版本，或关闭对话框稍后手动重启。</p>
        </div>
      </template>

      <template v-else-if="phase === 'up_to_date'">
        <div class="state-block success">
          <el-icon :size="40" color="#67c23a"><CircleCheck /></el-icon>
          <p>已是最新版本 v{{ currentVersion }}</p>
        </div>
      </template>

      <template v-else-if="phase === 'error'">
        <div class="state-block error">
          <el-icon :size="40" color="#f56c6c"><CircleClose /></el-icon>
          <p class="error-title">检查 / 下载更新失败</p>
          <pre class="error-detail">{{ updaterStore.error }}</pre>
          <p class="error-hint">
            可以稍后重试，或前往
            <a href="#" @click.prevent="openReleasesPage">Releases 页面</a>
            手动下载。
          </p>
        </div>
      </template>
    </div>

    <template #footer>
      <div class="footer-actions">
        <template v-if="phase === 'available'">
          <el-button @click="skipVersion">跳过此版本</el-button>
          <el-button @click="handleClose">稍后</el-button>
          <el-button type="primary" @click="startDownload">立即更新</el-button>
        </template>

        <template v-else-if="phase === 'downloading' || phase === 'installing'">
          <el-button disabled>{{ phase === 'installing' ? '安装中...' : '下载中...' }}</el-button>
        </template>

        <template v-else-if="phase === 'ready'">
          <el-button @click="handleClose">稍后重启</el-button>
          <el-button type="primary" @click="restartNow">立即重启</el-button>
        </template>

        <template v-else-if="phase === 'up_to_date' || phase === 'error'">
          <el-button type="primary" @click="handleClose">关闭</el-button>
        </template>

        <template v-else>
          <el-button @click="handleClose">关闭</el-button>
        </template>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { ElMessage } from 'element-plus';
import { Loading, Right, CircleCheck, CircleClose } from '@element-plus/icons-vue';
import { openUrl } from '@tauri-apps/plugin-opener';
import dayjs from 'dayjs';
import { useUpdaterStore } from '@/store/modules/updater';

const props = defineProps<{
  modelValue: boolean;
  currentVersion?: string;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
}>();

const updaterStore = useUpdaterStore();

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
});

const phase = computed(() => updaterStore.phase);

const dialogTitle = computed(() => {
  switch (phase.value) {
    case 'checking':
      return '检查更新';
    case 'available':
      return '发现新版本';
    case 'downloading':
    case 'installing':
      return '更新进行中';
    case 'ready':
      return '更新已就绪';
    case 'up_to_date':
      return '已是最新版本';
    case 'error':
      return '更新失败';
    default:
      return '更新';
  }
});

function formatDate(input?: string): string {
  if (!input) return '';
  const d = dayjs(input);
  return d.isValid() ? d.format('YYYY-MM-DD HH:mm') : input;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

async function startDownload() {
  try {
    await updaterStore.downloadAndInstall();
  } catch (e: any) {
    ElMessage.error(`更新失败: ${e?.message || e}`);
  }
}

async function restartNow() {
  try {
    await updaterStore.restartApp();
  } catch (e: any) {
    ElMessage.error(`重启失败: ${e?.message || e}`);
  }
}

function skipVersion() {
  updaterStore.skipCurrentVersion();
  handleClose();
}

async function openReleasesPage() {
  try {
    await openUrl('https://github.com/chaogei/windsurf-account-manager-simple/releases');
  } catch (e) {
    console.error('Failed to open releases page:', e);
  }
}

function handleClose() {
  if (updaterStore.isBusy) return;
  emit('update:modelValue', false);
}

function handleClosed() {
  updaterStore.dismiss();
}
</script>

<style scoped>
.update-dialog :deep(.el-dialog__header) {
  padding: 16px 20px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  margin-right: 0;
}

.update-body {
  min-height: 160px;
  padding: 8px 4px;
}

.state-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 24px 0;
  text-align: center;
  color: var(--el-text-color-regular);
}

.spin-icon {
  color: var(--el-color-primary);
}

.version-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 20px;
  padding: 14px 0 10px;
}

.version-col {
  display: flex;
  flex-direction: column;
  gap: 4px;
  align-items: center;
}

.version-label {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.version-value {
  font-size: 18px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.version-value.latest {
  color: var(--el-color-primary);
}

.arrow-icon {
  color: var(--el-text-color-placeholder);
}

.release-date {
  text-align: center;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-bottom: 12px;
}

.release-notes {
  background: var(--el-fill-color-light);
  border-radius: 8px;
  padding: 12px 14px;
  max-height: 220px;
  overflow-y: auto;
}

.notes-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  margin-bottom: 8px;
}

.notes-body {
  margin: 0;
  font-family: inherit;
  font-size: 13px;
  line-height: 1.6;
  color: var(--el-text-color-regular);
  white-space: pre-wrap;
  word-break: break-word;
}

.progress-title {
  font-size: 14px;
  color: var(--el-text-color-primary);
  margin: 0;
}

.progress-bar {
  width: 100%;
}

.progress-detail {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  font-variant-numeric: tabular-nums;
}

.ready-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--el-color-success);
  margin: 0;
}

.ready-desc {
  font-size: 13px;
  color: var(--el-text-color-regular);
  margin: 0;
}

.error-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--el-color-danger);
  margin: 0;
}

.error-detail {
  width: 100%;
  margin: 0;
  padding: 8px 10px;
  background: var(--el-fill-color-light);
  border-radius: 6px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
  color: var(--el-color-danger);
  white-space: pre-wrap;
  text-align: left;
  max-height: 120px;
  overflow-y: auto;
}

.error-hint {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin: 0;
}

.error-hint a {
  color: var(--el-color-primary);
  text-decoration: none;
}

.error-hint a:hover {
  text-decoration: underline;
}

.footer-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
</style>

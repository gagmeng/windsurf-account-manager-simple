<template>
  <el-dialog
    v-model="visible"
    title="批量导入账号"
    width="700px"
    :close-on-click-modal="false"
    @close="handleClose"
  >
    <div class="import-container">
      <!-- 认证流派（Devin Session / Auth1 Token 模式下无关，所以隐藏） -->
      <div class="mode-section" v-if="importMode !== 'devin_session_token' && importMode !== 'devin_auth1_token'">
        <span class="mode-label">认证流派：</span>
        <div class="mode-grid mode-grid--3col" role="radiogroup" aria-label="认证流派">
          <div
            v-for="opt in authProviderOptions"
            :key="opt.value"
            class="mode-card"
            :class="{ 'is-active': authProvider === opt.value }"
            :title="opt.desc"
            role="radio"
            :aria-checked="authProvider === opt.value"
            tabindex="0"
            @click="selectAuthProvider(opt.value)"
            @keydown.enter.prevent="selectAuthProvider(opt.value)"
            @keydown.space.prevent="selectAuthProvider(opt.value)"
          >
            <el-icon class="mode-card__icon">
              <component :is="opt.icon" />
            </el-icon>
            <span class="mode-card__title">{{ opt.title }}</span>
            <el-tag
              v-if="opt.tag"
              :type="opt.tagType"
              size="small"
              effect="light"
              class="mode-card__tag"
            >
              {{ opt.tag }}
            </el-tag>
            <el-icon v-if="authProvider === opt.value" class="mode-card__check">
              <Check />
            </el-icon>
          </div>
        </div>
      </div>

      <!-- 导入模式切换（Devin / 智能不支持 Refresh Token；devin_session_token 仅走 Devin） -->
      <div class="mode-section">
        <span class="mode-label">导入模式：</span>
        <div class="mode-grid mode-grid--3col" role="radiogroup" aria-label="导入模式">
          <div
            v-for="opt in importModeOptions"
            :key="opt.value"
            class="mode-card"
            :class="{
              'is-active': importMode === opt.value,
              'is-disabled': opt.disabled,
            }"
            :title="opt.disabled && opt.disabledReason ? opt.disabledReason : opt.desc"
            role="radio"
            :aria-checked="importMode === opt.value"
            :aria-disabled="opt.disabled"
            :tabindex="opt.disabled ? -1 : 0"
            @click="!opt.disabled && selectImportMode(opt.value)"
            @keydown.enter.prevent="!opt.disabled && selectImportMode(opt.value)"
            @keydown.space.prevent="!opt.disabled && selectImportMode(opt.value)"
          >
            <el-icon class="mode-card__icon">
              <component :is="opt.icon" />
            </el-icon>
            <span class="mode-card__title">{{ opt.title }}</span>
            <el-tag
              v-if="opt.tag"
              :type="opt.tagType"
              size="small"
              effect="light"
              class="mode-card__tag"
            >
              {{ opt.tag }}
            </el-tag>
            <el-icon v-if="importMode === opt.value" class="mode-card__check">
              <Check />
            </el-icon>
          </div>
        </div>
      </div>

      <!-- 格式说明 -->
      <el-alert
        :type="(importMode === 'devin_session_token' || importMode === 'devin_auth1_token') ? 'warning' : (authProvider === 'firebase' ? 'info' : 'success')"
        :closable="false"
        show-icon
        style="margin-bottom: 16px;"
      >
        <template #title>
          <span v-if="importMode === 'devin_session_token'">
            [Devin Session Token] 每行一个 token，格式：<code>devin-session-token$... 备注(可选)</code>。
            系统逐条调 GetCurrentUser 反查 email / 配额 / api_key 并落库；无效或过期的 token 会归入导入失败。
          </span>
          <span v-else-if="importMode === 'devin_auth1_token'">
            [Devin Auth1 Token] 每行一个 token，格式：<code>auth1_&lt;52字符&gt; 备注(可选)</code>。
            系统逐条用 auth1_token 换取 session_token → 反查 email 并落库；
            <strong>相比 Session Token 额外保留 auth1_token，后续支持刷新续期</strong>。
            多组织账号会自动选择主组织完成导入。
          </span>
          <span v-else-if="importMode === 'password' && authProvider === 'smart'">
            [智能识别] 每行一个账号，格式：<code>邮箱 密码 备注(可选)</code>。
            系统对每行并发三路嗅探 <strong>Firebase</strong> / <strong>Devin（Windsurf 桥接）</strong> /
            <strong>Devin 原生</strong>（app.devin.ai）并自动分派；
            SSO / 未设密码 / 未注册 / Devin 原生无密账号会归入导入失败。
          </span>
          <span v-else-if="importMode === 'password' && authProvider === 'devin'">
            [Devin] 每行一个账号，格式：<code>邮箱 密码 备注(可选)</code>。
            全部走 <strong>Windsurf 桥接侧</strong>，多组织账号将自动选择首个组织完成导入。
          </span>
          <span v-else-if="importMode === 'password' && authProvider === 'devin_native'">
            [Devin 原生] 每行一个账号，格式：<code>邮箱 密码 备注(可选)</code>。
            全部走 <strong>app.devin.ai 原生侧</strong>（省掉 sniff 探测开销），适用于已确认
            整批账号都是纯 Devin 原生注册的场景；多组织将自动选择首个组织完成导入。
          </span>
          <span v-else-if="importMode === 'password'">
            每行一个账号，支持空格或连字符分隔：
            <code>邮箱 密码 备注(可选)</code> 或 <code>邮箱---密码---备注(可选)</code>
          </span>
          <span v-else>每行一个 Token，格式：<code>refresh_token 备注(可选)</code></span>
        </template>
      </el-alert>

      <!-- 输入区域 -->
      <div class="input-section">
        <div class="section-header">
          <span class="section-title">{{ sectionTitle }}</span>
          <el-button type="primary" link @click="handleFileImport">
            <el-icon><Upload /></el-icon>
            从文件导入
          </el-button>
        </div>
        <el-input
          v-model="inputText"
          type="textarea"
          :rows="12"
          :placeholder="inputPlaceholder"
          @input="parseAccounts"
        />
        <input
          ref="fileInputRef"
          type="file"
          accept=".txt,.csv"
          style="display: none;"
          @change="handleFileSelected"
        />
      </div>

      <!-- 解析预览 -->
      <div class="preview-section" v-if="inputText.trim()">
        <div class="section-header">
          <span class="section-title">解析预览</span>
          <div class="stats">
            <el-tag type="success" size="small">有效: {{ validAccounts.length }}</el-tag>
            <el-tag v-if="invalidLines.length > 0" type="danger" size="small">
              无效: {{ invalidLines.length }}
            </el-tag>
          </div>
        </div>
        
        <!-- 有效账号表格 -->
        <el-table
          v-if="validAccounts.length > 0"
          :data="validAccounts.slice(0, 10)"
          size="small"
          max-height="200"
          stripe
        >
          <el-table-column prop="email" label="邮箱" min-width="180" />
          <el-table-column prop="password" label="密码" width="120">
            <template #default="{ row }">
              <span class="password-mask">{{ maskPassword(row.password) }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="remark" label="备注" min-width="100">
            <template #default="{ row }">
              <span class="remark-text">{{ row.remark || '-' }}</span>
            </template>
          </el-table-column>
        </el-table>
        <div v-if="validAccounts.length > 10" class="more-hint">
          ... 还有 {{ validAccounts.length - 10 }} 个账号
        </div>

        <!-- 无效行提示 -->
        <el-alert
          v-if="invalidLines.length > 0"
          type="warning"
          :closable="false"
          style="margin-top: 12px;"
        >
          <template #title>
            格式错误的行: {{ invalidLines.slice(0, 5).join(', ') }}
            <span v-if="invalidLines.length > 5">... 等 {{ invalidLines.length }} 行</span>
          </template>
        </el-alert>
      </div>

      <!-- 导入设置 -->
      <div class="settings-section">
        <div class="section-header">
          <span class="section-title">导入设置</span>
        </div>
        <div class="settings-content">
          <!-- 分组选择 -->
          <div class="setting-item">
            <span class="setting-label">分组:</span>
            <el-select
              v-model="selectedGroup"
              placeholder="选择分组（可选）"
              clearable
              style="width: 180px;"
            >
              <el-option
                v-for="group in settingsStore.groups"
                :key="group"
                :label="group"
                :value="group"
              />
            </el-select>
            <span class="setting-hint">留空则使用默认分组</span>
          </div>
          
          <!-- 标签选择 -->
          <div class="setting-item">
            <span class="setting-label">标签:</span>
            <el-select
              v-model="selectedTags"
              multiple
              collapse-tags
              collapse-tags-tooltip
              placeholder="选择标签（可选）"
              clearable
              style="width: 180px;"
            >
              <el-option
                v-for="tag in settingsStore.tags"
                :key="tag.name"
                :label="tag.name"
                :value="tag.name"
              >
                <span :style="{ color: tag.color }">{{ tag.name }}</span>
              </el-option>
            </el-select>
            <span class="setting-hint">留空则不添加标签</span>
          </div>
          
          <div class="setting-item">
            <span class="setting-label">并发模式:</span>
            <el-tag :type="unlimitedConcurrent ? 'danger' : 'primary'" size="small">
              {{ unlimitedConcurrent ? '全量并发' : `限制并发 (${concurrencyLimit})` }}
            </el-tag>
            <span class="setting-hint">可在设置中修改</span>
          </div>
          <div class="setting-item">
            <el-checkbox v-model="autoLogin">导入后自动登录</el-checkbox>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="handleClose">取消</el-button>
        <el-button
          type="primary"
          :disabled="validAccounts.length === 0"
          :loading="importing"
          @click="handleImport"
        >
          {{ importing ? '导入中...' : `导入 ${validAccounts.length} 个账号` }}
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import {
  Upload,
  MagicStick,
  Platform,
  User,
  Lock,
  Refresh,
  Connection,
  Check,
} from '@element-plus/icons-vue';
import { useSettingsStore } from '@/store';

const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void;
  (
    e: 'import',
    accounts: Array<{ email: string; password: string; remark: string; refreshToken?: string; sessionToken?: string; auth1Token?: string }>,
    autoLogin: boolean,
    group: string,
    tags: string[],
    mode: 'password' | 'refresh_token' | 'devin_session_token' | 'devin_auth1_token',
    // v1.7.6：新增 `'devin_native'` 手动模式，强制走 app.devin.ai/api/auth1/password/login
    authProvider: 'firebase' | 'devin' | 'devin_native' | 'smart',
  ): void;
}>();

const settingsStore = useSettingsStore();

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val)
});

const inputText = ref('');
const validAccounts = ref<Array<{ email: string; password: string; remark: string; refreshToken?: string; sessionToken?: string; auth1Token?: string }>>([]);
const invalidLines = ref<number[]>([]);
const autoLogin = ref(true);
const importing = ref(false);
const fileInputRef = ref<HTMLInputElement | null>(null);
const selectedGroup = ref<string>('');
const selectedTags = ref<string[]>([]);
const importMode = ref<'password' | 'refresh_token' | 'devin_session_token' | 'devin_auth1_token'>('password');
/// 认证流派：
/// - `smart`（默认，推荐）：逐行并发三路嗅探 Firebase / Devin / Devin 原生自动分派
/// - `firebase`：手动强制走原有 add_account + login_account（Firebase 体系）
/// - `devin`：手动强制走 add_account_by_devin_login（Windsurf 桥接侧）
/// - `devin_native`（v1.7.6 新增）：手动强制走 add_account_by_devin_native_login
///   （app.devin.ai/api/auth1 原生侧），适用于已知全部是纯 Devin 原生账号、
///   想省掉 sniff 开销的批量导入场景
const authProvider = ref<'firebase' | 'devin' | 'devin_native' | 'smart'>('smart');

// 切到 Devin / Devin 原生 / smart 后，Refresh Token 模式不适用
// （smart 因 Token 无 email 无法嗅探；Devin / Devin 原生 均为 Auth1 体系，不使用 refresh_token）
// 自动回落到邮箱密码模式并清空输入
watch(authProvider, (val) => {
  if (
    (val === 'devin' || val === 'devin_native' || val === 'smart') &&
    importMode.value === 'refresh_token'
  ) {
    importMode.value = 'password';
    handleModeChange();
  }
});

/**
 * 认证流派卡片选项（v1.7.6 扩充 4 项固定）
 *
 * - smart：推荐流派，逐行三路嗅探自动分派
 * - firebase：强制走传统 Firebase 体系
 * - devin：强制走 Windsurf 桥接侧 Devin Session
 * - devin_native：强制走 app.devin.ai 原生侧（节省 sniff 开销的批量迁 Devin 原生专用）
 */
const authProviderOptions = [
  {
    value: 'smart' as const,
    title: '智能识别',
    desc: '逐行并发三路嗅探 Firebase / Devin / Devin 原生，自动分派到对应命令',
    icon: MagicStick,
    tag: '推荐',
    tagType: 'primary' as const,
  },
  {
    value: 'firebase' as const,
    title: 'Firebase（官方）',
    desc: '手动强制走原有 add_account + login_account（Firebase 体系）',
    icon: Platform,
    tag: '',
    tagType: 'info' as const,
  },
  {
    value: 'devin' as const,
    title: 'Devin（新版）',
    desc: '强制走 add_account_by_devin_login（Windsurf 桥接侧），多组织自动选 orgs[0]',
    icon: User,
    tag: '新',
    tagType: 'success' as const,
  },
  {
    value: 'devin_native' as const,
    title: 'Devin 原生',
    desc: '强制走 add_account_by_devin_native_login（app.devin.ai/api/auth1），多组织自动选 orgs[0]',
    icon: User,
    tag: '新',
    tagType: 'success' as const,
  },
];

/**
 * 导入模式卡片选项（4 项，按 authProvider 动态 disabled）
 *
 * - password：邮箱 + 密码 + 可选备注
 * - refresh_token：Firebase refresh_token；只在 authProvider === 'firebase' 时可用
 * - devin_session_token：devin-session-token$... 迁入
 * - devin_auth1_token：auth1_... 迁入（多保留 auth1_token，支持刷新续期）
 */
const importModeOptions = computed(() => [
  {
    value: 'password' as const,
    title: '邮箱密码',
    desc: '每行一个账号：邮箱 密码 [备注]',
    icon: Lock,
    tag: '',
    tagType: 'info' as const,
    disabled: false,
    disabledReason: '',
  },
  {
    value: 'refresh_token' as const,
    title: 'Refresh Token',
    desc: '每行一个 Firebase refresh_token（+ 可选备注）',
    icon: Refresh,
    tag: '',
    tagType: 'info' as const,
    disabled:
      authProvider.value === 'devin' ||
      authProvider.value === 'devin_native' ||
      authProvider.value === 'smart',
    disabledReason:
      authProvider.value === 'devin' || authProvider.value === 'devin_native'
        ? 'Devin 体系不适用 refresh_token'
        : authProvider.value === 'smart'
          ? '智能识别需要 email，Token 格式无法嗅探'
          : '',
  },
  {
    value: 'devin_session_token' as const,
    title: 'Devin Session Token',
    desc: '粘贴 devin-session-token$... 直接迁入，无需邮箱密码',
    icon: Connection,
    tag: '迁入',
    tagType: 'warning' as const,
    disabled: false,
    disabledReason: '',
  },
  {
    value: 'devin_auth1_token' as const,
    title: 'Devin Auth1 Token',
    desc: '粘贴 auth1_... 直接迁入，多组织自动选主组织',
    icon: Connection,
    tag: '迁入',
    tagType: 'warning' as const,
    disabled: false,
    disabledReason: '',
  },
]);

/**
 * 切换认证流派：等价原 v-model="authProvider"。
 * 保留同值点击早返以避免触发 watch 側效应。
 */
function selectAuthProvider(value: 'smart' | 'firebase' | 'devin' | 'devin_native') {
  if (authProvider.value === value) return;
  authProvider.value = value;
}

/**
 * 切换导入模式：等价原 v-model + @change="handleModeChange"。
 * disabled 项已在模板层拦截，本函数只处理合法切换。
 */
function selectImportMode(value: 'password' | 'refresh_token' | 'devin_session_token' | 'devin_auth1_token') {
  if (importMode.value === value) return;
  importMode.value = value;
  handleModeChange();
}

const unlimitedConcurrent = computed(() => settingsStore.settings?.unlimitedConcurrentRefresh || false);
const concurrencyLimit = computed(() => settingsStore.settings?.concurrent_limit || 5);

// 按当前模式生成输入区的标题与占位符
const sectionTitle = computed(() => {
  if (importMode.value === 'devin_session_token') return 'Devin Session Token 列表';
  if (importMode.value === 'devin_auth1_token') return 'Devin Auth1 Token 列表';
  return importMode.value === 'password' ? '账号数据' : 'Refresh Token 列表';
});
const inputPlaceholder = computed(() => {
  if (importMode.value === 'password') {
    return 'user1@example.com password123 测试账号1\nuser2@example.com---password456\nuser3@example.com---password789---备注信息';
  }
  if (importMode.value === 'refresh_token') {
    return 'AMf-vBx...长token... 测试账号1\nAMf-vBy...长token...\nAMf-vBz...长token... 备注信息';
  }
  if (importMode.value === 'devin_auth1_token') {
    return 'auth1_uzq3g3s7leh774zhbwdlnbx7wotroz2srxeck53dujxa3bdje7yq 测试账号1\nauth1_xxxxxxxx...\nauth1_yyyyyyyy... 备注信息';
  }
  // devin_session_token
  return 'devin-session-token$eyJhbGciOi... 测试账号1\ndevin-session-token$eyJhbGciOi...\ndevin-session-token$eyJhbGciOi... 备注信息';
});

/**
 * 批量导入行切分：同时支持空白分隔与 `---`（3+ 个 `-`）分隔。
 *
 * 优先判定是否存在 3+ 个连续 `-`（量阈避免与 emails / refresh_token 中偶发的单/双连字符
 * 冲突）——存在则按它切；否则回退到空白切分，保证历史格式向后兼容。
 *
 * 空段会被过滤（避免连续多个分隔符间的空值干扰后续 parts.length 判定）。
 */
function splitLine(line: string): string[] {
  const trimmed = line.trim();
  if (/-{3,}/.test(trimmed)) {
    return trimmed.split(/-{3,}/).map(s => s.trim()).filter(s => s !== '');
  }
  return trimmed.split(/\s+/);
}

// 切换模式时重置
function handleModeChange() {
  inputText.value = '';
  validAccounts.value = [];
  invalidLines.value = [];
}

// 解析账号数据
function parseAccounts() {
  const lines = inputText.value.split('\n').filter(line => line.trim());
  validAccounts.value = [];
  invalidLines.value = [];

  if (importMode.value === 'password') {
    // 邮箱密码模式：支持 `email password remark` 与 `email---password---remark` 两种格式
    lines.forEach((line, index) => {
      const parts = splitLine(line);
      if (parts.length >= 2) {
        const [email, password, ...remarkParts] = parts;
        if (email.includes('@')) {
          validAccounts.value.push({
            email,
            password,
            remark: remarkParts.join(' ') || ''
          });
        } else {
          invalidLines.value.push(index + 1);
        }
      } else {
        invalidLines.value.push(index + 1);
      }
    });
  } else if (importMode.value === 'devin_session_token') {
    // Devin Session Token 模式：首个非空白段为 session_token，必须以 devin-session-token$ 开头
    lines.forEach((line, index) => {
      const parts = splitLine(line);
      if (parts.length >= 1 && parts[0].startsWith('devin-session-token$')) {
        const [token, ...remarkParts] = parts;
        validAccounts.value.push({
          email: `Session #${index + 1}`, // 实际 email 由后端反查填写；占位用于预览表格
          password: '',
          remark: remarkParts.join(' ') || '',
          sessionToken: token,
        });
      } else {
        invalidLines.value.push(index + 1);
      }
    });
  } else if (importMode.value === 'devin_auth1_token') {
    // Devin Auth1 Token 模式：首个非空白段为 auth1_token，必须以 auth1_ 开头且长度合理
    lines.forEach((line, index) => {
      const parts = splitLine(line);
      if (parts.length >= 1 && parts[0].startsWith('auth1_') && parts[0].length >= 20) {
        const [token, ...remarkParts] = parts;
        validAccounts.value.push({
          email: `Auth1 #${index + 1}`, // 实际 email 由后端反查填写；占位用于预览表格
          password: '',
          remark: remarkParts.join(' ') || '',
          auth1Token: token,
        });
      } else {
        invalidLines.value.push(index + 1);
      }
    });
  } else {
    // Refresh Token 模式
    lines.forEach((line, index) => {
      const parts = splitLine(line);
      if (parts.length >= 1 && parts[0].length >= 10) {
        const [token, ...remarkParts] = parts;
        validAccounts.value.push({
          email: `Token #${index + 1}`,
          password: '',
          remark: remarkParts.join(' ') || '',
          refreshToken: token
        });
      } else {
        invalidLines.value.push(index + 1);
      }
    });
  }
}

// 遮蔽密码显示
function maskPassword(password: string): string {
  if (password.length <= 4) {
    return '*'.repeat(password.length);
  }
  return password.slice(0, 2) + '*'.repeat(password.length - 4) + password.slice(-2);
}

// 从文件导入
function handleFileImport() {
  fileInputRef.value?.click();
}

function handleFileSelected(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;

  const reader = new FileReader();
  reader.onload = (e) => {
    const content = e.target?.result as string;
    inputText.value = content;
    parseAccounts();
  };
  reader.readAsText(file);
  
  // 重置input，允许再次选择同一文件
  input.value = '';
}

// 执行导入
function handleImport() {
  if (validAccounts.value.length === 0) return;
  importing.value = true;
  emit(
    'import',
    [...validAccounts.value],
    autoLogin.value,
    selectedGroup.value || '默认分组',
    [...selectedTags.value],
    importMode.value,
    authProvider.value,
  );
}

// 关闭对话框
function handleClose() {
  if (!importing.value) {
    inputText.value = '';
    validAccounts.value = [];
    invalidLines.value = [];
    selectedGroup.value = '';
    selectedTags.value = [];
    importMode.value = 'password';
    authProvider.value = 'smart';
    visible.value = false;
  }
}

// 导入完成后重置状态
function resetImporting() {
  importing.value = false;
}

// 监听对话框关闭
watch(visible, (val) => {
  if (!val) {
    inputText.value = '';
    validAccounts.value = [];
    invalidLines.value = [];
    selectedGroup.value = '';
    selectedTags.value = [];
    importing.value = false;
    importMode.value = 'password';
    authProvider.value = 'smart';
  }
});

defineExpose({
  resetImporting
});
</script>

<style scoped>
.import-container {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* ==================== 模式选择区（认证流派 / 导入模式） ==================== */

/* label 在左，卡片网格占右侧剩余空间；背景切换为中性浅灰，不再用绿背高亮 */
.mode-section {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 10px 12px;
  background: var(--el-fill-color-lighter);
  border-radius: 8px;
}

.mode-label {
  flex-shrink: 0;
  padding-top: 8px; /* 与卡片文字垂直居中 */
  min-width: 72px;
  font-size: 13px;
  font-weight: 500;
  color: var(--el-text-color-regular);
}

/* ==================== 卡片式 radio 网格（对齐 AddAccountDialog 风格） ==================== */

/* 默认 2 列；BatchImport 使用 .mode-grid--3col 显式声明 3 列。窄屏自动降为单列 */
.mode-grid {
  flex: 1;
  min-width: 0;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}
.mode-grid--3col {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

/* 单张卡片：单行 flex，矮版 34px；说明载于原生 title tooltip */
.mode-card {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border: 1.5px solid var(--el-border-color);
  border-radius: 6px;
  background-color: var(--el-bg-color);
  cursor: pointer;
  transition: border-color 0.2s ease, background-color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease;
  user-select: none;
  outline: none;
  min-height: 34px;
}

.mode-card:hover {
  border-color: var(--el-color-primary-light-3);
  background-color: var(--el-color-primary-light-9);
}

.mode-card:focus-visible {
  box-shadow: 0 0 0 2px var(--el-color-primary-light-5);
}

.mode-card.is-active {
  border-color: var(--el-color-primary);
  background-color: var(--el-color-primary-light-9);
  box-shadow: 0 0 0 2px var(--el-color-primary-light-7);
}

/* 禁用态：变灰不可点击；覆盖 hover 效果；原因说明通过模板的 :title 属性展示 */
.mode-card.is-disabled {
  opacity: 0.55;
  cursor: not-allowed;
  background-color: var(--el-fill-color-light);
}
.mode-card.is-disabled:hover {
  border-color: var(--el-border-color);
  background-color: var(--el-fill-color-light);
  box-shadow: none;
}

.mode-card__icon {
  flex-shrink: 0;
  font-size: 18px;
  color: var(--el-color-primary);
  width: 18px;
  height: 18px;
}

.mode-card.is-disabled .mode-card__icon {
  color: var(--el-text-color-placeholder);
}

.mode-card__title {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mode-card.is-disabled .mode-card__title {
  color: var(--el-text-color-secondary);
}

.mode-card__tag {
  flex-shrink: 0;
}

.mode-card__check {
  flex-shrink: 0;
  font-size: 14px;
  color: var(--el-color-primary);
}

/* 窄屏降级：dialog 宽度 < 680px 时卡片变单列，label 也换行 */
@media (max-width: 680px) {
  .mode-section {
    flex-direction: column;
    align-items: stretch;
  }
  .mode-label {
    padding-top: 0;
    min-width: 0;
  }
  .mode-grid,
  .mode-grid--3col {
    grid-template-columns: 1fr;
  }
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.section-title {
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.input-section :deep(.el-textarea__inner) {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
  line-height: 1.6;
}

.preview-section {
  background: var(--el-fill-color-light);
  border-radius: 8px;
  padding: 12px;
}

.stats {
  display: flex;
  gap: 8px;
}

.password-mask {
  font-family: monospace;
  color: var(--el-text-color-secondary);
}

.remark-text {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.more-hint {
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: 12px;
  padding: 8px 0;
}

.settings-section {
  background: var(--el-fill-color-lighter);
  border-radius: 8px;
  padding: 12px;
}

.settings-content {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.setting-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.setting-label {
  color: var(--el-text-color-regular);
  font-size: 13px;
}

.setting-hint {
  color: var(--el-text-color-placeholder);
  font-size: 12px;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

code {
  background: var(--el-fill-color);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: monospace;
}
</style>

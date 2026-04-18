<template>
  <el-dialog
    v-model="uiStore.showAddAccountDialog"
    title="添加账号"
    width="500px"
    :close-on-click-modal="false"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="currentRules"
      label-width="100px"
      autocomplete="off"
    >
      <!-- 添加方式切换 -->
      <el-form-item label="添加方式">
        <el-radio-group v-model="addMode" @change="handleModeChange">
          <el-radio value="smart">
            智能识别
            <el-tag size="small" type="primary" style="margin-left: 4px;">推荐</el-tag>
          </el-radio>
          <el-radio value="password">邮箱密码</el-radio>
          <el-radio value="refresh_token">Refresh Token</el-radio>
          <el-radio value="devin">
            Devin 账密
            <el-tag size="small" type="success" style="margin-left: 4px;">新</el-tag>
          </el-radio>
          <el-radio value="devin_session">
            Devin Session Token
            <el-tag size="small" type="warning" style="margin-left: 4px;">迁入</el-tag>
          </el-radio>
        </el-radio-group>
      </el-form-item>

      <!-- 智能识别模式：仅输入邮箱+密码，自动识别 Firebase / Devin 流派 -->
      <template v-if="addMode === 'smart'">
        <el-alert
          type="success"
          :closable="false"
          show-icon
          style="margin-bottom: 18px;"
        >
          <template #title>
            <span style="font-size: 12px;">
              输入邮箱密码，系统自动识别 <strong>Firebase</strong> / <strong>Devin Auth1</strong> 账号并分派到正确的登录协议
            </span>
          </template>
        </el-alert>
        <el-form-item label="邮箱" prop="email">
          <el-input
            v-model="formData.email"
            placeholder="请输入邮箱"
            :prefix-icon="Message"
            autocomplete="off"
          />
        </el-form-item>
        <el-form-item label="密码" prop="password">
          <el-input
            v-model="formData.password"
            type="password"
            placeholder="请输入密码"
            :prefix-icon="Lock"
            show-password
            autocomplete="new-password"
          />
        </el-form-item>
      </template>

      <!-- 邮箱密码模式（旧 Firebase 体系） -->
      <template v-else-if="addMode === 'password'">
        <el-form-item label="邮箱" prop="email">
          <el-input
            v-model="formData.email"
            placeholder="请输入邮箱"
            :prefix-icon="Message"
            autocomplete="off"
          />
        </el-form-item>
        
        <el-form-item label="密码" prop="password">
          <el-input
            v-model="formData.password"
            type="password"
            placeholder="请输入密码"
            :prefix-icon="Lock"
            show-password
            autocomplete="new-password"
          />
        </el-form-item>
      </template>

      <!-- Refresh Token 模式 -->
      <template v-else-if="addMode === 'refresh_token'">
        <el-form-item label="Refresh Token" prop="refreshToken">
          <el-input
            v-model="formData.refreshToken"
            type="textarea"
            :rows="3"
            placeholder="请输入 Refresh Token"
          />
        </el-form-item>
      </template>

      <!-- Devin Session Token 模式：直接粘贴 devin-session-token$... 迁入 -->
      <template v-else-if="addMode === 'devin_session'">
        <el-alert
          type="warning"
          :closable="false"
          show-icon
          style="margin-bottom: 18px;"
        >
          <template #title>
            <span style="font-size: 12px;">
              粘贴完整 <code>devin-session-token$...</code> 的 session_token，
              系统自动调 GetCurrentUser 反查 email / 配额 / api_key 并落库。
              适用于已在浏览器登录后从 localStorage / cookie 拷贝 token 的迁入场景。
            </span>
          </template>
        </el-alert>
        <el-form-item label="Session Token" prop="sessionToken">
          <el-input
            v-model="formData.sessionToken"
            type="textarea"
            :rows="3"
            placeholder="请粘贴完整的 devin-session-token$... 令牌"
          />
        </el-form-item>
      </template>

      <!-- Devin 账密模式（新 Devin Session 体系） -->
      <template v-else>
        <el-form-item label="邮箱" prop="email">
          <el-input
            v-model="formData.email"
            placeholder="请输入 Devin 账号邮箱"
            :prefix-icon="Message"
            autocomplete="off"
          />
        </el-form-item>

        <el-form-item label="密码" prop="password">
          <el-input
            v-model="formData.password"
            type="password"
            placeholder="请输入 Devin 账号密码"
            :prefix-icon="Lock"
            show-password
            autocomplete="new-password"
          />
        </el-form-item>

        <el-alert
          type="info"
          :closable="false"
          show-icon
          style="margin-bottom: 18px;"
        >
          <template #title>
            <span style="font-size: 12px;">
              通过 Devin Session 新体系登录（<code>/_devin-auth/password/login</code> +
              <code>WindsurfPostAuth</code>），无 Google API Key 限制、无需 Token 刷新
            </span>
          </template>
        </el-alert>
      </template>
      
      <el-form-item label="备注名称" prop="nickname">
        <el-input
          v-model="formData.nickname"
          placeholder="留空则使用邮箱用户名"
          :prefix-icon="User"
        />
      </el-form-item>
      
      <el-form-item label="分组">
        <el-select
          v-model="formData.group"
          placeholder="选择分组"
          clearable
        >
          <el-option
            v-for="group in settingsStore.groups"
            :key="group"
            :label="group"
            :value="group"
          />
        </el-select>
      </el-form-item>
      
      <el-form-item label="标签">
        <el-select
          v-model="formData.tags"
          multiple
          filterable
          allow-create
          placeholder="输入或选择标签"
          style="width: 100%"
        >
          <el-option
            v-for="tag in settingsStore.tags"
            :key="tag.name"
            :label="tag.name"
            :value="tag.name"
          >
            <span :style="getTagOptionStyle(tag.color)">{{ tag.name }}</span>
          </el-option>
        </el-select>
      </el-form-item>
    </el-form>
    
    <template #footer>
      <el-button @click="handleClose">取消</el-button>
      <el-button type="primary" @click="handleSubmit" :loading="loading">
        确定
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { Message, Lock, User } from '@element-plus/icons-vue';
import { useAccountsStore, useSettingsStore, useUIStore } from '@/store';
import { apiService, accountApi, devinApi } from '@/api';
import type { WindsurfOrg, LoginMethodSniffResult } from '@/types';
import { invoke } from '@tauri-apps/api/core';

const accountsStore = useAccountsStore();
const settingsStore = useSettingsStore();
const uiStore = useUIStore();

const formRef = ref<FormInstance>();
const loading = ref(false);
const addMode = ref<'smart' | 'password' | 'refresh_token' | 'devin' | 'devin_session'>('smart');

const formData = reactive({
  email: '',
  password: '',
  refreshToken: '',
  sessionToken: '',
  nickname: '',
  group: '默认分组',
  tags: [] as string[]
});

// 邮箱密码模式的验证规则
const passwordRules: FormRules = {
  email: [
    { required: true, message: '请输入邮箱', trigger: 'blur' },
    { type: 'email', message: '请输入有效的邮箱地址', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, message: '密码长度至少6位', trigger: 'blur' }
  ],
  nickname: [
    { max: 20, message: '备注名称最多20个字符', trigger: 'blur' }
  ]
};

// Refresh Token 模式的验证规则
const refreshTokenRules: FormRules = {
  refreshToken: [
    { required: true, message: '请输入 Refresh Token', trigger: 'blur' },
    { min: 10, message: 'Refresh Token 格式不正确', trigger: 'blur' }
  ],
  nickname: [
    { max: 20, message: '备注名称最多20个字符', trigger: 'blur' }
  ]
};

// Devin 账密模式的验证规则（与 passwordRules 一致）
const devinRules: FormRules = {
  email: [
    { required: true, message: '请输入 Devin 账号邮箱', trigger: 'blur' },
    { type: 'email', message: '请输入有效的邮箱地址', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入 Devin 账号密码', trigger: 'blur' },
    { min: 6, message: '密码长度至少6位', trigger: 'blur' }
  ],
  nickname: [
    { max: 20, message: '备注名称最多20个字符', trigger: 'blur' }
  ]
};

// Devin Session Token 模式的验证规则
const devinSessionRules: FormRules = {
  sessionToken: [
    { required: true, message: '请粘贴 Devin session_token', trigger: 'blur' },
    {
      validator: (_rule, value: string, callback) => {
        const trimmed = (value || '').trim();
        if (!trimmed) return callback(new Error('请粘贴 Devin session_token'));
        if (!trimmed.startsWith('devin-session-token$')) {
          return callback(new Error('session_token 必须以 devin-session-token$ 前缀开头'));
        }
        callback();
      },
      trigger: 'blur',
    },
  ],
  nickname: [
    { max: 20, message: '备注名称最多20个字符', trigger: 'blur' }
  ]
};

// 根据模式选择验证规则
const currentRules = computed(() => {
  // 智能模式复用邮箱密码规则（同样需要 email + password）
  if (addMode.value === 'smart' || addMode.value === 'password') return passwordRules;
  if (addMode.value === 'refresh_token') return refreshTokenRules;
  if (addMode.value === 'devin_session') return devinSessionRules;
  return devinRules;
});

// 切换模式时重置表单
function handleModeChange() {
  formRef.value?.resetFields();
}

// 获取标签选项样式
function getTagOptionStyle(color: string): Record<string, string> {
  if (!color) return {};
  
  let r = 0, g = 0, b = 0;
  let parsed = false;
  
  // 解析 rgba 或 rgb 格式
  if (color.startsWith('rgba') || color.startsWith('rgb')) {
    const match = color.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
    if (match) {
      r = parseInt(match[1]);
      g = parseInt(match[2]);
      b = parseInt(match[3]);
      parsed = true;
    }
  } 
  // 解析 HEX 格式
  if (!parsed && color.startsWith('#')) {
    const hex = color.slice(1);
    if (hex.length >= 6) {
      r = parseInt(hex.slice(0, 2), 16);
      g = parseInt(hex.slice(2, 4), 16);
      b = parseInt(hex.slice(4, 6), 16);
      parsed = true;
    }
  }
  
  if (!parsed) return {};
  
  return {
    color: `rgb(${r}, ${g}, ${b})`,
    fontWeight: '500'
  };
}

async function handleSubmit() {
  if (!formRef.value) return;
  
  await formRef.value.validate(async (valid) => {
    if (!valid) return;
    
    loading.value = true;
    try {
      if (addMode.value === 'refresh_token') {
        // Refresh Token 模式
        const trimmedToken = formData.refreshToken.trim();
        const trimmedNickname = formData.nickname.trim() || undefined;
        
        if (!trimmedToken) {
          ElMessage.error('Refresh Token 不能为空');
          loading.value = false;
          return;
        }
        
        // 调用后端接口添加账号
        const result = await invoke<any>('add_account_by_refresh_token', {
          refreshToken: trimmedToken,
          nickname: trimmedNickname,
          tags: formData.tags,
          group: formData.group || '默认分组'
        });
        
        if (result.success) {
          ElMessage.success(`账号 ${result.email} 添加成功`);
          // 刷新账号列表
          await accountsStore.loadAccounts();
          handleClose();
        } else {
          ElMessage.error(result.error || '添加失败');
        }
      } else if (addMode.value === 'devin') {
        // Devin 账密模式
        await handleDevinSubmit();
      } else if (addMode.value === 'devin_session') {
        // Devin Session Token 直接迁入
        await handleDevinSessionSubmit();
      } else if (addMode.value === 'smart') {
        // 智能识别模式：先嗅探再分派
        await handleSmartSubmit();
      } else {
        // 邮箱密码模式（旧 Firebase）
        await handleFirebaseSubmit();
      }
    } catch (error) {
      ElMessage.error(`添加失败: ${error}`);
    } finally {
      loading.value = false;
    }
  });
}

/**
 * Firebase 邮箱密码登录流程（原 'password' 分支抽取）
 *
 * 供 'password' 模式直接调用，也被 'smart' 模式在嗅探结果为 firebase 时复用
 */
async function handleFirebaseSubmit() {
  const trimmedEmail = formData.email.trim();
  const trimmedPassword = formData.password.trim();
  const trimmedNickname = formData.nickname.trim() || trimmedEmail.split('@')[0];

  if (!trimmedPassword) {
    ElMessage.error('密码不能为空或只包含空格');
    return;
  }

  // 添加账号
  const newAccount = await accountsStore.addAccount({
    email: trimmedEmail,
    password: trimmedPassword,
    nickname: trimmedNickname,
    tags: formData.tags,
    group: formData.group || '默认分组'
  });

  ElMessage.success('账号添加成功，正在获取账号信息...');

  // 自动登录并获取账号详细信息
  try {
    const loginResult = await apiService.loginAccount(newAccount.id);

    if (loginResult.success) {
      const latestAccount = await accountApi.getAccount(newAccount.id);
      await accountsStore.updateAccount(latestAccount);
      ElMessage.success('账号信息已更新');
    } else {
      ElMessage.warning('账号已添加，但登录失败，请手动刷新');
    }
  } catch (infoError) {
    console.error('获取账号信息失败:', infoError);
    ElMessage.warning('账号已添加，但获取详细信息失败，请手动刷新');
  }

  handleClose();
}

/**
 * 智能识别模式：先嗅探账号属于 Firebase / Devin 哪一派，再自动分派
 *
 * 后端 `sniff_login_method` 并发调两侧探测端点，返回 `recommended` 字段：
 * - firebase：走 `handleFirebaseSubmit`
 * - devin：　走 `handleDevinSubmit`
 * - sso / no_password / not_found / blocked：弹对话框指引用户处理
 */
async function handleSmartSubmit() {
  const trimmedEmail = formData.email.trim();
  const trimmedPassword = formData.password.trim();

  if (!trimmedEmail || !trimmedPassword) {
    ElMessage.error('邮箱和密码不能为空');
    return;
  }

  ElMessage.info('正在识别账号类型……');

  let sniff: LoginMethodSniffResult;
  try {
    sniff = await devinApi.sniffLoginMethod(trimmedEmail);
  } catch (e) {
    ElMessage.error(`识别登录方式失败: ${e}`);
    return;
  }

  switch (sniff.recommended) {
    case 'firebase':
      ElMessage.success('已识别为 Firebase 账号，正在登录……');
      await handleFirebaseSubmit();
      break;
    case 'devin':
      ElMessage.success('已识别为 Devin 账号，正在登录……');
      await handleDevinSubmit();
      break;
    case 'sso':
      await ElMessageBox.alert(
        `${sniff.reason}\n\n请先在浏览器中完成 SSO 登录，然后改用「Refresh Token」模式导入。`,
        '无法自动登录（企业 SSO）',
        { type: 'info', confirmButtonText: '知道了' }
      ).catch(() => {});
      break;
    case 'no_password':
      await ElMessageBox.alert(
        `${sniff.reason}\n\n请先访问 windsurf.com 使用 Google/GitHub 登录或重置密码，然后再尝试。`,
        '此账号未设置密码',
        { type: 'warning', confirmButtonText: '知道了' }
      ).catch(() => {});
      break;
    case 'not_found':
      await ElMessageBox.alert(
        `${sniff.reason}\n\n若确认邮箱无误，请先完成注册流程。`,
        '账号不存在',
        { type: 'warning', confirmButtonText: '知道了' }
      ).catch(() => {});
      break;
    case 'blocked':
      await ElMessageBox.alert(
        `${sniff.reason}`,
        '账号受限',
        { type: 'error', confirmButtonText: '知道了' }
      ).catch(() => {});
      break;
    default:
      ElMessage.error(`未知的嗅探结果：${sniff.recommended}`);
  }
}

/**
 * Devin Session Token 直接迁入流程
 *
 * 用户仅需粘贴 `devin-session-token$...` 即可建号，
 * 后端自动调 GetCurrentUser 反查 email / api_key / 配额 并落库。
 */
async function handleDevinSessionSubmit() {
  const trimmedToken = formData.sessionToken.trim();
  const trimmedNickname = formData.nickname.trim() || undefined;

  if (!trimmedToken) {
    ElMessage.error('Session Token 不能为空');
    return;
  }
  if (!trimmedToken.startsWith('devin-session-token$')) {
    ElMessage.error('session_token 必须以 devin-session-token$ 前缀开头');
    return;
  }

  ElMessage.info('正在反查 Devin 账号信息……');
  const result = await devinApi.addAccountBySessionToken({
    sessionToken: trimmedToken,
    nickname: trimmedNickname,
    tags: formData.tags,
    group: formData.group || '默认分组',
  });

  if (result.success) {
    ElMessage.success(`Devin 账号 ${result.email} 已通过 session_token 导入成功`);
    await accountsStore.loadAccounts();
    handleClose();
  } else {
    ElMessage.error(result.message || 'Session Token 迁入失败');
  }
}

/**
 * Devin 账密登录的完整流程
 *
 * 1. 调用 addAccountByLogin
 * 2. 若返回 requires_org_selection=true，弹出组织选择对话框
 * 3. 用户选择后调用 addAccountWithOrg 完成创建
 */
async function handleDevinSubmit() {
  const trimmedEmail = formData.email.trim();
  const trimmedPassword = formData.password.trim();
  const trimmedNickname = formData.nickname.trim() || undefined;

  if (!trimmedEmail || !trimmedPassword) {
    ElMessage.error('邮箱和密码不能为空');
    return;
  }

  const result = await devinApi.addAccountByLogin({
    email: trimmedEmail,
    password: trimmedPassword,
    nickname: trimmedNickname,
    tags: formData.tags,
    group: formData.group || '默认分组',
  });

  // 分支 1：需要选择组织
  if (result.requires_org_selection && result.auth1_token && result.orgs) {
    const chosenOrg = await promptOrgSelection(result.orgs);
    if (!chosenOrg) {
      ElMessage.info('已取消多组织选择');
      return;
    }

    const confirmResult = await devinApi.addAccountWithOrg({
      email: trimmedEmail,
      auth1Token: result.auth1_token,
      orgId: chosenOrg,
      nickname: trimmedNickname,
      tags: formData.tags,
      group: formData.group || '默认分组',
    });

    if (confirmResult.success) {
      ElMessage.success(`Devin 账号 ${trimmedEmail} 添加成功`);
      await accountsStore.loadAccounts();
      handleClose();
    } else {
      ElMessage.error(confirmResult.message || '组织选择后创建账号失败');
    }
    return;
  }

  // 分支 2：直接成功
  if (result.success) {
    ElMessage.success(`Devin 账号 ${result.email} 添加成功`);
    await accountsStore.loadAccounts();
    handleClose();
  } else {
    ElMessage.error(result.message || 'Devin 登录失败');
  }
}

/**
 * 多组织选择对话框
 *
 * 使用 ElMessageBox 以最小依赖实现，返回用户选择的 org_id 或 null（取消）
 */
async function promptOrgSelection(orgs: WindsurfOrg[]): Promise<string | null> {
  // 构建选项 HTML（Element Plus 的 MessageBox 支持 dangerouslyUseHTMLString）
  const optionsHtml = orgs
    .map(
      (org, i) => `
        <div style="margin: 8px 0;">
          <label style="display: flex; align-items: center; cursor: pointer;">
            <input type="radio" name="devin-org" value="${escapeHtml(org.id)}" ${i === 0 ? 'checked' : ''} style="margin-right: 8px;" />
            <div>
              <div style="font-weight: 600;">${escapeHtml(org.name) || '(未命名组织)'}</div>
              <div style="font-size: 11px; color: #909399; font-family: monospace;">${escapeHtml(org.id)}</div>
            </div>
          </label>
        </div>
      `
    )
    .join('');

  try {
    await ElMessageBox({
      title: `该账号属于 ${orgs.length} 个组织，请选择`,
      message: `<div id="devin-org-picker">${optionsHtml}</div>`,
      dangerouslyUseHTMLString: true,
      showCancelButton: true,
      confirmButtonText: '选择此组织',
      cancelButtonText: '取消',
      closeOnClickModal: false,
    });

    const checked = document.querySelector<HTMLInputElement>(
      '#devin-org-picker input[name="devin-org"]:checked'
    );
    return checked ? checked.value : null;
  } catch {
    return null;
  }
}

/** 转义 HTML 以避免 XSS */
function escapeHtml(s: string): string {
  return (s || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function handleClose() {
  uiStore.closeAddAccountDialog();
  formRef.value?.resetFields();
  
  // 重置表单数据
  formData.email = '';
  formData.password = '';
  formData.refreshToken = '';
  formData.sessionToken = '';
  formData.nickname = '';
  formData.group = '默认分组';
  formData.tags = [];
  addMode.value = 'smart';
}
</script>

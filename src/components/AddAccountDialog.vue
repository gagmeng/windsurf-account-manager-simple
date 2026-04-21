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
      <!-- 添加方式切换：紧凑卡片网格（2 列，单行布局，窄屏自动单列）
           desc 说明载在原生 title 属性，鼠标悬停显示。 -->
      <el-form-item label="添加方式">
        <div class="mode-grid" role="radiogroup" aria-label="添加方式">
          <div
            v-for="opt in modeOptions"
            :key="opt.value"
            class="mode-card"
            :class="{ 'is-active': addMode === opt.value }"
            :title="opt.desc"
            role="radio"
            :aria-checked="addMode === opt.value"
            tabindex="0"
            @click="selectMode(opt.value)"
            @keydown.enter.prevent="selectMode(opt.value)"
            @keydown.space.prevent="selectMode(opt.value)"
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
            <el-icon v-if="addMode === opt.value" class="mode-card__check">
              <Check />
            </el-icon>
          </div>
        </div>
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

      <!-- Devin Auth1 Token 模式：直接粘贴 auth1_xxx 迁入（与 Session Token 对称，但多保留 auth1_token 使刷新可用） -->
      <template v-else-if="addMode === 'devin_auth1'">
        <el-alert
          type="warning"
          :closable="false"
          show-icon
          style="margin-bottom: 18px;"
        >
          <template #title>
            <span style="font-size: 12px;">
              粘贴完整 <code>auth1_&lt;52 字符&gt;</code> 的 auth1_token（浏览器 localStorage 键名
              <code>devin_auth1_token</code>），系统自动换取 session_token → 反查 email / 配额并落库。
              <strong>相比 Session Token 多保留 auth1_token，后续可用「刷新」命令自动续期</strong>。
            </span>
          </template>
        </el-alert>
        <el-form-item label="Auth1 Token" prop="auth1Token">
          <el-input
            v-model="formData.auth1Token"
            type="textarea"
            :rows="3"
            placeholder="请粘贴完整的 auth1_xxxxxxxx... 令牌"
          />
        </el-form-item>
      </template>

      <!-- Devin 邮箱验证码统一卡：两步流程；Step 0 选择 source / flow，Step 1 按 source+flow 渲染字段 -->
      <template v-else-if="addMode === 'devin_email_code'">
        <!-- 顶部说明：按 source + flow 动态文案 -->
        <el-alert
          v-if="devinEmailCodeSource === 'devin_native'"
          type="success"
          :closable="false"
          show-icon
          style="margin-bottom: 16px;"
        >
          <template #title>
            <span style="font-size: 12px;">
              <strong>Devin 原生注册</strong>：通过 <code>app.devin.ai/api/auth1/*</code> 端口建号，
              账号主归属 <strong>Devin</strong>（JWT product=Devin），注册后自动桥接 Windsurf。
              <strong>只需邮箱 + 验证码两步</strong>，不需要设置密码/姓名。
            </span>
          </template>
        </el-alert>
        <el-alert
          v-else-if="devinEmailCodeFlow === 'signup'"
          type="warning"
          :closable="false"
          show-icon
          style="margin-bottom: 16px;"
        >
          <template #title>
            <span style="font-size: 12px;">
              <strong>Windsurf 侧注册</strong>：通过 <code>windsurf.com/_devin-auth/*</code> 端口创建新账号，
              需设置密码和姓名。<strong>此流程会创建新账号并落库</strong>。
            </span>
          </template>
        </el-alert>
        <el-alert
          v-else
          type="info"
          :closable="false"
          show-icon
          style="margin-bottom: 16px;"
        >
          <template #title>
            <span style="font-size: 12px;">
              <strong>Windsurf 侧无密码登录</strong>：适用已存在 Devin 账号但无密码的场景
              （SSO 迁移 / 忘记密码 / Google・GitHub 登录过的账号），通过邮箱验证码登录并添加。
              <strong>此流程不会创建新账号</strong>。
            </span>
          </template>
        </el-alert>

        <el-steps :active="devinEmailCodeStep" finish-status="success" simple style="margin-bottom: 20px;">
          <el-step title="选择来源 / 发送验证码" />
          <el-step :title="devinEmailCodeSource === 'windsurf_side' && devinEmailCodeFlow === 'signup' ? '完成注册' : (devinEmailCodeSource === 'devin_native' ? '完成注册' : '输入验证码')" />
        </el-steps>

        <!-- Step 0：选择 source + 可选 flow + 输入邮箱 -->
        <template v-if="devinEmailCodeStep === 0">
          <!-- 注册来源：mode-card 网格，与主「添加方式」卡片风格对齐 -->
          <el-form-item label="注册来源">
            <div class="mode-grid" role="radiogroup" aria-label="注册来源">
              <div
                v-for="opt in sourceOptions"
                :key="opt.value"
                class="mode-card"
                :class="{ 'is-active': devinEmailCodeSource === opt.value }"
                :title="opt.desc"
                role="radio"
                :aria-checked="devinEmailCodeSource === opt.value"
                tabindex="0"
                @click="selectSource(opt.value)"
                @keydown.enter.prevent="selectSource(opt.value)"
                @keydown.space.prevent="selectSource(opt.value)"
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
                <el-icon v-if="devinEmailCodeSource === opt.value" class="mode-card__check">
                  <Check />
                </el-icon>
              </div>
            </div>
          </el-form-item>

          <!-- 子流程：仅 Windsurf 侧需要区分 login / signup；Devin 原生只有 signup -->
          <el-form-item v-if="devinEmailCodeSource === 'windsurf_side'" label="子流程">
            <div class="mode-grid" role="radiogroup" aria-label="子流程">
              <div
                v-for="opt in flowOptions"
                :key="opt.value"
                class="mode-card"
                :class="{ 'is-active': devinEmailCodeFlow === opt.value }"
                :title="opt.desc"
                role="radio"
                :aria-checked="devinEmailCodeFlow === opt.value"
                tabindex="0"
                @click="selectFlow(opt.value)"
                @keydown.enter.prevent="selectFlow(opt.value)"
                @keydown.space.prevent="selectFlow(opt.value)"
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
                <el-icon v-if="devinEmailCodeFlow === opt.value" class="mode-card__check">
                  <Check />
                </el-icon>
              </div>
            </div>
          </el-form-item>

          <el-form-item label="邮箱" prop="email">
            <el-input
              v-model="formData.email"
              :placeholder="devinEmailCodeSource === 'devin_native' ? '请输入用于注册的新邮箱' : '请输入 Devin 账号邮箱'"
              :prefix-icon="Message"
              autocomplete="off"
            />
          </el-form-item>
        </template>

        <!-- Step 1：输入验证码（按 source + flow 决定是否额外要密码/姓名） -->
        <template v-else>
          <el-alert
            v-if="devinEmailCodeSource === 'windsurf_side' && devinEmailCodeFlow === 'signup'"
            type="warning"
            :closable="false"
            show-icon
            style="margin-bottom: 16px;"
          >
            <template #title>
              <span style="font-size: 12px;">
                <strong>注册新账号</strong>：验证码已发送至 {{ formData.email }}，
                请填入验证码并设置密码/姓名完成注册
              </span>
            </template>
          </el-alert>
          <el-alert v-else type="success" :closable="false" show-icon style="margin-bottom: 16px;">
            验证码已发送至：{{ formData.email }}
          </el-alert>

          <el-form-item label="邮箱">
            <el-input :model-value="formData.email" disabled />
          </el-form-item>
          <el-form-item label="验证码" prop="devinEmailCodeOtp">
            <el-input
              v-model="formData.devinEmailCodeOtp"
              placeholder="请输入邮箱中的 6 位验证码"
              maxlength="10"
            />
          </el-form-item>

          <!-- 仅 source=windsurf_side + flow=signup 需要密码/姓名 -->
          <template v-if="devinEmailCodeSource === 'windsurf_side' && devinEmailCodeFlow === 'signup'">
            <el-form-item label="新密码" prop="devinEmailCodePassword">
              <el-input
                v-model="formData.devinEmailCodePassword"
                type="password"
                placeholder="请设置新账号密码，至少 6 位"
                :prefix-icon="Lock"
                show-password
                autocomplete="new-password"
              />
            </el-form-item>
            <el-form-item label="姓名" prop="devinEmailCodeName">
              <el-input
                v-model="formData.devinEmailCodeName"
                placeholder="请输入账号显示名称（留空用邮箱前缀）"
                :prefix-icon="User"
              />
            </el-form-item>
          </template>
        </template>
      </template>

      <!-- Devin 账密模式（新 Devin Session 体系） -->
      <template v-else>
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

      <!-- Devin 邮箱验证码合并卡：按 step + source + flow 动态按钮文案 -->
      <template v-if="addMode === 'devin_email_code'">
        <el-button v-if="devinEmailCodeStep === 1" @click="devinEmailCodeStep = 0" :disabled="loading">
          上一步
        </el-button>
        <el-button type="primary" @click="handleSubmit" :loading="loading">
          {{ devinEmailCodeStep === 0
              ? '发送验证码'
              : (devinEmailCodeSource === 'devin_native'
                  ? '完成 Devin 原生注册'
                  : (devinEmailCodeFlow === 'signup' ? '完成注册' : '完成添加')) }}
        </el-button>
      </template>

      <!-- 其他模式：统一"确定"按钮 -->
      <el-button v-else type="primary" @click="handleSubmit" :loading="loading">
        确定
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed, nextTick } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { Message, Lock, User, MagicStick, Refresh, Connection, Check, Key } from '@element-plus/icons-vue';
import { useAccountsStore, useSettingsStore, useUIStore } from '@/store';
import { apiService, accountApi, devinApi } from '@/api';
import type { WindsurfOrg, LoginMethodSniffResult } from '@/types';
import { invoke } from '@tauri-apps/api/core';

const accountsStore = useAccountsStore();
const settingsStore = useSettingsStore();
const uiStore = useUIStore();

const formRef = ref<FormInstance>();
const loading = ref(false);
const addMode = ref<'smart' | 'password' | 'refresh_token' | 'devin' | 'devin_session' | 'devin_auth1' | 'devin_email_code'>('smart');

// Devin 邮箱验证码合并卡（mode === 'devin_email_code' 专属）的状态：
//   - step 0：选择 source / flow + 输入邮箱 + 发送验证码
//   - step 1：输入验证码（按 source+flow 可能额外填密码/姓名）
//
// source：一级子单选
//   - 'devin_native'【原来的 devin_native_signup 卡合并而来】
//     端口走 app.devin.ai/api/auth1/*，账号主归属 Devin（JWT product=Devin）+ 自动桥接 Windsurf
//     只有 signup 子流程，不需密码/姓名
//   - 'windsurf_side'
//     端口走 windsurf.com/_devin-auth/*，账号主归属 Windsurf
//     支持 flow=login / signup；signup 需密码+姓名
const devinEmailCodeSource = ref<'devin_native' | 'windsurf_side'>('devin_native');
// step 0：输入邮箱 + 发送验证码；step 1：输入验证码 + 完成登录/注册
const devinEmailCodeStep = ref<0 | 1>(0);
// /email/start 返回的 email_verification_token，用于后续 /email/complete
const devinEmailCodeEmailToken = ref('');
// flow：仅在 source==='windsurf_side' 时生效，当 source==='devin_native' 时满量视作 'signup'
//   - 'login'：登录已有无密码账号（SSO 迁移 / 忘密 / Google・GitHub 登录过的账号）
//   - 'signup'：注册新账号（额外要密码+姓名）
//   - 从 smart 分派 not_found 快捷按钮进入时自动设为 'signup' + source=windsurf_side
const devinEmailCodeFlow = ref<'login' | 'signup'>('login');

const formData = reactive({
  email: '',
  password: '',
  refreshToken: '',
  sessionToken: '',
  auth1Token: '',
  devinEmailCodeOtp: '',
  devinEmailCodePassword: '',
  devinEmailCodeName: '',
  nickname: '',
  group: '默认分组',
  tags: [] as string[]
});

/**
 * 添加方式选项的元数据
 *
 * 顺序按「推荐度 + 流派聚合」排列：
 * 1) smart 智能识别（推荐，置顶）
 * 2) Devin 系：账密 / 邮箱验证码 / session_token（新体系，日常主力）
 * 3) Firebase 系：邮箱密码 / Refresh Token（传统体系，兼容老账号）
 *
 * 每项承载卡片渲染所需的全部视觉数据（图标、标题、标签、一句话说明）。
 * 新增模式时只需在此数组里追加一条，模板网格自动同步渲染。
 */
const modeOptions = [
  {
    value: 'smart',
    title: '智能识别',
    desc: '输入邮箱密码，自动选择最佳登录流派',
    icon: MagicStick,
    tag: '推荐',
    tagType: 'primary' as const,
  },
  {
    value: 'devin',
    title: 'Devin 账密',
    desc: '用 Devin 账号密码直接登录',
    icon: User,
    tag: '新',
    tagType: 'success' as const,
  },
  {
    value: 'devin_email_code',
    title: 'Devin 邮箱验证码',
    desc: '原生（app.devin.ai）或 Windsurf 侧验证码登录/注册，进入后选择来源',
    icon: Message,
    tag: '验证码',
    tagType: 'success' as const,
  },
  {
    value: 'devin_session',
    title: 'Devin Session Token',
    desc: '粘贴 devin-session-token$... 直接迁入',
    icon: Connection,
    tag: '迁入',
    tagType: 'warning' as const,
  },
  {
    value: 'devin_auth1',
    title: 'Devin Auth1 Token',
    desc: '粘贴 auth1_... 直接迁入，支持刷新续期',
    icon: Connection,
    tag: '迁入',
    tagType: 'warning' as const,
  },
  {
    value: 'password',
    title: '邮箱密码',
    desc: '传统 Firebase 账号密码登录',
    icon: Lock,
    tag: '',
    tagType: 'info' as const,
  },
  {
    value: 'refresh_token',
    title: 'Refresh Token',
    desc: '粘贴 Firebase refresh_token 导入',
    icon: Refresh,
    tag: '',
    tagType: 'info' as const,
  },
] as const;

/**
 * Devin 邮箱验证码合并卡 —— source 子单选的卡片元数据
 *
 * 与主 modeOptions 结构对齐：icon + title + desc + tag + tagType，
 * 复用 .mode-grid / .mode-card 的现有样式，不新增 CSS。
 */
const sourceOptions = [
  {
    value: 'devin_native',
    title: 'Devin 原生',
    desc: 'app.devin.ai，账号主归属 Devin + 自动桥接 Windsurf',
    icon: Message,
    tag: '原生',
    tagType: 'success' as const,
  },
  {
    value: 'windsurf_side',
    title: 'Windsurf 侧',
    desc: 'windsurf.com，账号主归属 Windsurf',
    icon: Connection,
    tag: 'WS',
    tagType: 'info' as const,
  },
] as const;

/**
 * Devin 邮箱验证码合并卡 —— flow 子单选的卡片元数据
 *
 * 仅在 source='windsurf_side' 时渲染。
 */
const flowOptions = [
  {
    value: 'login',
    title: '登录无密码账号',
    desc: 'SSO / 忘密 / Google・GitHub 登录过的账号',
    icon: Key,
    tag: '登录',
    tagType: 'info' as const,
  },
  {
    value: 'signup',
    title: '注册新账号',
    desc: '需额外设置密码与姓名',
    icon: User,
    tag: '注册',
    tagType: 'warning' as const,
  },
] as const;

/**
 * 切换添加方式
 *
 * 卡片点击时由模板调用；内部直接写入 `addMode` 并复用原有的 `handleModeChange`
 * 清理逻辑（重置验证码 step / email_token / flow 等），保证与 el-radio 版本行为完全一致。
 */
function selectMode(value: string) {
  if (addMode.value === value) return;
  addMode.value = value as typeof addMode.value;
  handleModeChange();
}

/**
 * Devin 邮箱验证码合并卡——切换 source（原生 / Windsurf 侧）
 *
 * 切换来源时必须重置 step / token / 验证码输入，避免用户在已发过一次验证码后
 * 切换到另一端口，旧 token 被窜修改到新端口的提交逻辑而失败。
 * formData.email / nickname / tags / group 不重置，保留用户已输入内容。
 */
function selectSource(value: string) {
  const next = value as typeof devinEmailCodeSource.value;
  if (devinEmailCodeSource.value === next) return;
  devinEmailCodeSource.value = next;
  devinEmailCodeStep.value = 0;
  devinEmailCodeEmailToken.value = '';
  formData.devinEmailCodeOtp = '';
  formData.devinEmailCodePassword = '';
  formData.devinEmailCodeName = '';
}

/**
 * Devin 邮箱验证码合并卡——切换 flow（login / signup）
 *
 * 仅在 source='windsurf_side' 时有意义。切换时同样重置 step / token / 验证码输入，
 * 因为 login 和 signup 的 /email/start mode 不同，旧 token 不能复用。
 */
function selectFlow(value: string) {
  const next = value as typeof devinEmailCodeFlow.value;
  if (devinEmailCodeFlow.value === next) return;
  devinEmailCodeFlow.value = next;
  devinEmailCodeStep.value = 0;
  devinEmailCodeEmailToken.value = '';
  formData.devinEmailCodeOtp = '';
  formData.devinEmailCodePassword = '';
  formData.devinEmailCodeName = '';
}

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

// Devin 邮箱验证码模式的验证规则：按 step 分组
// step 0 只校验 email，step 1 只校验 验证码
// （避免在需要发验证码的阶段反骨用户填验证码）
const devinEmailCodeStep0Rules: FormRules = {
  email: [
    { required: true, message: '请输入邮箱', trigger: 'blur' },
    { type: 'email', message: '请输入有效的邮箱地址', trigger: 'blur' }
  ],
  nickname: [
    { max: 20, message: '备注名称最多20个字符', trigger: 'blur' }
  ]
};
const devinEmailCodeStep1Rules: FormRules = {
  devinEmailCodeOtp: [
    { required: true, message: '请输入验证码', trigger: 'blur' },
    { min: 4, message: '验证码长度不正确', trigger: 'blur' }
  ],
  nickname: [
    { max: 20, message: '备注名称最多20个字符', trigger: 'blur' }
  ]
};
// Step 1 注册子流程：验证码 + 新密码 (至少 6 位) + 姓名 (可选)
const devinEmailCodeStep1SignupRules: FormRules = {
  devinEmailCodeOtp: [
    { required: true, message: '请输入验证码', trigger: 'blur' },
    { min: 4, message: '验证码长度不正确', trigger: 'blur' }
  ],
  devinEmailCodePassword: [
    { required: true, message: '请设置新密码', trigger: 'blur' },
    { min: 6, message: '密码长度至少 6 位', trigger: 'blur' }
  ],
  devinEmailCodeName: [
    { max: 50, message: '姓名最多 50 个字符', trigger: 'blur' }
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

// Devin Auth1 Token 模式的验证规则：auth1_ 前缀 + 基本长度约束
const devinAuth1Rules: FormRules = {
  auth1Token: [
    { required: true, message: '请粘贴 Devin auth1_token', trigger: 'blur' },
    {
      validator: (_rule, value: string, callback) => {
        const trimmed = (value || '').trim();
        if (!trimmed) return callback(new Error('请粘贴 Devin auth1_token'));
        if (!trimmed.startsWith('auth1_')) {
          return callback(new Error('auth1_token 必须以 auth1_ 前缀开头'));
        }
        // 官方格式：auth1_ + 52 字符；给一定宽容度
        if (trimmed.length < 20) {
          return callback(new Error(`auth1_token 长度异常（${trimmed.length} 字符），请确认完整粘贴`));
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
  if (addMode.value === 'devin_auth1') return devinAuth1Rules;
  if (addMode.value === 'devin_email_code') {
    if (devinEmailCodeStep.value === 0) return devinEmailCodeStep0Rules;
    // Step 1 按 source + flow 分流：
    //   - source=devin_native：仅验证码（Devin 原生不需密码/姓名）
    //   - source=windsurf_side + flow=login：仅验证码
    //   - source=windsurf_side + flow=signup：验证码 + 新密码 + 姓名
    const needsSignupExtras =
      devinEmailCodeSource.value === 'windsurf_side' && devinEmailCodeFlow.value === 'signup';
    return needsSignupExtras ? devinEmailCodeStep1SignupRules : devinEmailCodeStep1Rules;
  }
  return devinRules;
});

// 切换模式时重置表单
function handleModeChange() {
  formRef.value?.resetFields();
  // Devin 邮箱验证码合并卡专属状态重置（source + step + token + flow + 表单字段）
  devinEmailCodeSource.value = 'devin_native';
  devinEmailCodeStep.value = 0;
  devinEmailCodeEmailToken.value = '';
  devinEmailCodeFlow.value = 'login';
  formData.devinEmailCodeOtp = '';
  formData.devinEmailCodePassword = '';
  formData.devinEmailCodeName = '';
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
      } else if (addMode.value === 'devin_auth1') {
        // Devin Auth1 Token 直接迁入
        await handleDevinAuth1Submit();
      } else if (addMode.value === 'devin_email_code') {
        // Devin 邮箱验证码合并卡—— 按 step + source + flow 三路分派：
        //   step 0 统一发验证码（sendDevinEmailCode 内部按 source 选端口）
        //   step 1：
        //     - source=devin_native → completeDevinEmailCodeNativeRegister
        //     - source=windsurf_side + flow=signup → completeDevinEmailCodeRegister
        //     - source=windsurf_side + flow=login → completeDevinEmailCodeLogin
        if (devinEmailCodeStep.value === 0) {
          await sendDevinEmailCode();
        } else if (devinEmailCodeSource.value === 'devin_native') {
          await completeDevinEmailCodeNativeRegister();
        } else if (devinEmailCodeFlow.value === 'signup') {
          await completeDevinEmailCodeRegister();
        } else {
          await completeDevinEmailCodeLogin();
        }
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
      // 企业 SSO 账号：有些组织仍允许邮箱验证码登录，提供快捷按钮尝试
      try {
        await ElMessageBox.confirm(
          `${sniff.reason}\n\n可以尝试用邮箱验证码登录。若邮箱仍收不到验证码，请改用「Refresh Token」模式。`,
          '企业 SSO 账号',
          {
            type: 'info',
            confirmButtonText: '用邮箱验证码登录',
            cancelButtonText: '我知道了',
          }
        );
        await switchToEmailCodeModeAndSend();
      } catch {
        // 用户取消，不做任何处理
      }
      break;
    case 'no_password':
      // 无密码账号：正是“邮箱验证码登录”的主场景
      try {
        await ElMessageBox.confirm(
          `${sniff.reason}\n\n此账号可以通过邮箱验证码登录，无需密码。是否立即发送验证码？`,
          '账号未设置密码',
          {
            type: 'warning',
            confirmButtonText: '发送验证码',
            cancelButtonText: '我知道了',
          }
        );
        await switchToEmailCodeModeAndSend();
      } catch {
        // 用户取消
      }
      break;
    case 'not_found':
      // 账号两侧都不存在：直接走“邮箱验证码注册”流程（mode=signup）
      // 不再弹 alert 要用户去别处注册，一步到位
      try {
        await ElMessageBox.confirm(
          `${sniff.reason}\n\n此邮箱尚未注册 Devin 账号。是否立即通过邮箱验证码注册新账号？需要在下一步设置密码。`,
          '账号不存在',
          {
            type: 'warning',
            confirmButtonText: '立即注册',
            cancelButtonText: '我知道了',
          }
        );
        await switchToEmailCodeModeAndSend('signup');
      } catch {
        // 用户取消
      }
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
 * Devin Auth1 Token 直接迁入流程
 *
 * 与 handleDevinSessionSubmit 对称，但通过 auth1_token 换取 session_token 并额外保留 auth1_token，
 * 后续 Devin session 到期可直接用 refreshSession 刷新，无需重新手动获取 token。
 *
 * 多组织场景：后端返回 { requires_org_selection, email, auth1_token, orgs }，
 * 前端弹 promptOrgSelection → 调 addAccountWithOrg 完成落库（复用 handleDevinSubmit 的交互）。
 */
async function handleDevinAuth1Submit() {
  const trimmedToken = formData.auth1Token.trim();
  const trimmedNickname = formData.nickname.trim() || undefined;

  if (!trimmedToken) {
    ElMessage.error('Auth1 Token 不能为空');
    return;
  }
  if (!trimmedToken.startsWith('auth1_')) {
    ElMessage.error('auth1_token 必须以 auth1_ 前缀开头');
    return;
  }

  ElMessage.info('正在用 auth1_token 换取 session 并反查账号信息……');
  const result = await devinApi.addAccountByAuth1Token({
    auth1Token: trimmedToken,
    nickname: trimmedNickname,
    tags: formData.tags,
    group: formData.group || '默认分组',
  });

  // 分支 1：多组织，需用户二次选择
  if (result.requires_org_selection && result.auth1_token && result.orgs && result.email) {
    const chosenOrg = await promptOrgSelection(result.orgs);
    if (!chosenOrg) {
      ElMessage.info('已取消多组织选择');
      return;
    }

    const confirmResult = await devinApi.addAccountWithOrg({
      email: result.email,
      auth1Token: result.auth1_token,
      orgId: chosenOrg,
      nickname: trimmedNickname,
      tags: formData.tags,
      group: formData.group || '默认分组',
    });

    if (confirmResult.success) {
      ElMessage.success(`Devin 账号 ${result.email} 已通过 auth1_token 导入成功`);
      await accountsStore.loadAccounts();
      handleClose();
    } else {
      ElMessage.error(confirmResult.message || '组织选择后创建账号失败');
    }
    return;
  }

  // 分支 2：直接成功
  if (result.success) {
    ElMessage.success(`Devin 账号 ${result.email} 已通过 auth1_token 导入成功`);
    await accountsStore.loadAccounts();
    handleClose();
  } else {
    ElMessage.error(result.message || 'Auth1 Token 迁入失败');
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

/**
 * 切换到「Devin 邮箱验证码」模式并自动发送验证码
 *
 * 供智能识别分派失败时的快捷引导：保留用户已输入的邮箱，
 * 自动切换 addMode、重置 step=0、马上发送验证码，进入 step=1等待用户输入。
 *
 * - `flow = 'login'`（默认）：登录已有无密码账号（no_password / sso 分派使用）
 * - `flow = 'signup'`：注册新账号（not_found 分派使用）
 *
 * 外层 handleSmartSubmit 已在 validate 回调内 `loading = true`，本函数无需再管理。
 */
async function switchToEmailCodeModeAndSend(flow: 'login' | 'signup' = 'login') {
  addMode.value = 'devin_email_code';
  // smart 分派进入的场景都是 Windsurf 侧（sniff_login_method 只探测 windsurf.com 侧的登录方式），
  // 合并卡默认 source='devin_native' 会走错端口，此处必须强制置为 'windsurf_side'
  devinEmailCodeSource.value = 'windsurf_side';
  devinEmailCodeFlow.value = flow;
  devinEmailCodeStep.value = 0;
  devinEmailCodeEmailToken.value = '';
  formData.devinEmailCodeOtp = '';
  formData.devinEmailCodePassword = '';
  formData.devinEmailCodeName = '';
  // formData.email 保留，不清空

  // 等模式切换后再发验证码（避免 currentRules 切换时的瑕疵触发 validate）
  await nextTick();
  await sendDevinEmailCode();
}

/**
 * Devin 邮箱验证码合并卡 —— 第 1 步：按 source 分派发验证码
 *
 * - source=devin_native：走 `app.devin.ai/api/auth1/email/start(mode=signup)`
 *   账号主归属 Devin；只有 signup，不用分 login/signup
 * - source=windsurf_side：走 `windsurf.com/_devin-auth/email/start`。mode 按 flow 切：
 *     * login —— 仅对已存在账号有效，服务端不会创建新账号
 *     * signup —— 服务端向邮箱发送注册验证码，后续 `/email/complete` 时创建新账号
 *
 * 成功后更新 step=1，进入验证码输入屏。
 */
async function sendDevinEmailCode() {
  const trimmedEmail = formData.email.trim();
  if (!trimmedEmail) {
    ElMessage.error('邮箱不能为空');
    return;
  }

  // 分支 1：Devin 原生—— app.devin.ai 端口，仅 signup
  if (devinEmailCodeSource.value === 'devin_native') {
    try {
      const resp = await devinApi.nativeEmailStart(trimmedEmail, 'signup');
      if (!resp || !resp.email_verification_token) {
        ElMessage.error('后端未返回 email_verification_token，无法继续');
        return;
      }
      devinEmailCodeEmailToken.value = resp.email_verification_token;
      devinEmailCodeStep.value = 1;
      ElMessage.success(`Devin 原生注册验证码已发送至 ${trimmedEmail}`);
    } catch (e: any) {
      const errMsg = String(e?.message || e || '');
      ElMessage.error(`发送验证码失败：${errMsg}`);
    }
    return;
  }

  // 分支 2：Windsurf 侧—— windsurf.com/_devin-auth 端口，mode 按 flow 切
  const mode = devinEmailCodeFlow.value === 'signup' ? 'signup' : 'login';
  try {
    const resp = await devinApi.emailStart(trimmedEmail, mode, 'Windsurf');
    if (!resp || !resp.email_verification_token) {
      ElMessage.error('后端未返回 email_verification_token，无法继续');
      return;
    }
    devinEmailCodeEmailToken.value = resp.email_verification_token;
    devinEmailCodeStep.value = 1;
    const hint = mode === 'signup' ? '注册验证码已发送至' : '验证码已发送至';
    ElMessage.success(`${hint} ${trimmedEmail}`);
  } catch (e: any) {
    const errMsg = String(e?.message || e || '');
    // login flow 遇到服务端“账号不存在”判定时，引导用户改为 signup flow 并自动重试
    // 覆盖三种场景：
    // 1) radio 主入口直选 devin_email_code 但输入了未注册邮箱
    // 2) sniff_login_method 给出的 no_password / sso 判定与 /email/start 不一致
    // 3) 账号刚被删除/迁移，CheckUserLoginMethod 仍有缓存但 /email/start 已同步
    if (mode === 'login' && /no account found/i.test(errMsg)) {
      try {
        await ElMessageBox.confirm(
          `服务端判定此邮箱尚未注册 Devin 账号：\n${errMsg}\n\n是否改为「邮箱验证码注册」创建新账号？下一步需要设置密码。`,
          '账号不存在',
          {
            type: 'warning',
            confirmButtonText: '改为注册',
            cancelButtonText: '我知道了',
          }
        );
        // 切 flow 后递归一次；signup mode 不会再返回 No account found，不会无限循环
        devinEmailCodeFlow.value = 'signup';
        await sendDevinEmailCode();
      } catch {
        // 用户取消：保持在 step=0，提示原始错误以便用户修正邮箱或切换模式
        ElMessage.info('已取消。请确认邮箱是否正确，或改用其它添加方式。');
      }
      return;
    }
    ElMessage.error(`发送验证码失败：${errMsg}`);
  }
}

/**
 * Devin 邮箱验证码登录—— 第 2 步：提交验证码，完成登录并建账号
 *
 * - 未设密码的 Devin 账号要走「/email/complete mode=login」，后端命令为
 *   `add_account_by_devin_email_login`（内部自动完成 WindsurfPostAuth + enrich）
 * - 多组织场景复用 `promptOrgSelection` + `addAccountWithOrg`（与 handleDevinSubmit 同步一致）
 */
async function completeDevinEmailCodeLogin() {
  const trimmedEmail = formData.email.trim();
  const otp = formData.devinEmailCodeOtp.trim();
  const trimmedNickname = formData.nickname.trim() || undefined;

  if (!otp) {
    ElMessage.error('请输入验证码');
    return;
  }
  if (!devinEmailCodeEmailToken.value) {
    ElMessage.error('会话状态异常，请返回上一步重新发送验证码');
    return;
  }

  const result = await devinApi.addAccountByEmailLogin({
    email: trimmedEmail,
    emailVerificationToken: devinEmailCodeEmailToken.value,
    code: otp,
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
    ElMessage.success(`Devin 账号 ${result.email || trimmedEmail} 添加成功`);
    await accountsStore.loadAccounts();
    handleClose();
  } else {
    ElMessage.error(result.message || '邮箱验证码登录失败');
  }
}

/**
 * Devin 邮箱验证码注册—— 第 2 步：提交验证码 + 新密码 + 姓名，完成注册并建账号
 *
 * - 调用后端 `add_account_by_devin_register`（内部自动完成注册 + WindsurfPostAuth + enrich）
 * - 多组织场景复用 `promptOrgSelection` + `addAccountWithOrg`（注册流程的原始密码会随二次写入账号卡的 password 字段）
 */
async function completeDevinEmailCodeRegister() {
  const trimmedEmail = formData.email.trim();
  const otp = formData.devinEmailCodeOtp.trim();
  const newPassword = formData.devinEmailCodePassword.trim();
  const displayName =
    formData.devinEmailCodeName.trim() || trimmedEmail.split('@')[0] || 'Devin User';
  const trimmedNickname = formData.nickname.trim() || undefined;

  if (!otp) {
    ElMessage.error('请输入验证码');
    return;
  }
  if (!newPassword) {
    ElMessage.error('请设置新密码');
    return;
  }
  if (!devinEmailCodeEmailToken.value) {
    ElMessage.error('会话状态异常，请返回上一步重新发送验证码');
    return;
  }

  const result = await devinApi.addAccountByRegister({
    email: trimmedEmail,
    emailVerificationToken: devinEmailCodeEmailToken.value,
    code: otp,
    password: newPassword,
    name: displayName,
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

    // 注册流程将原始密码随二次选组织入库，便于账号卡回显密码
    const confirmResult = await devinApi.addAccountWithOrg({
      email: trimmedEmail,
      auth1Token: result.auth1_token,
      orgId: chosenOrg,
      nickname: trimmedNickname,
      tags: formData.tags,
      group: formData.group || '默认分组',
      password: newPassword,
    });

    if (confirmResult.success) {
      ElMessage.success(`Devin 账号 ${trimmedEmail} 注册成功`);
      await accountsStore.loadAccounts();
      handleClose();
    } else {
      ElMessage.error(confirmResult.message || '组织选择后注册账号失败');
    }
    return;
  }

  // 分支 2：直接注册成功
  if (result.success) {
    ElMessage.success(`Devin 账号 ${result.email || trimmedEmail} 注册成功`);
    await accountsStore.loadAccounts();
    handleClose();
  } else {
    ElMessage.error(result.message || '邮箱验证码注册失败');
  }
}

/**
 * Devin 邮箱验证码合并卡（source=devin_native）—— 第 2 步：提交验证码完成注册 + 桥接 + 落库
 *
 * 调用后端 `add_account_by_devin_native_register`：
 * 1. `devin_app_email_complete(token, code, mode="signup")` → 得 auth1_token
 * 2. `WindsurfPostAuth(auth1_token, org_id)` → 得 session_token
 * 3. 落库为新账号（password 字段留空，因为 Devin 原生不收集密码）
 *
 * 复用 `devinEmailCodeEmailToken` / `formData.devinEmailCodeOtp` 统一状态，不再维护独立的 devinNative* 字段。
 * 多组织场景复用 `promptOrgSelection` + `addAccountWithOrg`（与其他 Devin 注册流程一致）。
 */
async function completeDevinEmailCodeNativeRegister() {
  const trimmedEmail = formData.email.trim();
  const otp = formData.devinEmailCodeOtp.trim();
  const trimmedNickname = formData.nickname.trim() || undefined;

  if (!otp) {
    ElMessage.error('请输入验证码');
    return;
  }
  if (!devinEmailCodeEmailToken.value) {
    ElMessage.error('会话状态异常，请返回上一步重新发送验证码');
    return;
  }

  const result = await devinApi.addAccountByNativeRegister({
    email: trimmedEmail,
    emailVerificationToken: devinEmailCodeEmailToken.value,
    code: otp,
    nickname: trimmedNickname,
    tags: formData.tags,
    group: formData.group || '默认分组',
  });

  // 分支 1：多组织，需用户二次选择
  if (result.requires_org_selection && result.auth1_token && result.orgs) {
    const chosenOrg = await promptOrgSelection(result.orgs);
    if (!chosenOrg) {
      ElMessage.info('已取消多组织选择');
      return;
    }

    // Devin 原生注册场景 password 始终留空（服务端不支持预设密码）
    const confirmResult = await devinApi.addAccountWithOrg({
      email: trimmedEmail,
      auth1Token: result.auth1_token,
      orgId: chosenOrg,
      nickname: trimmedNickname,
      tags: formData.tags,
      group: formData.group || '默认分组',
    });

    if (confirmResult.success) {
      ElMessage.success(`Devin 原生账号 ${trimmedEmail} 注册成功（已桥接 Windsurf）`);
      await accountsStore.loadAccounts();
      handleClose();
    } else {
      ElMessage.error(confirmResult.message || '组织选择后注册账号失败');
    }
    return;
  }

  // 分支 2：直接注册成功
  if (result.success) {
    ElMessage.success(`Devin 原生账号 ${result.email || trimmedEmail} 注册成功（已桥接 Windsurf）`);
    await accountsStore.loadAccounts();
    handleClose();
  } else {
    ElMessage.error(result.message || 'Devin 原生注册失败');
  }
}

function handleClose() {
  uiStore.closeAddAccountDialog();
  formRef.value?.resetFields();
  
  // 重置表单数据
  formData.email = '';
  formData.password = '';
  formData.refreshToken = '';
  formData.sessionToken = '';
  formData.auth1Token = '';
  formData.devinEmailCodeOtp = '';
  formData.devinEmailCodePassword = '';
  formData.devinEmailCodeName = '';
  formData.nickname = '';
  formData.group = '默认分组';
  formData.tags = [];
  addMode.value = 'smart';
  // Devin 邮箱验证码合并卡状态重置
  devinEmailCodeSource.value = 'devin_native';
  devinEmailCodeStep.value = 0;
  devinEmailCodeEmailToken.value = '';
  devinEmailCodeFlow.value = 'login';
}
</script>

<style scoped>
/* ==================== 添加方式卡片网格（紧凑版） ====================
 * 单行布局：icon + title(flex 1 可省略) + tag(可选) + check(仅选中时)
 * 说明文本仅以原生 tooltip 呈现（见模板 `:title="opt.desc"`），
 * 不占用纵向空间。
 */

/* 外层 2 列网格，窄屏自动降为单列 */
.mode-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
  width: 100%;
}

/* 单张卡片：单行 flex，矮版 34px */
.mode-card {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border: 1.5px solid var(--el-border-color);
  border-radius: 6px;
  background-color: var(--el-bg-color);
  cursor: pointer;
  transition: border-color 0.2s ease, background-color 0.2s ease, box-shadow 0.2s ease;
  user-select: none;
  outline: none;
  min-height: 34px;
}

/* 鼠标悬停：浅主色边框 + 极浅背景 */
.mode-card:hover {
  border-color: var(--el-color-primary-light-3);
  background-color: var(--el-color-primary-light-9);
}

/* 键盘 focus 态 */
.mode-card:focus-visible {
  box-shadow: 0 0 0 2px var(--el-color-primary-light-5);
}

/* 选中态：主色边框 + 浅主色背景 + 外环 */
.mode-card.is-active {
  border-color: var(--el-color-primary);
  background-color: var(--el-color-primary-light-9);
  box-shadow: 0 0 0 2px var(--el-color-primary-light-7);
}

/* 图标：紧凑版与文字基线对齐 */
.mode-card__icon {
  flex-shrink: 0;
  font-size: 18px;
  color: var(--el-color-primary);
  width: 18px;
  height: 18px;
}

/* 标题：占用剩余空间单行省略；字号 13 避免在 2 列 ~220px 下频繁省略 */
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

/* 标签：不收缩，跟在标题后 */
.mode-card__tag {
  flex-shrink: 0;
}

/* 选中勾选：内联放在最右，与 tag 并列；不再用 absolute 避免在紧凑高度下压到文字 */
.mode-card__check {
  flex-shrink: 0;
  font-size: 14px;
  color: var(--el-color-primary);
}

/* 窄屏降级：小窗下单列，避免标题 + 标签撑穷卡片 */
@media (max-width: 520px) {
  .mode-grid {
    grid-template-columns: 1fr;
  }
}
</style>

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { Account, AccountFilter, PaginationConfig, SortConfig, SortField, SortDirection } from '@/types';
import { accountApi, apiService, settingsApi } from '@/api';
import type { AccountPageRequest, AccountAggregates } from '@/api';
// settingsApi 包含 getGroups 方法
import dayjs from 'dayjs';

export const useAccountsStore = defineStore('accounts', () => {
  const accounts = ref<Account[]>([]);
  const groups = ref<string[]>([]);  // 独立存储的分组列表
  const globalTags = ref<{name: string}[]>([]);  // 独立存储的标签列表
  const selectedAccounts = ref<Set<string>>(new Set());
  const currentFilter = ref<AccountFilter>({});
  const loading = ref(false);
  const error = ref<string | null>(null);
  
  // 批量更新队列（用于优化大量账号更新时的性能）
  const pendingUpdates = ref<Map<string, Account>>(new Map());
  let batchUpdateTimer: ReturnType<typeof setTimeout> | null = null;

  // 分页状态
  const pagination = ref<PaginationConfig>({
    currentPage: 1,
    pageSize: 20,
    pageSizes: [10, 20, 50, 100]
  });

  // 排序配置
  const sortConfig = ref<SortConfig>({
    field: 'created_at',
    direction: 'asc'
  });

  // **v1.7.8 方案 B**：后端聚合统计（替代原 6 个 computed 各遍历 10 万对象）
  const aggregates = ref<AccountAggregates>({
    total_count: 0,
    groups: [],
    plan_names: [],
    domains: [],
    tags: [],
    active_count: 0,
    group_counts: {},
    tag_counts: {},
  });
  // 后端分页查询返回的总数（过滤后）
  const serverTotal = ref(0);

  // 辅助函数：计算剩余额度
  function getRemainingQuota(account: Account): number {
    if (!account.total_quota || account.used_quota === undefined) return 0;
    return Math.max(0, account.total_quota - account.used_quota);
  }

  // 辅助函数：计算剩余天数
  function getDaysUntilExpiry(account: Account): number | null {
    if (!account.subscription_expires_at) return null;
    const now = dayjs();
    const expiry = dayjs(account.subscription_expires_at);
    return expiry.diff(now, 'day');
  }

  // v1.7.8 方案 B：isPaidPlan / getAccountStatusType 已移除（状态过滤下沉后端
  // SqliteAccountStore::STATUS_CASE_EXPR SQL CASE 表达式，优先级链与原前端逻辑一致）。

  // **v1.7.8 方案 B**：下拉框数据源从后端聚合查询获取，替代原 6 个 computed 各遍历 10 万对象。
  // 合并独立存储的分组/标签 + 后端 SQLite 聚合结果，保持与原接口兼容。
  const allTags = computed(() => {
    const tagSet = new Set<string>();
    globalTags.value.forEach(t => tagSet.add(t.name));
    aggregates.value.tags.forEach(t => tagSet.add(t));
    return Array.from(tagSet).sort();
  });

  const allPlanNames = computed(() => aggregates.value.plan_names);

  const allDomains = computed(() => aggregates.value.domains);

  const allGroups = computed(() => {
    const groupSet = new Set<string>();
    groups.value.forEach(g => groupSet.add(g));
    aggregates.value.groups.forEach(g => groupSet.add(g));
    return Array.from(groupSet).sort();
  });

  // **v1.7.8 方案 B**：后端已做分页/过滤/排序，前端不再遍历全量。
  // filteredAccounts / paginatedAccounts 直接返回当前页数据（后端已过滤排序）。
  const filteredAccounts = computed(() => accounts.value);
  const paginatedAccounts = computed(() => accounts.value);

  const totalPages = computed(() => {
    return Math.ceil(serverTotal.value / pagination.value.pageSize);
  });

  const totalCount = computed(() => serverTotal.value);

  const selectedAccountsList = computed(() => {
    // 选中状态是跨页的，但只能返回当前页可见的选中账号
    return accounts.value.filter(acc => selectedAccounts.value.has(acc.id));
  });

  const activeAccountsCount = computed(() => aggregates.value.active_count);

  // Actions
  async function loadGroups() {
    try {
      groups.value = await settingsApi.getGroups();
    } catch (e) {
      console.error('加载分组失败:', e);
    }
  }

  async function loadTags() {
    try {
      globalTags.value = await settingsApi.getTags();
    } catch (e) {
      console.error('加载标签失败:', e);
    }
  }

  /**
   * **v1.7.8 方案 B 核心方法**：从后端分页查询获取当前页数据。
   *
   * 将 currentFilter + pagination + sortConfig 组装为 AccountPageRequest，
   * 调用后端 `get_accounts_page`，仅拉取 20-100 条。IPC 传输从 300MB 降到 <100KB。
   */
  async function fetchPage() {
    loading.value = true;
    error.value = null;
    try {
      const f = currentFilter.value;
      const request: AccountPageRequest = {
        page: pagination.value.currentPage,
        page_size: pagination.value.pageSize,
        search: f.search || undefined,
        group: f.group || undefined,
        tags: f.tags && f.tags.length > 0 ? f.tags : undefined,
        plan_names: f.planNames && f.planNames.length > 0 ? f.planNames : undefined,
        domains: f.domains && f.domains.length > 0 ? f.domains : undefined,
        statuses: f.statuses && f.statuses.length > 0 ? f.statuses : undefined,
        remaining_quota_min: f.remainingQuotaMin,
        remaining_quota_max: f.remainingQuotaMax,
        total_quota_min: f.totalQuotaMin,
        total_quota_max: f.totalQuotaMax,
        expiry_days_min: f.expiryDaysMin,
        expiry_days_max: f.expiryDaysMax,
        daily_quota_percent_min: f.dailyQuotaPercentMin,
        daily_quota_percent_max: f.dailyQuotaPercentMax,
        weekly_quota_percent_min: f.weeklyQuotaPercentMin,
        weekly_quota_percent_max: f.weeklyQuotaPercentMax,
        sort_field: sortConfig.value.field,
        sort_direction: sortConfig.value.direction,
      };

      const response = await accountApi.getAccountsPage(request);
      accounts.value = response.accounts;
      serverTotal.value = response.total;
    } catch (e) {
      error.value = (e as Error).message;
      throw e;
    } finally {
      loading.value = false;
    }
  }

  /** 刷新后端聚合统计（分组/套餐/域名/标签列表 + 总数/活跃数） */
  async function refreshAggregates() {
    try {
      aggregates.value = await accountApi.getAccountAggregates();
    } catch (e) {
      console.error('刷新聚合统计失败:', e);
    }
  }

  /**
   * 加载账号列表（v1.7.8 方案 B：改为分页查询 + 聚合统计）。
   *
   * 同时加载：当前页数据 + 分组 + 标签 + 聚合统计。
   * 替代原全量 `get_all_accounts`（10 万级 300MB IPC），每次只拿 20-100 条。
   */
  async function loadAccounts() {
    loading.value = true;
    error.value = null;
    try {
      await Promise.all([
        fetchPage(),
        loadGroups(),
        loadTags(),
        refreshAggregates(),
      ]);
    } catch (e) {
      error.value = (e as Error).message;
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function addAccount(data: {
    email: string;
    password: string;
    nickname: string;
    tags: string[];
    group?: string;
  }) {
    error.value = null;
    try {
      const account = await accountApi.addAccount(data);
      // 静默添加：只刷新聚合统计（total_count +1 → totalPages 自动更新），不 push 到当前页
      refreshAggregates().catch(() => {});
      return account;
    } catch (e) {
      error.value = (e as Error).message;
      throw e;
    }
  }

  /**
   * 本地追加一个已落库的账号到 store 快照（不走后端，不触发 loading）。
   *
   * 使用场景：任务队列注册成功后，`finalize_devin_task_account` 后端命令已完成落库
   * 并返回完整账号对象，前端只需把该账号增量 push 进 `accounts` 数组。
   *
   * 相比 `loadAccounts()`：
   * - 不触发全局 `loading` 状态 → 无 loading 遮罩 / 骨架屏闪烁
   * - 不全量替换数组 → Vue 仅为新增账号卡挂载，其他账号卡不 re-render
   * - 不发起额外的 HTTP 请求（零 RTT）
   *
   * 相比 `addAccount()`：
   * - `addAccount` 内部调 `accountApi.addAccount`（会触发后端 INSERT），本函数不调任何后端
   * - 适用于后端已经完成落库、前端只需同步 store 快照的场景
   *
   * 幂等性：按 `id` 去重。若传入账号的 id 已存在于 `accounts` 数组，则替换该位置（避免重复）；
   * 否则 push 到数组末尾。
   */
  function appendLocalAccount(account: Account) {
    // 已存在则原地替换（刷新场景），不存在则只刷新聚合统计
    const idx = accounts.value.findIndex(a => a.id === account.id);
    if (idx !== -1) {
      accounts.value.splice(idx, 1, account);
    }
    refreshAggregates().catch(() => {});
  }

  async function updateAccount(account: Account) {
    // 单个账号更新不触发全局loading，避免页面闪烁
    error.value = null;
    try {
      await accountApi.updateAccount(account);
      const index = accounts.value.findIndex(a => a.id === account.id);
      if (index !== -1) {
        // 使用splice确保触发响应式更新
        accounts.value.splice(index, 1, account);
      }
    } catch (e) {
      error.value = (e as Error).message;
      throw e;
    }
  }

  /**
   * 本地 patch 多个账号的字段（不走后端，仅同步 store 快照）。
   *
   * 使用场景：批量操作已通过专用后端 API（如 `batch_update_account_tags`、
   * `batch_update_account_group`）成功落库，本地只需把 `accounts` 数组中受影响账号的
   * 对应字段同步更新。
   *
   * 相比 `loadAccounts()`：
   * - 不触发全局 `loading` 状态 → 无 loading 遮罩 / 骨架屏闪烁
   * - 不全量替换数组 → Vue 只 re-render 字段真正变化的账号卡
   * - 不发起额外的 HTTP 请求
   *
   * @param patches 每项必须包含 `id`；可选字段：
   *   - `tagsAdd` / `tagsRemove`：对 `tags` 数组做增/删（自动去重）
   *   - `group`：目标分组，传 `null` 表示清空分组
   *   - `plan_name`：目标套餐名称
   */
  function patchLocalAccounts(
    patches: Array<{
      id: string;
      tagsAdd?: string[];
      tagsRemove?: string[];
      group?: string | null;
      plan_name?: string;
    }>,
  ) {
    if (!patches || patches.length === 0) return;

    const byId = new Map(patches.map((p) => [p.id, p]));

    // 一次性 map 整个数组只触发一次响应式更新；未命中的账号对象引用保持不变，
    // Vue 对应的 AccountCard 不会 re-render。
    accounts.value = accounts.value.map((acc) => {
      const patch = byId.get(acc.id);
      if (!patch) return acc;

      // tags
      let nextTags = acc.tags;
      const hasTagOp = (patch.tagsAdd && patch.tagsAdd.length > 0)
        || (patch.tagsRemove && patch.tagsRemove.length > 0);
      if (hasTagOp) {
        const tagSet = new Set(acc.tags ?? []);
        patch.tagsAdd?.forEach((t) => tagSet.add(t));
        patch.tagsRemove?.forEach((t) => tagSet.delete(t));
        nextTags = Array.from(tagSet);
      }

      // group：undefined 表示不改，null 表示清空
      const nextGroup = patch.group === undefined
        ? acc.group
        : (patch.group === null ? undefined : patch.group);

      // plan_name：undefined 表示不改
      const nextPlan = patch.plan_name === undefined ? acc.plan_name : patch.plan_name;

      return {
        ...acc,
        tags: nextTags,
        group: nextGroup,
        plan_name: nextPlan,
      };
    });
  }

  /**
   * 将账号加入批量更新队列（不立即触发UI更新）
   * 用于大量账号刷新时的性能优化
   */
  function queueAccountUpdate(account: Account) {
    pendingUpdates.value.set(account.id, account);
    
    // 使用防抖，300ms内的更新合并为一次
    if (batchUpdateTimer) {
      clearTimeout(batchUpdateTimer);
    }
    batchUpdateTimer = setTimeout(() => {
      flushPendingUpdates();
    }, 300);
  }

  /**
   * 立即应用所有待更新的账号（一次性更新UI）
   */
  async function flushPendingUpdates() {
    if (pendingUpdates.value.size === 0) return;
    
    const updates = Array.from(pendingUpdates.value.values());
    console.log(`[批量更新] 一次性更新 ${updates.length} 个账号到UI`);
    
    // 清空队列
    pendingUpdates.value.clear();
    if (batchUpdateTimer) {
      clearTimeout(batchUpdateTimer);
      batchUpdateTimer = null;
    }
    
    // 构建ID到更新数据的映射
    const updateMap = new Map(updates.map(acc => [acc.id, acc]));
    
    // 一次性更新所有账号（只触发一次响应式更新）
    accounts.value = accounts.value.map(acc => {
      const updated = updateMap.get(acc.id);
      return updated || acc;
    });
    
    // 批量保存到后端（使用 Promise.all 但不等待）
    // 这里先更新UI，后台异步保存
    Promise.all(updates.map(acc => accountApi.updateAccount(acc).catch(e => {
      console.error(`[批量更新] 保存账号 ${acc.email} 失败:`, e);
    })));
  }

  async function deleteAccount(id: string) {
    loading.value = true;
    error.value = null;
    try {
      await accountApi.deleteAccount(id);
      selectedAccounts.value.delete(id);
      // v1.7.8 方案 B：删除后刷新当前页 + 聚合统计
      await Promise.all([fetchPage(), refreshAggregates()]);
    } catch (e) {
      error.value = (e as Error).message;
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function deleteSelectedAccounts() {
    if (selectedAccounts.value.size === 0) return;
    
    loading.value = true;
    error.value = null;
    try {
      const ids = Array.from(selectedAccounts.value);
      const result = await accountApi.deleteAccountsBatch(ids);
      selectedAccounts.value.clear();
      // v1.7.8 方案 B：批量删除后刷新当前页 + 聚合统计
      await Promise.all([fetchPage(), refreshAggregates()]);
      return result;
    } catch (e) {
      error.value = (e as Error).message;
      throw e;
    } finally {
      loading.value = false;
    }
  }

  function toggleAccountSelection(id: string) {
    if (selectedAccounts.value.has(id)) {
      selectedAccounts.value.delete(id);
    } else {
      selectedAccounts.value.add(id);
    }
  }

  /**
   * 全选账号（v1.7.8：在分组视图中只选当前分组，无分组时选全部）。
   * 从后端获取 ID 列表，不限于当前页。
   */
  async function selectAll() {
    try {
      const group = currentFilter.value.group || undefined;
      const allIds = await accountApi.getAllAccountIds(group);
      allIds.forEach(id => selectedAccounts.value.add(id));
    } catch {
      filteredAccounts.value.forEach(acc => selectedAccounts.value.add(acc.id));
    }
  }

  function clearSelection() {
    selectedAccounts.value.clear();
  }

  function setFilter(filter: AccountFilter) {
    currentFilter.value = filter;
    pagination.value.currentPage = 1;
    // v1.7.8 方案 B：过滤变化触发后端重新查询
    fetchPage().catch(() => {});
  }

  function clearFilter() {
    currentFilter.value = {};
    pagination.value.currentPage = 1;
    // v1.7.8 方案 B：过滤清除触发后端重新查询
    fetchPage().catch(() => {});
  }

  // 分页操作
  function setCurrentPage(page: number) {
    pagination.value.currentPage = page;
    // v1.7.8 方案 B：翻页触发后端查询
    fetchPage().catch(() => {});
  }

  function setPageSize(size: number) {
    pagination.value.pageSize = size;
    pagination.value.currentPage = 1;
    // v1.7.8 方案 B：页大小变化触发后端查询
    fetchPage().catch(() => {});
  }

  // 自动刷新Token功能
  const autoRefreshTimerId = ref<number | null>(null);
  const refreshingAccounts = ref<Set<string>>(new Set()); // 跟踪正在刷新的账号

  /**
   * 检查Token是否已过期或即将过期（5分钟内）
   */
  function isTokenExpiredOrExpiring(account: Account): boolean {
    if (!account.token_expires_at) return true;
    
    const expiresAt = dayjs(account.token_expires_at);
    const now = dayjs();
    const fiveMinutesLater = now.add(5, 'minute');
    
    // Token已过期或将5分钟内过期
    return expiresAt.isBefore(fiveMinutesLater);
  }

  /**
   * 获取需要刷新Token的账号列表
   */
  function getAccountsNeedingRefresh(): Account[] {
    return accounts.value.filter(account => {
      // 跳过状态为 inactive 或 error 的账号
      if (account.status === 'inactive' || account.status === 'error') {
        return false;
      }
      
      // 跳过正在刷新的账号
      if (refreshingAccounts.value.has(account.id)) {
        return false;
      }
      
      // 检查Token是否需要刷新
      return isTokenExpiredOrExpiring(account);
    });
  }

  /**
   * 刷新单个账号的Token
   * @param useBatchUpdate 是否使用批量更新（大量刷新时设为true提升性能）
   */
  async function refreshAccountToken(account: Account, useBatchUpdate: boolean = false): Promise<{ success: boolean; error?: string }> {
    // 标记为正在刷新
    refreshingAccounts.value.add(account.id);
    
    try {
      const result = await apiService.refreshToken(account.id);
      
      if (result.success) {
        // 更新账号信息
        const updatedAccount = { ...account, status: 'active' as const };
        
        // 更新新的 token
        if (result.token) {
          updatedAccount.token = result.token;
        }
        if (result.expires_at) {
          updatedAccount.token_expires_at = result.expires_at;
        }
        if (result.plan_name) {
          updatedAccount.plan_name = result.plan_name;
        }
        if (result.used_quota !== undefined) {
          updatedAccount.used_quota = result.used_quota;
        }
        if (result.total_quota !== undefined) {
          updatedAccount.total_quota = result.total_quota;
        }
        // 只有大于0才更新，避免1970年问题
        if (result.subscription_expires_at && typeof result.subscription_expires_at === 'number' && result.subscription_expires_at > 0) {
          updatedAccount.subscription_expires_at = dayjs.unix(result.subscription_expires_at).toISOString();
        }
        // 更新账户禁用状态
        if (result.is_disabled !== undefined) {
          updatedAccount.is_disabled = result.is_disabled;
        }
        // 更新团队所有者状态
        if (result.is_team_owner !== undefined) {
          updatedAccount.is_team_owner = result.is_team_owner;
        }
        // 更新配额百分比字段
        if (result.billing_strategy !== undefined) {
          updatedAccount.billing_strategy = result.billing_strategy;
        }
        if (result.daily_quota_remaining_percent !== undefined) {
          updatedAccount.daily_quota_remaining_percent = result.daily_quota_remaining_percent;
        }
        if (result.weekly_quota_remaining_percent !== undefined) {
          updatedAccount.weekly_quota_remaining_percent = result.weekly_quota_remaining_percent;
        }
        if (result.daily_quota_reset_at_unix !== undefined) {
          updatedAccount.daily_quota_reset_at_unix = result.daily_quota_reset_at_unix;
        }
        if (result.weekly_quota_reset_at_unix !== undefined) {
          updatedAccount.weekly_quota_reset_at_unix = result.weekly_quota_reset_at_unix;
        }
        if (result.overage_balance_micros !== undefined) {
          updatedAccount.overage_balance_micros = result.overage_balance_micros;
        }
        updatedAccount.last_quota_update = dayjs().toISOString();
        
        // 根据模式选择更新方式
        if (useBatchUpdate) {
          // 批量模式：加入队列，稍后一次性更新UI
          queueAccountUpdate(updatedAccount);
        } else {
          // 单个模式：立即更新
          await updateAccount(updatedAccount);
        }
        
        console.log(`[自动刷新] ${account.email} Token刷新成功`);
        return { success: true };
      } else {
        // 刷新失败，更新账号状态为error
        const updatedAccount = { ...account, status: 'error' as const };
        if (useBatchUpdate) {
          queueAccountUpdate(updatedAccount);
        } else {
          await updateAccount(updatedAccount);
        }
        
        console.error(`[自动刷新] ${account.email} Token刷新失败`);
        return { success: false, error: 'Token刷新失败' };
      }
    } catch (error) {
      // 刷新失败，更新账号状态为error
      const updatedAccount = { ...account, status: 'error' as const };
      if (useBatchUpdate) {
        queueAccountUpdate(updatedAccount);
      } else {
        await updateAccount(updatedAccount);
      }
      
      console.error(`[自动刷新] ${account.email} Token刷新异常:`, error);
      return { success: false, error: String(error) };
    } finally {
      // 移除正在刷新标记
      refreshingAccounts.value.delete(account.id);
    }
  }

  /**
   * 批量刷新Token（使用优化的批量 API，后端只保存一次）
   */
  async function batchRefreshTokens(accountsToRefresh?: Account[], _concurrentLimit: number = 3): Promise<{
    total: number;
    success: number;
    failed: number;
    results: Array<{ id: string; email: string; success: boolean; error?: string }>;
  }> {
    const targetAccounts = accountsToRefresh || getAccountsNeedingRefresh();
    
    if (targetAccounts.length === 0) {
      return { total: 0, success: 0, failed: 0, results: [] };
    }
    
    console.log(`[自动刷新] 开始批量刷新 ${targetAccounts.length} 个账号的Token（使用优化API）`);
    
    // 标记所有账号为正在刷新
    targetAccounts.forEach(a => refreshingAccounts.value.add(a.id));
    
    try {
      // 使用优化的批量刷新 API（后端只保存一次）
      const ids = targetAccounts.map(a => a.id);
      const apiResult = await apiService.batchRefreshTokens(ids);
      
      const results: Array<{ id: string; email: string; success: boolean; error?: string }> = [];
      
      // 处理结果，直接用返回的数据更新本地 store
      if (apiResult.results) {
        for (const item of apiResult.results) {
          const idx = accounts.value.findIndex(a => a.id === item.id);
          if (idx === -1) continue;
          
          const account = targetAccounts.find(a => a.id === item.id);
          if (!account) continue;
          
          if (item.success && item.data) {
            // 使用后端返回的完整数据更新本地 store
            // 使用 splice 替换整个对象以确保触发 Vue 响应式更新
            const updatedAcc = { ...accounts.value[idx] };
            if (item.data.plan_name) updatedAcc.plan_name = item.data.plan_name;
            if (item.data.used_quota !== undefined) updatedAcc.used_quota = item.data.used_quota;
            if (item.data.total_quota !== undefined) updatedAcc.total_quota = item.data.total_quota;
            if (item.data.expires_at) updatedAcc.token_expires_at = item.data.expires_at;
            if (item.data.windsurf_api_key) updatedAcc.windsurf_api_key = item.data.windsurf_api_key;
            if (item.data.is_disabled !== undefined) updatedAcc.is_disabled = item.data.is_disabled;
            if (item.data.is_team_owner !== undefined) updatedAcc.is_team_owner = item.data.is_team_owner;
            if (item.data.subscription_active !== undefined) updatedAcc.subscription_active = item.data.subscription_active;
            if (item.data.subscription_expires_at && typeof item.data.subscription_expires_at === 'number' && item.data.subscription_expires_at > 0) {
              updatedAcc.subscription_expires_at = dayjs.unix(item.data.subscription_expires_at).toISOString();
            }
            if (item.data.last_quota_update) updatedAcc.last_quota_update = item.data.last_quota_update;
            if (item.data.billing_strategy !== undefined) updatedAcc.billing_strategy = item.data.billing_strategy;
            if (item.data.daily_quota_remaining_percent !== undefined) updatedAcc.daily_quota_remaining_percent = item.data.daily_quota_remaining_percent;
            if (item.data.weekly_quota_remaining_percent !== undefined) updatedAcc.weekly_quota_remaining_percent = item.data.weekly_quota_remaining_percent;
            if (item.data.daily_quota_reset_at_unix !== undefined) updatedAcc.daily_quota_reset_at_unix = item.data.daily_quota_reset_at_unix;
            if (item.data.weekly_quota_reset_at_unix !== undefined) updatedAcc.weekly_quota_reset_at_unix = item.data.weekly_quota_reset_at_unix;
            if (item.data.overage_balance_micros !== undefined) updatedAcc.overage_balance_micros = item.data.overage_balance_micros;
            updatedAcc.status = 'active';
            accounts.value.splice(idx, 1, updatedAcc);
            
            results.push({ id: account.id, email: account.email, success: true });
          } else {
            // 刷新失败，使用 splice 确保响应式更新
            const failedAcc = { ...accounts.value[idx], status: 'error' as const };
            accounts.value.splice(idx, 1, failedAcc);
            results.push({ id: account.id, email: account.email, success: false, error: item.error });
          }
        }
      }
      
      const successCount = results.filter(r => r.success).length;
      const failedCount = results.filter(r => !r.success).length;
      
      console.log(`[自动刷新] 批量刷新完成: 成功 ${successCount}/${targetAccounts.length}, 失败 ${failedCount}`);
      
      return {
        total: targetAccounts.length,
        success: successCount,
        failed: failedCount,
        results
      };
    } finally {
      // 移除正在刷新标记
      targetAccounts.forEach(a => refreshingAccounts.value.delete(a.id));
    }
  }

  /**
   * 检查并自动刷新过期Token（供外部调用）
   */
  async function checkAndRefreshExpiredTokens(settingsStore?: any): Promise<void> {
    // 检查是否开启自动刷新
    if (settingsStore && !settingsStore.settings.auto_refresh_token) {
      return;
    }
    
    const accountsToRefresh = getAccountsNeedingRefresh();
    
    if (accountsToRefresh.length === 0) {
      return;
    }
    
    console.log(`[自动刷新] 检测到 ${accountsToRefresh.length} 个账号需要刷新Token`);
    
    // 获取并发限制
    // 如果开启了全量并发刷新，则不限制并发数（使用账号数量作为并发数）
    const unlimitedConcurrent = settingsStore?.settings.unlimitedConcurrentRefresh;
    const concurrentLimit = unlimitedConcurrent 
      ? accountsToRefresh.length 
      : (settingsStore?.settings.concurrent_limit || 3);
    
    if (unlimitedConcurrent) {
      console.log(`[自动刷新] 全量并发模式，同时刷新 ${accountsToRefresh.length} 个账号`);
    }
    
    await batchRefreshTokens(accountsToRefresh, concurrentLimit);
  }

  /**
   * 启动定时轮询（每10分钟检查一次）
   */
  function startAutoRefreshTimer(settingsStore?: any) {
    // 先清除旧的定时器
    stopAutoRefreshTimer();
    
    // 检查是否开启自动刷新
    if (settingsStore && !settingsStore.settings.auto_refresh_token) {
      console.log('[自动刷新] 自动刷新Token功能已关闭');
      return;
    }
    
    console.log('[自动刷新] 启动定时轮询，间陔10分钟');
    
    // 立即执行一次检查
    checkAndRefreshExpiredTokens(settingsStore);
    
    // 设置定时器，每10分钟执行一次
    autoRefreshTimerId.value = window.setInterval(() => {
      checkAndRefreshExpiredTokens(settingsStore);
    }, 10 * 60 * 1000); // 10分钟
  }

  /**
   * 停止定时轮询
   */
  function stopAutoRefreshTimer() {
    if (autoRefreshTimerId.value !== null) {
      clearInterval(autoRefreshTimerId.value);
      autoRefreshTimerId.value = null;
      console.log('[自动刷新] 定时轮询已停止');
    }
  }

  // ==================== 排序功能 ====================

  /**
   * 加载排序配置
   */
  async function loadSortConfig() {
    try {
      const config = await settingsApi.getSortConfig();
      sortConfig.value = config;
    } catch (e) {
      console.error('加载排序配置失败:', e);
    }
  }

  /**
   * 更新排序配置并重新排序
   */
  async function setSortConfig(field: SortField, direction: SortDirection) {
    sortConfig.value = { field, direction };
    try {
      await settingsApi.updateSortConfig(sortConfig.value);
      await applySorting();
    } catch (e) {
      console.error('更新排序配置失败:', e);
    }
  }

  /**
   * 应用当前排序配置（v1.7.8 方案 B：后端分页查询已支持排序，直接 fetchPage）
   */
  async function applySorting() {
    try {
      await fetchPage();
    } catch (e) {
      console.error('应用排序失败:', e);
    }
  }

  /**
   * 更新账户顺序（用于拖拽排序）
   */
  async function updateAccountsOrder(accountIds: string[]) {
    try {
      await settingsApi.updateAccountsOrder(accountIds);
      // v1.7.8 方案 B：后端已更新 sort_order，刷新当前页
      await fetchPage();
    } catch (e) {
      console.error('更新账户顺序失败:', e);
      throw e;
    }
  }

  /**
   * 按 ID 获取单个账号（v1.7.8：当前页命中直接返回，miss 则从后端拉取）。
   *
   * SQLite 分页后 `accounts` 只存当前页 20-100 条，编辑/查看/操作非当前页账号时
   * 需要此方法透明地从后端补齐。返回 null 表示后端也不存在该 ID。
   */
  async function getAccountById(id: string): Promise<Account | null> {
    // 当前页快速路径
    const local = accounts.value.find(a => a.id === id);
    if (local) return local;
    // 后端拉取
    try {
      return await accountApi.getAccount(id);
    } catch {
      return null;
    }
  }

  return {
    // State
    accounts,
    selectedAccounts,
    currentFilter,
    loading,
    error,
    pagination,
    sortConfig,
    
    // Computed
    filteredAccounts,
    paginatedAccounts,
    selectedAccountsList,
    activeAccountsCount,
    totalPages,
    totalCount,
    allTags,
    allPlanNames,
    allDomains,
    allGroups,
    
    // Actions
    loadGroups,
    loadTags,
    loadAccounts,
    addAccount,
    appendLocalAccount,
    updateAccount,
    patchLocalAccounts,
    deleteAccount,
    deleteSelectedAccounts,
    toggleAccountSelection,
    selectAll,
    clearSelection,
    setFilter,
    clearFilter,
    setCurrentPage,
    setPageSize,
    
    // 辅助函数
    getRemainingQuota,
    getDaysUntilExpiry,
    
    // 自动刷新Token
    isTokenExpiredOrExpiring,
    getAccountsNeedingRefresh,
    refreshAccountToken,
    batchRefreshTokens,
    checkAndRefreshExpiredTokens,
    startAutoRefreshTimer,
    stopAutoRefreshTimer,
    
    // 批量更新优化
    flushPendingUpdates,
    
    // 排序功能
    loadSortConfig,
    setSortConfig,
    applySorting,
    updateAccountsOrder,

    // v1.7.8 方案 B：分页查询 + 聚合统计
    fetchPage,
    refreshAggregates,
    aggregates,

    // v1.7.8：按 ID 获取单个账号（当前页命中直接返回，miss 则从后端拉取）
    getAccountById,
  };
});

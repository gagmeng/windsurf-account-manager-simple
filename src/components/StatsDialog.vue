<template>
  <el-dialog
    v-model="uiStore.showStatsDialog"
    title="统计信息"
    width="900px"
    class="stats-dialog"
  >
    <div v-if="loading" class="loading-container">
      <el-icon class="is-loading" size="32"><Loading /></el-icon>
    </div>
    
    <div v-else-if="stats" class="stats-content">
      <!-- 账号概览 -->
      <div class="section-title">账号概览</div>
      <el-row :gutter="16">
        <el-col :span="6">
          <div class="stat-card">
            <div class="stat-value">{{ stats.total_accounts }}</div>
            <div class="stat-label">账号总数</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="stat-card">
            <div class="stat-value success">{{ stats.active_accounts }}</div>
            <div class="stat-label">活跃账号</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="stat-card">
            <div class="stat-value">{{ stats.groups }}</div>
            <div class="stat-label">分组数量</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="stat-card">
            <div class="stat-value">{{ stats.tags_count || 0 }}</div>
            <div class="stat-label">标签数量</div>
          </div>
        </el-col>
      </el-row>

      <!-- 订阅与认证 -->
      <div class="section-title">订阅与认证</div>
      <el-row :gutter="16">
        <el-col :span="6">
          <div class="stat-card">
            <div class="stat-value success">{{ stats.active_subscriptions }}</div>
            <div class="stat-label">订阅激活</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="stat-card">
            <div class="stat-value warning">{{ stats.inactive_subscriptions }}</div>
            <div class="stat-label">订阅未激活</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="stat-card">
            <div class="stat-value">{{ stats.accounts_with_token }}</div>
            <div class="stat-label">已登录(Token)</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="stat-card">
            <div class="stat-value">{{ stats.accounts_with_refresh_token }}</div>
            <div class="stat-label">可切换(RefreshToken)</div>
          </div>
        </el-col>
      </el-row>

      <!-- 特殊账号 -->
      <div class="section-title">特殊账号</div>
      <el-row :gutter="16">
        <el-col :span="6">
          <div class="stat-card">
            <div class="stat-value primary">{{ stats.team_owners }}</div>
            <div class="stat-label">团队所有者</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="stat-card">
            <div class="stat-value danger">{{ stats.disabled_accounts }}</div>
            <div class="stat-label">禁用账号</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="stat-card">
            <div class="stat-value">{{ stats.accounts_with_quota }}</div>
            <div class="stat-label">有配额信息</div>
          </div>
        </el-col>
        <el-col :span="6">
          <el-tooltip placement="top">
            <template #content>
              已使用: {{ formatNumber(stats.total_used_quota) }}<br/>
              总配额: {{ formatNumber(stats.total_quota) }}
            </template>
            <div class="stat-card quota-card">
              <div class="quota-display">
                <span class="quota-used">{{ formatNumber(stats.total_used_quota) }}</span>
                <span class="quota-divider">——</span>
                <span class="quota-total">{{ formatNumber(stats.total_quota) }}</span>
              </div>
              <div class="stat-label">总配额使用</div>
            </div>
          </el-tooltip>
        </el-col>
      </el-row>

      <!-- 订阅类型分布 -->
      <div class="section-title">订阅类型分布</div>
      <div class="plan-stats">
        <el-tag 
          v-for="(count, plan) in stats.plan_stats" 
          :key="plan"
          :type="getPlanTagType(String(plan))"
          class="plan-tag"
        >
          {{ plan }}: {{ count }}
        </el-tag>
        <span v-if="!stats.plan_stats || Object.keys(stats.plan_stats).length === 0" class="no-data">暂无数据</span>
      </div>

      <!-- 分组分布 -->
      <div class="section-title">分组分布</div>
      <div class="group-stats">
        <el-tag 
          v-for="(count, group) in stats.group_stats" 
          :key="group"
          type="info"
          class="group-tag"
        >
          {{ group }}: {{ count }}
        </el-tag>
      </div>

      <!-- 标签分布 -->
      <div class="section-title">标签分布</div>
      <div class="tag-stats">
        <el-tag 
          v-for="(count, tag) in stats.tag_stats" 
          :key="tag"
          type="warning"
          class="tag-item"
        >
          {{ tag }}: {{ count }}
        </el-tag>
        <span v-if="!stats.tag_stats || Object.keys(stats.tag_stats).length === 0" class="no-data">暂无标签</span>
      </div>

      <el-divider />

      <!-- 操作统计 -->
      <div class="section-title">操作统计</div>
      <el-row :gutter="16">
        <el-col :span="12">
          <div class="stat-card large">
            <div class="stat-value" :class="{ success: stats.success_rate >= 80, warning: stats.success_rate >= 50 && stats.success_rate < 80, danger: stats.success_rate < 50 }">
              {{ stats.success_rate?.toFixed(1) || 0 }}%
            </div>
            <div class="stat-label">操作成功率</div>
            <div class="stat-detail">成功: {{ stats.successful_operations }} / 失败: {{ stats.failed_operations }}</div>
          </div>
        </el-col>
        <el-col :span="12">
          <div class="stat-card large">
            <div class="stat-value" :class="{ success: stats.reset_success_rate >= 80, warning: stats.reset_success_rate >= 50 && stats.reset_success_rate < 80, danger: stats.reset_success_rate < 50 }">
              {{ stats.reset_success_rate?.toFixed(1) || 0 }}%
            </div>
            <div class="stat-label">积分重置成功率</div>
            <div class="stat-detail">成功: {{ stats.successful_resets }} / 失败: {{ stats.failed_resets }}</div>
          </div>
        </el-col>
      </el-row>

      <!-- 操作类型统计 -->
      <div class="section-title">操作类型统计</div>
      <div class="operation-stats">
        <el-tag 
          v-for="(count, opType) in stats.operation_type_stats" 
          :key="opType"
          :type="getOperationTagType(String(opType))"
          class="op-tag"
        >
          {{ translateOperationType(String(opType)) }}: {{ count }}
        </el-tag>
        <span v-if="!stats.operation_type_stats || Object.keys(stats.operation_type_stats).length === 0" class="no-data">暂无操作记录</span>
      </div>

      <el-divider />

      <!-- 系统设置 -->
      <div class="section-title">系统设置</div>
      <el-descriptions :column="2" border size="small">
        <el-descriptions-item label="总操作次数">{{ stats.total_operations }}</el-descriptions-item>
        <el-descriptions-item label="总重置次数">{{ stats.total_resets }}</el-descriptions-item>
        <el-descriptions-item label="最后操作时间">{{ stats.last_operation ? formatDate(stats.last_operation) : '暂无' }}</el-descriptions-item>
        <el-descriptions-item label="自动刷新Token">
          <el-tag :type="stats.settings?.auto_refresh_token ? 'success' : 'info'" size="small">
            {{ stats.settings?.auto_refresh_token ? '开启' : '关闭' }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="重试次数">{{ stats.settings?.retry_times || 2 }}</el-descriptions-item>
        <el-descriptions-item label="并发限制">{{ stats.settings?.concurrent_limit || 5 }}</el-descriptions-item>
      </el-descriptions>
    </div>
    
    <template #footer>
      <el-button @click="refresh" :icon="Refresh">刷新</el-button>
      <el-button @click="uiStore.closeStatsDialog">关闭</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { ElMessage } from 'element-plus';
import { Loading, Refresh } from '@element-plus/icons-vue';
import { useUIStore } from '@/store';
import { settingsApi } from '@/api';
import dayjs from 'dayjs';

const uiStore = useUIStore();

const loading = ref(false);
const stats = ref<any>(null);

watch(() => uiStore.showStatsDialog, (show) => {
  if (show) {
    loadStats();
  }
});

onMounted(() => {
  if (uiStore.showStatsDialog) {
    loadStats();
  }
});

async function loadStats() {
  loading.value = true;
  try {
    stats.value = await settingsApi.getStats();
  } catch (error) {
    ElMessage.error(`加载统计信息失败: ${error}`);
  } finally {
    loading.value = false;
  }
}

function refresh() {
  loadStats();
}

function formatDate(date: string) {
  return dayjs(date).format('YYYY-MM-DD HH:mm:ss');
}

function formatNumber(value: number): string {
  if (!value) return '0';
  return value.toLocaleString();
}

function getPlanTagType(plan: string): string {
  const planLower = plan.toLowerCase();
  if (planLower.includes('pro') || planLower.includes('premium')) return 'success';
  if (planLower.includes('free') || planLower.includes('basic')) return 'info';
  if (planLower.includes('team') || planLower.includes('enterprise')) return 'warning';
  if (planLower === '未知') return 'info';
  return '';
}

function getOperationTagType(opType: string): string {
  if (opType.includes('Reset') || opType.includes('Delete') || opType.includes('Remove')) return 'danger';
  if (opType.includes('Add') || opType.includes('Create') || opType.includes('Register')) return 'success';
  if (opType.includes('Update') || opType.includes('Edit') || opType.includes('Rename')) return 'warning';
  if (opType.includes('Login') || opType.includes('Refresh') || opType.includes('Get')) return 'info';
  return '';
}

const operationTypeMap: Record<string, string> = {
  'Login': '登录',
  'RefreshToken': '刷新Token',
  'ResetCredits': '重置积分',
  'UpdateSeats': '更新席位',
  'GetBilling': '获取账单',
  'UpdatePlan': '更新套餐',
  'GetAccountInfo': '获取账号信息',
  'AddAccount': '添加账号',
  'DeleteAccount': '删除账号',
  'EditAccount': '编辑账号',
  'BatchOperation': '批量操作',
  'AddGroup': '添加分组',
  'DeleteGroup': '删除分组',
  'RenameGroup': '重命名分组',
  'ChangeGroup': '更改分组',
  'AddTag': '添加标签',
  'DeleteTag': '删除标签',
  'UpdateTag': '更新标签',
  'BatchUpdateTags': '批量更新标签',
  'CreateTeam': '创建团队',
  'JoinTeam': '加入团队',
  'LeaveTeam': '离开团队',
  'InviteMember': '邀请成员',
  'RemoveMember': '移除成员',
  'TransferSubscription': '转移订阅',
  'GetTeamInfo': '获取团队信息',
  'ImportData': '导入数据',
  'ExportData': '导出数据',
  'CreateBackup': '创建备份',
  'RestoreBackup': '恢复备份',
  'SwitchAccount': '切换账号',
  'ApplyTrial': '申请试用',
  'CancelSubscription': '取消订阅',
  'RegisterAccount': '注册账号',
};

function translateOperationType(opType: string): string {
  return operationTypeMap[opType] || opType;
}
</script>

<style scoped>
.stats-content {
  max-height: 70vh;
  overflow-y: auto;
  padding-right: 8px;
}

.loading-container {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 40px;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  margin: 16px 0 12px 0;
  padding-left: 8px;
  border-left: 3px solid #409eff;
}

.section-title:first-child {
  margin-top: 0;
}

.stat-card {
  background: #f5f7fa;
  border-radius: 8px;
  padding: 16px;
  text-align: center;
  transition: all 0.3s;
  min-height: 80px;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.stat-card:hover {
  background: #ecf5ff;
  transform: translateY(-2px);
}

.stat-card.large {
  padding: 20px;
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.stat-card.large .stat-value {
  font-size: 32px;
}

.stat-value.success { color: #67c23a; }
.stat-value.warning { color: #e6a23c; }
.stat-value.danger { color: #f56c6c; }
.stat-value.primary { color: #409eff; }

.quota-card {
  cursor: help;
}

.quota-display {
  display: flex;
  flex-direction: column;
  align-items: center;
  line-height: 1.1;
}

.quota-used {
  font-size: 16px;
  font-weight: 700;
  color: #409eff;
}

.quota-divider {
  font-size: 12px;
  color: #c0c4cc;
  margin: 1px 0;
}

.quota-total {
  font-size: 16px;
  font-weight: 700;
  color: #303133;
}

.stat-label {
  font-size: 12px;
  color: #909399;
  margin-top: 6px;
}

.stat-detail {
  margin-top: 8px;
  font-size: 12px;
  color: #909399;
}

.plan-stats,
.group-stats,
.tag-stats,
.operation-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 8px 0;
}

.plan-tag,
.group-tag,
.tag-item,
.op-tag {
  font-size: 13px;
}

.no-data {
  color: #909399;
  font-size: 13px;
}

.el-row {
  margin-bottom: 12px;
}

.el-divider {
  margin: 16px 0;
}

:deep(.el-descriptions__label) {
  width: 120px;
}
</style>

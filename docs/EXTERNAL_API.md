# 外部 HTTP API 使用文档

本应用内置一个轻量 HTTP REST API，供**同机其它工具**（脚本 / 浏览器插件 / 第三方客户端）读取账号数据并取用 token，无需重新实现登录与刷新逻辑。

> ⚠️ **安全提示**：本接口**没有任何鉴权 / CORS 限制 / 速率限制**，且**默认返回所有字段（password / token / refresh_token / devin_auth1_token 全部明文）**。仅可绑定 `127.0.0.1` 仅本机使用，**严禁暴露到公网**。

---

## 一、启用 API 服务

1. 启动应用 → 顶部菜单或设置入口 → **设置**
2. 切到 **外部 API** 标签页
3. 打开「**启用 API 服务**」开关
4. （可选）修改监听地址 / 端口
   - **监听地址**：默认 `127.0.0.1`（仅本机），改为 `0.0.0.0` 则对所有网卡开放（**强烈不推荐**）
   - **监听端口**：默认 `46953`
5. 点击保存。无需重启应用，HTTP 服务会自动启动 / 重启
6. 应用日志会输出：`[API] 外部 HTTP API 服务已启动：http://127.0.0.1:46953`

---

## 二、通用约定

### 基础 URL

```
http://<host>:<port>/api/v1
```

默认即：`http://127.0.0.1:46953/api/v1`

### 编码

- **请求**：所有参数通过 URL Query String 传递，UTF-8 编码
- **响应**：`Content-Type: application/json; charset=utf-8`

### HTTP 方法

当前所有路由均为 **GET**。

### 错误格式

非 2xx 响应统一为：

```json
{
  "error": {
    "code": 404,
    "message": "账号不存在：..."
  }
}
```

| HTTP 状态码 | 场景 |
|---|---|
| `200 OK` | 成功 |
| `400 Bad Request` | 参数非法（如 UUID 格式错误） |
| `404 Not Found` | 账号 / 资源不存在 |
| `500 Internal Server Error` | 服务端异常（DB 错误、token 刷新失败等） |

---

## 三、路由清单

| Method | Path | 描述 |
|---|---|---|
| `GET` | `/api/v1/health` | 健康检查 + 版本号 |
| `GET` | `/api/v1/accounts` | 账号列表（分页 + 过滤 + 排序） |
| `GET` | `/api/v1/accounts/:id` | 单条账号详情 |
| `GET` | `/api/v1/accounts/:id/token` | 取有效 access_token（可自动刷新） |
| `GET` | `/api/v1/groups` | 分组列表 |
| `GET` | `/api/v1/stats` | 聚合统计 |

---

## 四、各路由详解

### 1. `GET /api/v1/health`

#### 请求
```
GET http://127.0.0.1:46953/api/v1/health
```

#### 响应
```json
{
  "ok": true,
  "version": "1.7.12",
  "name": "windsurf-account-manager"
}
```

用途：探活、版本号检查、客户端兼容性判断。

---

### 2. `GET /api/v1/accounts`

分页查询账号列表，支持丰富的过滤和排序。

#### Query 参数

##### 分页

| 参数 | 类型 | 默认 | 说明 |
|---|---|---|---|
| `page` | int | `1` | 页码（1-indexed） |
| `page_size` | int | `20` | 每页条数，最多 1 万左右建议 |

##### 过滤

| 参数 | 类型 | 说明 |
|---|---|---|
| `search` | string | 模糊匹配 `email` / `nickname` / `tags` |
| `group` | string | 分组**精确**匹配 |
| `tags[]` | string 数组 | 任一标签匹配（OR） |
| `plan_names[]` | string 数组 | 套餐名匹配（OR） |
| `domains[]` | string 数组 | 邮箱域名匹配（OR） |
| `statuses[]` | string 数组 | 状态：`normal` / `offline` / `error` / `disabled` / `inactive` |
| `remaining_quota_min` / `remaining_quota_max` | int×100 | 剩余额度（值已乘 100；100% 传 `10000`） |
| `total_quota_min` / `total_quota_max` | int×100 | 总额度（同上） |
| `expiry_days_min` / `expiry_days_max` | int | 距订阅到期天数 |
| `daily_quota_percent_min` / `daily_quota_percent_max` | int 0-100 | 日配额剩余百分比 |
| `weekly_quota_percent_min` / `weekly_quota_percent_max` | int 0-100 | 周配额剩余百分比 |

> **数组传参方式**：URL 里多次出现同名 key 即可，如 `?plan_names[]=TRIAL&plan_names[]=PRO`。
> URL 编码下：`[]` → `%5B%5D`、空格 → `%20` 或 `+`。

##### 排序

| 参数 | 类型 | 说明 |
|---|---|---|
| `sort_field` | string | 排序字段，常用：`email` / `created_at` / `last_login_at` / `subscription_expires_at` / `total_quota` / `used_quota` |
| `sort_direction` | string | `asc` / `desc`，默认 `asc` |

#### 响应

```json
{
  "accounts": [ /* Account[] - 详见后文 Account 字段说明 */ ],
  "total": 234,
  "page": 1,
  "page_size": 50
}
```

#### 示例

```bash
# 拉所有账号
curl "http://127.0.0.1:46953/api/v1/accounts?page=1&page_size=10000"

# 仅 TRIAL 套餐
curl "http://127.0.0.1:46953/api/v1/accounts?plan_names%5B%5D=TRIAL"

# 多套餐 + 状态正常
curl "http://127.0.0.1:46953/api/v1/accounts?plan_names%5B%5D=TRIAL&plan_names%5B%5D=PRO&statuses%5B%5D=normal"

# 即将过期的账号（剩余 1-7 天）
curl "http://127.0.0.1:46953/api/v1/accounts?expiry_days_min=1&expiry_days_max=7"

# 按到期时间倒序
curl "http://127.0.0.1:46953/api/v1/accounts?sort_field=subscription_expires_at&sort_direction=desc"
```

---

### 3. `GET /api/v1/accounts/:id`

获取单条账号完整详情。

#### Path 参数

| 参数 | 类型 | 说明 |
|---|---|---|
| `id` | UUID 字符串 | 账号 UUID（带或不带连字符均可） |

#### 响应

返回单个 `Account` 对象（与列表项字段相同，含全部敏感字段）。

#### 示例

```bash
curl http://127.0.0.1:46953/api/v1/accounts/9f3e4a2b-1234-5678-9abc-def012345678
```

#### 错误

- `400`：UUID 格式非法
- `404`：账号不存在

---

### 4. `GET /api/v1/accounts/:id/token`

取该账号当前可用的 access_token。**可触发自动刷新**：当 token 已过期且 `auto_refresh=true` 时，会调用上游 refresh API、写回 SQLite 后再返回新 token。

#### Path 参数

| 参数 | 类型 | 说明 |
|---|---|---|
| `id` | UUID 字符串 | 账号 UUID |

#### Query 参数

| 参数 | 类型 | 默认 | 说明 |
|---|---|---|---|
| `auto_refresh` | bool | `true` | token 不存在或过期时是否触发刷新 |

#### 响应

```json
{
  "token": "eyJhbGciOi...（Firebase idToken 或 Devin session_token）",
  "expires_at": "2026-05-09T15:30:00Z",
  "is_devin": false
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `token` | string | 主认证令牌 |
| `expires_at` | string \| null | ISO 8601 过期时间，Devin 体系可能为 null |
| `is_devin` | bool | true 表示 Devin Session 体系账号（token 为 session_token） |

#### 示例

```bash
# 自动刷新（默认）
curl http://127.0.0.1:46953/api/v1/accounts/<UUID>/token

# 不刷新，只读取当前存储的 token
curl "http://127.0.0.1:46953/api/v1/accounts/<UUID>/token?auto_refresh=false"
```

---

### 5. `GET /api/v1/groups`

#### 响应

```json
["默认分组", "VIP", "测试组"]
```

字符串数组，按用户配置顺序返回。

---

### 6. `GET /api/v1/stats`

聚合统计，与前端统计面板使用同一份数据。

#### 响应

```json
{
  "total_count": 234,
  "groups": ["默认分组", "VIP"],
  "plan_names": ["TRIAL", "PRO", "TEAMS", "FREE"],
  "domains": ["gmail.com", "outlook.com", ...],
  "tags": ["new", "long-lived", ...],
  "active_count": 120,
  "group_counts": {
    "默认分组": 200,
    "VIP": 34
  },
  "tag_counts": {
    "new": 15,
    "long-lived": 80
  }
}
```

| 字段 | 说明 |
|---|---|
| `total_count` | 账号总数 |
| `active_count` | `subscription_active = true` 的账号数 |
| `groups` / `plan_names` / `domains` / `tags` | 去重后的所有取值，可作为前端下拉框选项 |
| `group_counts` / `tag_counts` | 每个分组 / 标签的账号数 |

---

## 五、Account 对象字段说明

`/accounts` 与 `/accounts/:id` 返回的对象字段（**所有字段无脱敏**）：

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | string (UUID) | 唯一标识 |
| `email` | string | 邮箱 |
| `password` | string | **明文密码**（DB 中以明文存储） |
| `nickname` | string | 昵称 |
| `tags` | string[] | 标签名数组 |
| `tagColors` | object[] | 带颜色的标签：`[{name, color}]` |
| `group` | string \| null | 分组名 |
| `token` | string \| null | **当前可用 access_token**（Firebase idToken 或 Devin session_token） |
| `refresh_token` | string \| null | **刷新令牌** |
| `token_expires_at` | string \| null | token 过期时间（ISO 8601） |
| `last_seat_count` | int \| null | 上次同步的席位数 |
| `created_at` | string | 添加到本应用的时间 |
| `last_login_at` | string \| null | 上次成功登录时间 |
| `status` | string \| object | 状态：`"active"` / `"inactive"` / `{"error": "..."}` |
| `plan_name` | string \| null | 套餐名（如 `TRIAL` / `PRO` / `TEAMS` / `FREE` 等） |
| `used_quota` | int \| null | 已用配额（×100） |
| `total_quota` | int \| null | 总配额（×100） |
| `last_quota_update` | string \| null | 配额最后更新时间 |
| `subscription_expires_at` | string \| null | 订阅到期时间 |
| `subscription_active` | bool \| null | 订阅是否激活 |
| `windsurf_api_key` | string \| null | Windsurf API Key（用户 UUID） |
| `is_disabled` | bool \| null | 账户是否被禁用 |
| `is_team_owner` | bool \| null | 是否为团队所有者 |
| `billing_strategy` | int \| null | 计费策略：0=未指定 / 1=Credits / 2=Quota / 3=ACU |
| `daily_quota_remaining_percent` | int \| null | 日配额剩余 %（0-100，仅 QUOTA 模式有效） |
| `weekly_quota_remaining_percent` | int \| null | 周配额剩余 %（同上） |
| `daily_quota_reset_at_unix` | int \| null | 日配额重置时间（Unix 秒） |
| `weekly_quota_reset_at_unix` | int \| null | 周配额重置时间（Unix 秒） |
| `overage_balance_micros` | int \| null | 额外使用余额（微美元，÷1e6 = 美元） |
| `sortOrder` | int | 自定义排序顺序 |
| `auth_provider` | string \| null | `"firebase"`（默认）或 `"devin"` |
| `devin_auth1_token` | string \| null | Devin 一级认证令牌 |
| `devin_account_id` | string \| null | Devin 账号 ID（`account-<32 hex>`） |
| `devin_primary_org_id` | string \| null | Devin 主组织 ID |

---

## 六、状态值含义

`statuses[]` 过滤参数和 `Account.status` 字段使用的状态枚举（与前端徽章一致）：

| 状态 | 判定优先级 | 含义 |
|---|---|---|
| `error` | 1 | `status` 字段含 `"error"`（账号操作出错） |
| `inactive` | 2 | 付费计划但 `subscription_active = false`（订阅已失效） |
| `disabled` | 3 | `is_disabled = true`（账号被官方禁用） |
| `offline` | 4 | token 未设置或已过期（需重登或刷新） |
| `normal` | 5 | 正常可用 |

---

## 七、典型使用场景

### 场景 1：批量取所有账号 + token，喂给下游脚本

#### Python

```python
import requests

BASE = "http://127.0.0.1:46953/api/v1"

resp = requests.get(f"{BASE}/accounts", params={"page": 1, "page_size": 10000}).json()

for acc in resp["accounts"]:
    if not acc.get("token"):
        # 自动刷新一次
        t = requests.get(f"{BASE}/accounts/{acc['id']}/token").json()
        acc["token"] = t["token"]

print(f"准备好 {len(resp['accounts'])} 个账号 + token")
# 直接拿 acc["token"] 调 Windsurf 下游 API
```

#### Node.js

```javascript
const BASE = 'http://127.0.0.1:46953/api/v1';
const list = await (await fetch(`${BASE}/accounts?page=1&page_size=10000`)).json();
for (const acc of list.accounts) {
  if (!acc.token) {
    const t = await (await fetch(`${BASE}/accounts/${acc.id}/token`)).json();
    acc.token = t.token;
  }
}
console.log(`Ready: ${list.accounts.length}`);
```

### 场景 2：找出所有 Trial 账号

```powershell
# PowerShell：客户端过滤（不区分大小写，最稳）
(Invoke-RestMethod "http://127.0.0.1:46953/api/v1/accounts?page=1&page_size=10000").accounts |
  Where-Object { $_.plan_name -match "trial" } |
  Select-Object email, plan_name, subscription_expires_at, daily_quota_remaining_percent |
  Format-Table -AutoSize
```

```bash
# 或服务端过滤（需先用 /stats 确认大小写）
curl "http://127.0.0.1:46953/api/v1/accounts?plan_names%5B%5D=TRIAL&page_size=10000"
```

### 场景 3：找快过期的账号（7 天内）

```bash
curl "http://127.0.0.1:46953/api/v1/accounts?expiry_days_min=0&expiry_days_max=7&page_size=10000"
```

### 场景 4：找配额还充足的账号（按周配额排序，剩余 > 50%）

```bash
curl "http://127.0.0.1:46953/api/v1/accounts?weekly_quota_percent_min=50&sort_field=weekly_quota_remaining_percent&sort_direction=desc&page_size=100"
```

### 场景 5：定时巡检 + 邮件告警

```python
import requests, smtplib

BASE = "http://127.0.0.1:46953/api/v1"
# 找出失效账号
bad = requests.get(f"{BASE}/accounts", params={
    "statuses[]": ["error", "disabled", "offline"],
    "page_size": 10000,
}).json()

if bad["total"] > 0:
    body = "\n".join(f"{a['email']} - {a['status']}" for a in bad["accounts"])
    # send_mail("账号告警", body)
    print(f"⚠️ 发现 {bad['total']} 个异常账号:\n{body}")
```

---

## 八、Postman / Apifox 配置

1. **新建 Collection**：`Windsurf Account Manager API`
2. **设置 Collection Variable**：
   - `base_url` = `http://127.0.0.1:46953/api/v1`
3. **示例请求**：
   - 名称：`列出 TRIAL 账号`
   - Method：`GET`
   - URL：`{{base_url}}/accounts`
   - **Params**：
     ```
     page          1
     page_size     10000
     plan_names[]  TRIAL
     plan_names[]  Devin Trial   (再加一行同名 key)
     statuses[]    normal
     ```
   - Postman 会自动拼成：
     `?page=1&page_size=10000&plan_names[]=TRIAL&plan_names[]=Devin+Trial&statuses[]=normal`

---

## 九、CORS / 浏览器调用

服务端启用了 `tower_http::cors::CorsLayer::permissive()`，**所有 Origin 都被允许**，可以直接在浏览器开发者工具或同机的 Web 应用中 `fetch()`：

```javascript
// 任意网页（同机）的 console 中可直接执行
fetch('http://127.0.0.1:46953/api/v1/health').then(r => r.json()).then(console.log)
```

---

## 十、运维 / 故障排查

### 端口被占

如果 `46953` 已被其他程序占用，启动 API 时应用日志会输出绑定失败，且 UI 上 API 服务未真正启动。请改用其他端口（建议 30000-65000 间不冲突的端口）。

### Vite 端口冲突

应用 `vite dev` 模式自身使用 `46952`（前端）与 `46953`（HMR）。**生产构建打包后不影响**，仅开发调试时若同时开启 API 默认端口需注意冲突 → 调整 API 端口或开发端口。

### 服务热重启

修改 **设置 → 外部 API** 中的任意字段（启用开关 / 监听地址 / 端口）保存后，**无需重启应用**：旧的 axum 监听器会优雅停止，新配置的监听器立刻接管。

### 强制关闭

关闭应用即关闭服务。HTTP 服务作为 tokio 任务运行在主进程内。

---

## 十一、版本

- 当前 API 版本：`v1`
- 应用版本：见 `/health` 响应的 `version` 字段
- 路由前缀使用 `/api/v1/...`，未来不兼容变更将通过 `v2` 引入。

---

## 十二、再次提醒

🔴 **此 API 无任何鉴权，且默认返回所有敏感字段（token、refresh_token、明文密码、devin_auth1_token）**。
🔴 **请始终绑定 `127.0.0.1` 仅本机使用**。
🔴 **如需对外开放，请自行在前端套一层带鉴权的反向代理**。

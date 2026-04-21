# Windsurf Account Manager - Simple

一个基于 Tauri + Vue 3 + TypeScript 开发的 Windsurf 多账号管理桌面应用，用于管理多个 Windsurf 账号并提供积分重置、账单查询、一键换号、订阅支付等功能。

> ⚠️ **免费软件声明**：本软件完全免费，如果你是付费购买的，说明你被骗了！

## 📦 项目信息

- **当前版本**: 1.7.5
- **许可证**: AGPL-3.0
- **开发语言**: Rust + TypeScript
- **支持平台**: Windows 10/11
- **架构模式**: Tauri 2.x 桌面应用

**🌐 English Version**: [README_EN.md](README_EN.md)

## 📱 交流群

<p align="center">
  <img src="public/交流群.png" alt="微信群二维码" width="300">
  &nbsp;&nbsp;&nbsp;&nbsp;
  <img src="public/QQ群.jpg" alt="QQ群二维码" width="300">
</p>

---

## 🖥️ 软件界面

<p align="center">
  <img src="public/主页.png" alt="软件主页" width="800">
</p>

<p align="center">
  <img src="public/账号信息.png" alt="账号信息" width="800">
</p>

<p align="center">
  <img src="public/设置.png" alt="设置" width="800">
</p>

---

## ✨ 功能特性

### 🔐 账号管理
- ✅ **添加/编辑/删除账号** - 完整的账号CRUD操作
- ✅ **账号分组管理** - 支持自定义分组，便于管理多个账号
- ✅ **标签系统** - 为账号添加自定义标签
- ✅ **账号状态实时显示** - 显示套餐类型、积分余额、过期时间等
- ✅ **批量账号操作** - 批量选择、批量重置、批量删除
- ✅ **密码/Token加密存储** - 使用AES-256-GCM加密，密钥存储在系统密钥链

### 💳 积分重置
- ✅ **一键积分重置** - 通过座位数更新API实现积分重置
- ✅ **智能座位轮换** - 自动在3/4/5座位数之间轮换
- ✅ **批量重置** - 支持多账号同时重置（最多5个并发）
- ✅ **团队批量重置** - 一键重置团队内所有成员的积分
- ✅ **自动重置计划** - 设置定时任务，自动执行积分重置

### 👥 团队管理
- ✅ **查看团队成员** - 列出团队内所有成员信息
- ✅ **邀请成员** - 通过邮箱邀请新成员加入团队
- ✅ **移除成员** - 从团队中移除指定成员
- ✅ **团队积分管理** - 统一管理团队成员的积分

### 🔄 一键换号
- ✅ **一键切换账号** - 快速切换到其他Windsurf账号
- ✅ **自动Token刷新** - 自动使用refresh_token获取新的access_token
- ✅ **OAuth回调触发** - 通过windsurf://协议自动完成登录
- ✅ **机器ID重置** - 重置设备标识，支持多设备使用（需管理员权限）

### 🔧 无感换号补丁
- ✅ **自动检测Windsurf路径** - 自动查找Windsurf安装位置
- ✅ **一键应用补丁** - 修改extension.js实现无感切换
- ✅ **移除超时限制** - 移除180秒OAuth超时限制
- ✅ **自动备份** - 打补丁前自动备份原文件（最多保留3份）
- ✅ **一键还原** - 从备份文件还原原始状态
- ✅ **自动重启Windsurf** - 补丁应用后自动重启生效

### 🛑 寸止功能
- ✅ **MCP 工具集成** - 提供 dialog-helper 和 confirm 工具
- ✅ **对话框辅助** - 帮助用户确认操作和输入信息
- ✅ **跨平台支持** - 支持 Windows、Linux、macOS
- ✅ **二进制文件管理** - 构建时按平台复制对应文件
- ✅ **智能打包** - 避免打包所有平台文件，减少安装包体积

### � 支付相关
- ✅ **虚拟卡生成** - 生成虚拟信用卡信息用于支付测试
- ✅ **自定义卡头** - 支持设置自定义BIN号或BIN范围
- ✅ **隐私支付窗口** - 独立的无痕浏览器窗口打开Stripe支付页面
- ✅ **支付宝/微信收款** - 支持国内支付方式（捐赠）

### 📊 数据查询
- ✅ **账单信息查询** - 查询套餐、额度、使用量等信息
- ✅ **订阅状态查看** - 显示订阅类型、到期时间、下次扣费日期
- ✅ **使用量统计** - 查看积分使用情况和剩余额度
- ✅ **全局刷新** - 右上角刷新按钮一键更新所有账号信息

### ⚙️ 系统设置
- ✅ **代理配置** - 支持HTTP代理设置
- ✅ **轻量API模式** - 使用GetPlanStatus替代GetCurrentUser减少请求
- ✅ **详细结果显示** - 可选显示API响应的详细信息
- ✅ **操作日志** - 记录所有操作历史，支持导出

### 🔒 数据安全
- ✅ **系统密钥链** - 使用Windows Credential Manager存储加密密钥
- ✅ **AES-256-GCM加密** - 所有敏感信息均加密存储
- ✅ **本地存储** - 数据仅存储在本地，不上传任何服务器
- ✅ **操作日志** - 完整的操作记录便于审计

### 🌐 多客户端支持
- ✅ **Windsurf 标准版** - 完整支持官方 Windsurf 客户端
- ✅ **Windsurf - Next** - 支持 Next 版本的独立配置与管理
- ✅ **自动检测** - 自动识别并适配不同客户端版本
- ✅ **独立配置** - 不同客户端使用独立的配置文件和数据存储

### 🤖 Devin 账号体系
- ✅ **auth1_token 鉴权** - 支持 Devin 的 auth1_token 认证机制
- ✅ **session_token 管理** - 自动刷新 session_token，32天占位机制
- ✅ **多组织支持** - 支持 Devin 多组织账号管理
- ✅ **智能刷新** - 基于 401 错误触发强制刷新机制
- ✅ **关键字段回填** - 自动补充 product 参数等关键字段

---

## 📜 版本历史

### v1.7.5 (2026-04-20)
- **新增 Firebase ↔ Devin 账号登录方式互转**：针对官方将部分老 Firebase 账号迁移到 Devin 体系（密码未变）导致本地 refresh_token 刷新失败的场景，新增后端双命令 `convert_account_to_devin` / `convert_account_to_firebase`，复用账号已存明文密码走目标体系的官方登录接口完成原子切换；账号卡操作第二排末尾加「转换登录方式」按钮，tooltip 按当前 `auth_provider` 动态切换（`Firebase→Devin` / `Devin→Firebase`）；Firebase→Devin 多组织场景内联 `ElMessageBox + h()` 渲染 radio list 让用户选 `org_id` 后二次提交。设计要点：幂等（目标体系=当前体系时直接 `already_converted=true` 无网络请求）；原子性（网络失败本地字段零改动）；切换成功自动调 GetCurrentUser + GetPlanStatus 补齐配额字段
- **新增 AddAccountDialog 合并卡片 + mode-card 网格美化**：删除独立的「Devin 原生注册」卡片，与「Devin 邮箱验证码」合并为统一单卡；进入后以两级 mode-card 网格子单选切换来源（`Devin 原生` / `Windsurf 侧`）与子流程（`login` / `signup`，仅 Windsurf 侧需要），Step 1 按 `source+flow` 动态渲染密码/姓名字段。新增 `sourceOptions` / `flowOptions` / `selectSource` / `selectFlow`，验证规则合并为 `devinEmailCodeStep0/Step1/Step1Signup`，原 `devinNative*` 状态全部清除，业务函数 `completeDevinNativeRegister` 重命名为 `completeDevinEmailCodeNativeRegister` 并复用合并后的 `devinEmailCodeEmailToken` / `formData.devinEmailCodeOtp` 统一状态
- **修复 WindsurfPostAuth 首次 404 时序竞态**：`devin_auth_service.rs` 的 `post_auth_to_login_result` 针对 "Devin 新建账号 → 立刻 PostAuth" 服务端 org 同步延迟，对 `404 + no_eligible_organizations / not_found` 特定错误做 1.5s / 3s / 5s 共 3 次指数退避重试（新增 `is_post_auth_org_sync_pending` 精确识别辅助函数），覆盖 `login_with_password` / `register_with_email_code` / `login_with_email_code` / `register_native_with_email_code` 四条汇聚路径；其他错误保持 fail-fast 不重试
- **新增 `enrich_account_with_plan_status` / `enrich_account_with_plan_status_inner`**（`devin_commands.rs`）：提供"仅调 GetPlanStatus 并回填账号字段"的独立入口，与 `enrich_account_with_user_info` 共享 ctx 构造逻辑
- **修复 Devin 注册/登录路径 QUOTA 模式字段缺失（A.1）**：`enrich_account_with_user_info` 内部在 GetCurrentUser 之后追加一次 GetPlanStatus 调用，统一补齐 `billing_strategy` / `daily_quota_remaining_percent` / `weekly_quota_remaining_percent` / `daily/weekly_quota_reset_at_unix` / `overage_balance_micros` 等 QUOTA 模式专用字段，避免账号卡降级到 CREDITS 积分显示
- **修复 Devin 迁入路径 QUOTA 字段缺失（A.2）**：`add_account_by_devin_session_token` / `add_account_by_devin_auth1_token` 两条已有 user_info 的路径在 `apply_user_info_to_account` 后显式调 `enrich_account_with_plan_status` 单独补齐 QUOTA 字段；auth1_token 路径改用 `&ctx.token` 传参，规避 `account.token` 已被 move 进 Option 期间无法再借用的 E0502 借用冲突
- **修复 Firebase refresh_token 迁入路径 QUOTA 字段缺失**：`account_commands.rs` 的 `add_account_by_refresh_token` 在 GetCurrentUser 之后补一次 GetPlanStatus + `apply_plan_status_to_account` 回填；头部新增 `use crate::commands::api_commands::apply_plan_status_to_account` 复用字段映射
- **修复 E0502 借用冲突**：`devin_commands.rs:1144` 处 `account.token` 已被 move 到 `Some(session_token)` 后，若再以 `account.token.as_deref()` 不可变借用会违反借用规则——改用上方构造好的 `ctx.token`（与 session_token 同值，仍 owned 可借）传入 `enrich_account_with_plan_status`
- **统一版本号**：`package.json` / `Cargo.toml` 从 `1.7.3` 同步到 `1.7.5`，与 `tauri.conf.json` 对齐，消除 GitHub Actions `tauri-action` 版本一致性检查告警
- **影响范围**：所有使用 Devin 原生 / Windsurf 侧邮箱验证码 / Firebase refresh_token 建号的路径；配额显示从 CREDITS 降级问题系统性修复；UI 层原 2 张 Devin 邮箱验证码卡 → 1 张（主卡从 8 减到 7）

### v1.7.4 (2026-04-20)
- **新增 自动更新功能 (tauri-plugin-updater)**: 集成官方 updater + process 插件，应用启动后 3 秒静默检测 GitHub Releases 最新版本，发现新版时自动弹出更新对话框；支持下载进度实时展示、安装后一键重启、"跳过此版本"本地记忆、24 小时检测防抖
- **新增 UpdateDialog.vue 独立更新对话框**: 显示当前版本 / 新版本 / 发布说明 / 下载进度 / 安装状态，支持"立即更新 / 跳过此版本 / 稍后提醒"
- **新增 关于对话框"检查更新"按钮**: 版本号旁可手动触发检查（忽略 24h 防抖）；按钮文案随状态动态切换（检查中 / 立即更新 / 下载中 / 安装中 / 重启应用）
- **新增 Pinia updater store** (`src/store/modules/updater.ts`): 统一管理更新检查、下载、安装、重启、版本跳过等状态与动作
- **改造 GitHub Actions 构建流程** (`.github/workflows/build.yml`): 6 个构建 Job 注入 `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 环境变量完成签名；Upload artifacts 扩展收集 `.msi.sig` / `.exe.sig` / `.app.tar.gz.sig` / `.AppImage.sig`；Release Job 新增"合并各平台签名生成 latest.json"步骤并上传至 Release
- **新增 Tauri 配置**: `src-tauri/tauri.conf.json` 新增 `plugins.updater`（endpoints 指向 GitHub Releases latest.json、Windows 静默安装、minisign 公钥嵌入）与 `bundle.createUpdaterArtifacts: true`；`capabilities/default.json` 授权 `updater:default` + `process:default`
- **新增 前端依赖**: `@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process`
- **新增 Devin Auth1 Token 直接导入通道**: 新增后端命令 `add_account_by_devin_auth1_token`，用户粘贴浏览器 localStorage 的 `devin_auth1_token`（格式 `auth1_<52字符>`）即可一键迁入 Devin 账号。后端内部自动完成 `windsurf_post_auth` 换取 session_token → `GetCurrentUser` 反查 email / 配额 / api_key → 落库；相比 Session Token 迁入通道**额外保留 auth1_token**，到期可直接用 `refresh_devin_session` 续期。`AddAccountDialog` 新增「Devin Auth1 Token」Tab（与「Devin Session Token」并列），`BatchImportDialog` 同步新增 `devin_auth1_token` 批量导入模式（多组织自动选 primary org，不中断批量流程）
- **修复 macOS / Linux 构建失败**: 删除 `src-tauri/tauri.conf.json` 中值为空串的 `windsurf-account-manager.exe.manifest` resource 条目——该 manifest 已通过 `build.rs::WindowsAttributes::app_manifest` 直接嵌入 Windows exe，无需作为运行时资源；原空值被 bundler 解析成目录路径触发 `Is a directory (os error 21)`
- **对齐 Tauri NPM / Rust crate 版本**: 收紧 `Cargo.toml` 中 `tauri = "2.10"` / `tauri-build = "2.5"` / `tauri-plugin-updater = "2.10"` / `tauri-plugin-dialog = "2.7"` / `tauri-plugin-opener = "2.5"` / `tauri-plugin-process = "2.3"` 版本约束；同步刷新 `package.json` 中 `@tauri-apps/*` 依赖与 `package-lock.json`，消除 tauri-action 的版本一致性检查告警
- **优化 GitHub Actions Release 流程** (`.github/workflows/build.yml`): Release Job 自动从 README.md 提取当前版本的版本历史条目作为 GitHub Release 描述与 `latest.json` 的 `notes` 字段；`draft: false` 构建完成后直接发布到 Release 页面，无需手动点"Publish release"
- **影响范围**: 所有平台（Windows x64/arm64、macOS x64/arm64、Linux x64 AppImage）；Linux ARM64 因交叉编译限制不支持自动更新（既有限制未改动）

### v1.7.3 (2026-04-20)
- **修复 Devin 刷新逻辑**: 补充关键字段回填、修正 product 参数大小写并完善批量刷新的 store 同步
- 优化 Devin session_token 到期时间占位机制
- 完善无感切换进度事件系统

### v1.7.2 (2026-04-18)
- **新增 Devin 账号体系完整支持**: 实现 auth1_token 鉴权、session_token 刷新与 5-header 认证机制
- 优化 update_codeium_access 错误处理: 新增 Connect Protocol 错误码映射与透明化日志
- 完善 Devin session_token 管理逻辑

### v1.7.1 (2026-04-14)
- **新增多客户端支持**: 支持 Windsurf 和 Windsurf - Next 的独立配置与管理
- 优化客户端检测逻辑
- 改进配置文件路径管理

### v1.7.0 (2026-04-10)
- **新增计费策略与配额详情字段支持**: 支持 billing_strategy、daily_quota、weekly_quota 等字段
- 扩展 OperationType 枚举: 新增分组、标签、团队、数据管理、账号切换、订阅和注册操作类型
- 优化批量刷新逻辑: 复用 apply_plan_status_to_account 函数并简化账号更新流程

### v1.6.7 (2026-01-09)
- 修复寸止功能问题
- 优化稳定性

### v1.6.6 (2026-01-08)
- **重构订阅支付功能**: 支持 Teams/Pro 计划和月付/年付选择
- 重构 MCP 工具命名: 将 `windsurf-cunzhi` 和 `zhi` 工具统一重命名为 `dialog-helper` 和 `confirm`

### v1.6.5 (2025-12-30)
- 恢复完整功能版本
- 移除未使用的 Rank 图标导入
- 新增 QQ 群二维码到 README 交流群章节
- 更新 README.md: 完善功能特性说明和界面展示

### v1.6.4 (2025-12-24)
- **优化分析数据获取逻辑**: 改进 GetAnalytics API 调用效率
- 更新所有平台的 windsurf-cunzhi 和 windsurf-cunzhi-ui 二进制文件
- 优化 cunzhi 文件打包策略: 构建时按平台复制，避免打包所有平台文件
- 修复 Linux ARM64 连接器问题
- 跳过 Linux ARM64 构建中的 AppImage 打包，仅生成 deb 和 rpm 包

---

## 技术栈

### 前端
- **框架**: Vue 3.5.13 + TypeScript 5.6.2
- **UI组件**: Element Plus 2.11.8
- **图标库**: @element-plus/icons-vue 2.3.2
- **状态管理**: Pinia 3.0.4
- **构建工具**: Vite 6.0.3
- **样式**: CSS3 + Element Plus主题 + Sass 1.94.2
- **HTTP客户端**: Axios 1.13.2
- **加密库**: Crypto-js 4.2.0
- **日期处理**: Dayjs 1.11.19
- **图表库**: ECharts 6.0.0
- **拖拽排序**: Vuedraggable 4.1.0
- **UUID生成**: UUID 13.0.0

### 后端
- **框架**: Tauri 2.x
- **语言**: Rust 2021 Edition
- **加密**: AES-256-GCM (aes-gcm 0.10)
- **密钥管理**: Windows Credential Manager / Keyring 2.0
- **网络请求**: Reqwest 0.11 (支持 JSON、SOCKS 代理)
- **异步运行时**: Tokio 1.x (full features)
- **序列化**: Serde 1.0 + Serde JSON 1.0
- **数据库**: SQLite 3.31 (bundled) - 用于 Windsurf state.vscdb 操作
- **时间处理**: Chrono 0.4
- **编码**: Base64 0.21 + Hex 0.4
- **错误处理**: Anyhow 1.0 + Thiserror 1.0
- **日志**: Log 0.4 + Env Logger 0.10
- **文件压缩**: Zip 0.6
- **正则表达式**: Regex 1.10

### Windows 特定依赖
- **注册表操作**: Winreg 0.52
- **Windows API**: Winapi 0.3 (securitybaseapi, processthreadsapi, winnt, handleapi, minwindef, shellapi, winuser)

### Unix 特定依赖
- **系统调用**: Libc 0.2

## 安装和运行

### 前提条件
- Node.js 16+
- Rust 1.70+
- Windows 10/11（目前仅支持Windows）

### 开发环境

```bash
# 克隆项目
git clone https://github.com/chaogei/windsurf-account-manager-simple.git
cd windsurf-account-manager-simple

# 安装依赖
npm install

# 开发模式运行
npm run tauri dev
```

### 构建发布版

```bash
# 构建Windows安装包
npm run tauri build
```

构建完成后，安装包位于 `src-tauri/target/release/bundle/`

## 使用说明

### 1. 首次使用

1. 启动应用后，点击右上角的"添加账号"按钮（+图标）
2. 在弹出的对话框中输入 Windsurf 账号信息：
   - **邮箱**: 你的 Windsurf 账号邮箱
   - **密码**: 账号密码
   - **备注名称**: 便于识别的名称（可选）
   - **分组**: 选择或创建分组（可选）
   - **标签**: 添加自定义标签（可选）
3. 点击"确定"保存账号
4. 应用会自动登录并获取账号信息

### 2. 积分重置

**单账号重置**：
1. 在账号卡片上点击"积分重置"按钮（刷新图标）
2. 应用会自动：
   - 登录获取 Token（如需要）
   - 执行一次座位数更新（在 3/4/5 之间自动切换）
   - 座位更新成功即完成积分重置
3. 操作结果会通过通知提示
4. 可在设置中开启"显示详细结果"查看座位更新的具体信息

**批量重置**：
1. 勾选多个账号卡片
2. 点击顶部的"批量重置积分"按钮
3. 确认操作后批量执行（最多 5 个并发）
4. 查看操作日志了解详细结果

**自动重置**：
1. 点击"自动重置"按钮
2. 设置重置计划（时间、频率、账号）
3. 系统会自动在指定时间执行重置
4. 可在操作日志中查看自动重置记录

### 3. 批量操作

**批量导入**：
1. 点击"批量导入"按钮（多个方块图标）
2. 准备账号列表文件，格式：
   ```
   邮箱 密码 备注
   user1@example.com password123 账号1
   user2@example.com password456 账号2
   ```
3. 选择文件并导入
4. 确认导入信息

**批量删除**：
1. 勾选要删除的账号卡片
2. 点击"批量删除"按钮（垃圾桶图标）
3. 确认删除操作

**批量刷新**：
1. 勾选要刷新的账号卡片
2. 点击"批量刷新状态"按钮
3. 系统会批量更新账号的积分、套餐等信息

**导出账号**：
1. 点击"导出账号"按钮（下载图标）
2. 选择导出格式（CSV/JSON/文本）
3. 文件会自动下载到本地

### 4. 账号分组

**创建分组**：
1. 点击侧边栏的"分组"菜单
2. 点击"添加分组"按钮
3. 输入分组名称和颜色
4. 保存分组

**管理分组**：
1. 在分组列表中可以编辑或删除分组
2. 在添加/编辑账号时选择分组
3. 分组可以帮助你更好地管理大量账号

### 5. 标签管理

**添加标签**：
1. 在添加/编辑账号时点击"标签"输入框
2. 输入标签名称
3. 选择标签颜色
4. 保存标签

**管理标签**：
1. 点击设置中的"标签管理"
2. 可以编辑或删除已有标签
3. 标签可以帮助你快速筛选账号

### 6. 团队管理

**查看团队成员**：
1. 点击账号卡片上的"团队管理"按钮
2. 查看团队内所有成员信息
3. 查看成员的积分使用情况

**邀请成员**：
1. 在团队管理对话框中点击"邀请成员"
2. 输入成员邮箱
3. 设置成员权限
4. 发送邀请

**移除成员**：
1. 在成员列表中选择要移除的成员
2. 点击"移除成员"按钮
3. 确认移除操作

### 7. 一键换号

**标准换号**：
1. 点击账号卡片上的"切号"按钮（Switch 图标）
2. 系统会自动：
   - 使用 refresh_token 获取新的 access_token
   - 调用 RegisterUser API 获取 api_key
   - 加密 sessions 数据并写入 Windsurf 数据库
   - 更新 Windsurf 配置文件
   - 重置机器 ID
   - 自动重启 Windsurf
3. 等待 Windsurf 重启完成

**无感换号（需要补丁）**：
1. 点击"无感换号补丁"按钮
2. 系统会自动检测 Windsurf 安装路径
3. 点击"应用补丁"修改 extension.js
4. 补丁会移除 180 秒超时限制
5. 应用补丁后重启 Windsurf
6. 之后可以随时点击"无感切换"按钮快速换号

### 8. 查看日志

1. 点击侧边栏的"操作日志"
2. 查看所有操作记录
3. 可以按类型、时间筛选日志
4. 支持清空或导出日志

### 9. 账单查询

**查看账单信息**：
1. 点击账号卡片上的"账单"按钮
2. 查看订阅类型、到期时间
3. 查看支付方式、下次扣费日期
4. 查看发票链接

**更新套餐**：
1. 点击"更新套餐"按钮
2. 选择新的套餐类型（Teams/Pro）
3. 选择计费周期（月付/年付）
4. 确认更新

### 10. 使用分析

**查看使用统计**：
1. 点击账号卡片上的"分析"按钮
2. 查看每日活跃统计
3. 查看工具使用情况
4. 查看模型使用情况
5. 查看 Token 消耗统计

## 数据存储

### 应用数据存储

应用数据存储在本地：
- **Windows**: `%APPDATA%\com.chao.windsurf-account-manager\accounts.json`

数据结构包括：
- 账号信息（加密的密码和Token）
- 分组列表
- 系统设置
- 操作日志

### Windsurf 数据存储架构

Windsurf 使用三层存储架构管理登录凭证：

**1. 加密层**
- **存储位置**: `secret://windsurf_auth.sessions`
- **加密方式**: Windows DPAPI 加密
- **数据格式**: Buffer 字节数组
- **作用**: 存储会话数据和 API Key

**2. 状态层**
- **存储位置**: `windsurfAuthStatus`
- **数据格式**: 明文 JSON
- **包含信息**: API Key (sk-ws-01-前缀)、用户信息、团队配置
- **作用**: 存储认证状态和用户基本信息

**3. 明文层**
- **存储位置**: `codeium.windsurf-windsurf_auth` 和 `codeium.windsurf`
- **数据格式**: 明文
- **包含信息**: 用户名、基础配置
- **作用**: 存储用户标识和配置信息

### 关键文件路径

**数据库文件**:
- **路径**: `%APPDATA%\Windsurf\User\globalStorage\state.vscdb`
- **格式**: SQLite
- **表名**: ItemTable (键值对存储)

**配置文件**:
- **路径**: `%APPDATA%\Windsurf\User\globalStorage\storage.json`
- **格式**: JSON
- **包含信息**: 机器 ID、设备标识等

### 安全机制

- **Windows Credential Manager 集成**: 加密密钥存储在系统密钥链
- **DPAPI 加密**: 与用户账户绑定，只有当前用户可以解密
- **Protobuf 编码**: 权限配置使用 Protobuf 编码
- **重要提示**: API Key 在 windsurfAuthStatus 中明文存储，保护数据库文件至关重要

## 安全说明

1. **密码安全**：所有密码使用AES-256-GCM加密
2. **密钥管理**：加密密钥存储在系统密钥链中
3. **Token刷新**：Token过期前5分钟自动刷新
4. **本地存储**：所有数据仅存储在本地，不会上传到任何服务器

## 注意事项

1. 请妥善保管你的账号信息
2. 定期备份 `accounts.json` 文件
3. 批量操作时注意API限流
4. 建议使用分组功能管理多个账号

## 开发说明

### 项目结构

```
windsurf-account-manager-simple/
├── src/                           # Vue前端源码
│   ├── views/                     # 页面组件
│   │   └── MainLayout.vue         # 主布局页面（包含所有功能）
│   ├── components/                # 可复用组件
│   │   ├── AboutDialog.vue        # 关于对话框
│   │   ├── AccountCard.vue        # 账号卡片组件
│   │   ├── AccountInfoDialog.vue  # 账号详细信息对话框
│   │   ├── AddAccountDialog.vue   # 添加账号对话框
│   │   ├── AnalyticsDialog.vue    # 使用分析对话框
│   │   ├── AutoRefillDialog.vue   # 自动充值对话框
│   │   ├── AutoResetDialog.vue    # 自动重置对话框
│   │   ├── BatchImportDialog.vue  # 批量导入对话框
│   │   ├── BatchUpdatePlanDialog.vue  # 批量更新套餐对话框
│   │   ├── BillingDialog.vue      # 账单信息对话框
│   │   ├── CardGeneratorDialog.vue   # 虚拟卡生成对话框
│   │   ├── CreditHistoryDialog.vue    # 积分历史对话框
│   │   ├── EditAccountDialog.vue  # 编辑账号对话框
│   │   ├── LogsDialog.vue        # 操作日志对话框
│   │   ├── SettingsDialog.vue    # 设置对话框
│   │   ├── StatsDialog.vue       # 统计信息对话框
│   │   ├── TagColorPicker.vue    # 标签颜色选择器
│   │   ├── TagManageDialog.vue   # 标签管理对话框
│   │   ├── TeamManagementDialog.vue  # 团队管理对话框
│   │   ├── TeamSettingsDialog.vue    # 团队设置对话框
│   │   ├── TurnstileDialog.vue   # Turnstile验证对话框
│   │   ├── UpdatePlanDialog.vue  # 更新套餐对话框
│   │   ├── UpdateSeatsResultDialog.vue  # 座位更新结果对话框
│   │   └── WelcomeDialog.vue     # 欢迎对话框
│   ├── api/                       # API封装层
│   ├── services/                  # 业务服务层
│   ├── store/                     # Pinia状态管理
│   ├── types/                     # TypeScript类型定义
│   ├── utils/                     # 工具函数
│   ├── assets/                    # 静态资源
│   ├── App.vue                    # 根组件
│   └── main.ts                    # 应用入口
├── src-tauri/                     # Rust后端源码
│   ├── src/
│   │   ├── commands/              # Tauri命令层（前后端通信接口）
│   │   │   ├── account_commands.rs    # 账号管理命令
│   │   │   ├── analytics_commands.rs  # 分析数据命令
│   │   │   ├── api_commands.rs        # API调用命令
│   │   │   ├── auto_reset_commands.rs # 自动重置命令
│   │   │   ├── cunzhi_commands.rs     # 寸止功能命令
│   │   │   ├── devin_commands.rs      # Devin账号命令
│   │   │   ├── payment_commands.rs    # 支付相关命令
│   │   │   ├── patch_commands.rs      # 补丁应用命令
│   │   │   ├── proto_commands.rs      # Protobuf解析命令
│   │   │   ├── settings_commands.rs   # 设置相关命令
│   │   │   ├── switch_account_commands.rs  # 切换账号命令
│   │   │   ├── team_commands.rs       # 团队管理命令
│   │   │   ├── app_info.rs            # 应用信息命令
│   │   │   ├── validate_path.rs       # 路径验证命令
│   │   │   ├── windsurf_info.rs       # Windsurf信息命令
│   │   │   └── mod.rs                 # 命令模块入口
│   │   ├── models/               # 数据模型
│   │   │   ├── account.rs       # 账号模型
│   │   │   ├── group.rs         # 分组模型
│   │   │   ├── operation_log.rs # 操作日志模型
│   │   │   └── settings.rs      # 设置模型
│   │   ├── repository/           # 数据访问层
│   │   │   └── data_store.rs    # 数据存储实现
│   │   ├── services/             # 业务逻辑层
│   │   │   ├── auth_service.rs  # 认证服务
│   │   │   └── windsurf_service.rs  # Windsurf API服务
│   │   ├── utils/                # 工具函数
│   │   │   ├── crypto.rs        # 加密工具
│   │   │   ├── proto_parser.rs  # Protobuf解析器
│   │   │   └── error.rs         # 错误处理
│   │   ├── lib.rs               # Rust库入口
│   │   └── main.rs              # Tauri应用入口
│   ├── Cargo.toml               # Rust依赖配置
│   ├── tauri.conf.json          # Tauri配置文件
│   └── build.rs                 # 构建脚本
├── public/                      # 公共资源
│   ├── 交流群.png               # 微信群二维码
│   ├── QQ群.jpg                 # QQ群二维码
│   └── 主页.png                 # 界面截图
├── scripts/                     # 脚本工具
│   └── sync-version.js          # 版本同步脚本
├── docs/                        # 文档目录
├── package.json                 # Node依赖配置
├── vite.config.ts               # Vite配置
├── tsconfig.json                # TypeScript配置
├── tsconfig.node.json           # Node环境TypeScript配置
├── build_with_admin.bat         # 管理员权限构建脚本
├── set_admin_manifest.ps1       # 设置管理员清单脚本
└── README.md                    # 项目文档
```

### API集成

应用集成了以下 Windsurf API：

#### Firebase 认证 API
- **端点**: `https://identitytoolkit.googleapis.com/v1/accounts:signInWithPassword`
- **端点**: `https://securetoken.googleapis.com/v1/token`
- **功能**: 用户登录、Token 刷新
- **认证方式**: API Key + Referer 头
- **重要**: 必须携带 `Referer: https://windsurf.com/` 头，否则返回 403 错误（`API_KEY_HTTP_REFERRER_BLOCKED`）
- **建议**: 同时添加 `X-Client-Version: Chrome/JsCore/11.0.0/FirebaseCore-web` 头

#### 座位管理 API
- **UpdateSeats**: 更新团队席位数量
  - 端点: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/UpdateSeats`
  - 功能: 通过修改席位数量实现积分重置
  - 协议: Connect-Web (gRPC-Web)
  - 请求格式: Protobuf

- **GetPlanStatus**: 获取账号套餐状态
  - 端点: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetPlanStatus`
  - 功能: 获取积分余额、配额信息、计费策略
  - 响应字段: available_flex_credits, available_prompt_credits, used_flex_credits, used_prompt_credits, billing_strategy, daily_quota, weekly_quota

#### 账单查询 API
- **GetTeamBilling**: 获取团队账单信息
  - 端点: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetTeamBilling`
  - 功能: 查询订阅状态、支付方式、计费周期
  - 响应字段: 订阅类型、席位数量、单价、支付方式、下次扣费日期

#### 用户管理 API
- **GetCurrentUser**: 获取当前用户信息
  - 端点: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetCurrentUser`
  - 功能: 获取用户基本信息、团队信息、套餐信息、权限信息
  - 响应字段: UserBasicInfo, TeamInfo, PlanInfo, UserRole

- **GetUsers**: 获取团队成员列表
  - 端点: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetUsers`
  - 功能: 批量获取团队成员信息
  - 响应字段: users[], user_roles[], user_team_details[], user_cascade_details[]

- **RegisterUser**: 注册用户获取 API Key
  - 端点: `https://register.windsurf.com/exa.seat_management_pb.SeatManagementService/RegisterUser`
  - 功能: 使用 Firebase Token 注册获取 sk-ws-01 格式 API Key
  - 响应字段: api_key, name, api_server_url

**API Key 格式说明**：
- **格式**: sk-ws-01-[88-94字符Base64编码]
- **总长度**: 103字符
- **结构**: sk(Secret Key) + ws(WindSurf) + 01(版本) + Base64编码payload
- **字符分布**: 大写35%、小写45%、数字17%、连字符4%
- **随机性**: 96.7%（接近最大熵，安全性高）
- **编码**: Base64编码，解码后32字节二进制数据
- **获取流程**: Firebase 登录获取 ID Token → 调用 RegisterUser API → 返回 sk-ws-01 格式 API Key
- **重要**: RegisterUser API 既支持 register.windsurf.com 也支持 web-backend.windsurf.com

#### 团队管理 API
- **InviteTeamMember**: 邀请团队成员
- **RemoveTeamMember**: 移除团队成员
- **UpdateTeamSettings**: 更新团队设置

#### 分析数据 API
- **GetAnalytics**: 获取使用分析数据
  - 端点: `https://web-backend.windsurf.com/exa.user_analytics_pb.UserAnalyticsService/GetAnalytics`
  - 功能: 获取每日活跃统计、工具使用、模型使用、Token 消耗
  - 响应字段: cascade_lines, cascade_tool_usage, cascade_runs, daily_active_user_counts

#### 积分历史 API
- **GetTeamCreditEntries**: 获取积分获取历史
  - 端点: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetTeamCreditEntries`
  - 功能: 查询积分获取记录（推荐奖励、购买等）
  - 响应字段: FlexCreditChronicalEntry（团队ID、授予时间、积分数量、类型、原因）

#### 推荐码 API
- **ProcessReferralCode**: 处理推荐码
  - 端点: `https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/ProcessReferralCode`
  - 功能: 使用推荐码获取 25,000 PROMPT 积分奖励
  - 请求字段: auth_token, referral_code

#### Devin API
- **auth1_token 鉴权**: Devin 专属认证机制
- **session_token 刷新**: 自动刷新 session_token
- **5-header 认证**: 完整的认证头设置

---

### 请求头格式说明

Windsurf API 调用需要携带完整的浏览器相关请求头：

**必需的请求头**：
- `connect-protocol-version: "1"` - Connect-Web 协议版本
- `x-debug-email: ""` - 调试邮箱（空字符串）
- `x-debug-team-name: ""` - 调试团队名称（空字符串）
- `Referer: https://windsurf.com/` - Referer 头（Firebase API 必需）
- `X-Client-Version: Chrome/JsCore/11.0.0/FirebaseCore-web` - 客户端版本（建议）

**浏览器标识头**：
- `sec-ch-ua` - 用户代理提示
- `sec-ch-ua-mobile` - 移动设备提示
- `sec-ch-ua-platform` - 平台提示
- `User-Agent` - 完整的用户代理字符串

### 请求体格式

**UpdateSeats 请求体**：
- 前缀: `0x0a, 0xa1, 0x07` (固定协议标识)
- Token: 认证 Token
- 座位字节: `0x10, seat_count`

**GetTeamBilling 请求体**：
- 前缀: `0x0a, 0xa1, 0x07` (固定协议标识)
- Token: 认证 Token

**注意**: 前缀 `0xa1 0x07` 是固定的协议标识，不要修改

---

### 架构设计

#### 整体架构
应用采用 **Tauri 桌面应用架构**，前后端分离设计：

```
┌─────────────────────────────────────────────────────────┐
│                     前端 (Vue 3)                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │  UI 组件层    │  │  状态管理层   │  │  API 调用层  │ │
│  │  Components  │  │    Pinia     │  │   Axios/Tauri │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────┘
                           │
                    Tauri IPC
                           │
┌─────────────────────────────────────────────────────────┐
│                    后端 (Rust)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │  命令层       │  │  服务层       │  │  数据层       │ │
│  │  Commands    │  │   Services   │  │  Repository  │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │  模型层       │  │  工具层       │  │  加密层       │ │
│  │   Models     │  │    Utils     │  │   Crypto     │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────┘
```

#### 分层设计

**1. 前端分层**
- **UI 组件层**: 可复用的 Vue 组件（对话框、卡片、表单等）
- **状态管理层**: Pinia Store 管理应用状态
- **API 调用层**: 封装 Tauri IPC 调用，与后端通信

**2. 后端分层**
- **命令层 (Commands)**: Tauri 命令接口，处理前端请求
- **服务层 (Services)**: 业务逻辑处理（认证、API 调用等）
- **数据层 (Repository)**: 数据持久化（JSON 文件、密钥链）
- **模型层 (Models)**: 数据结构定义
- **工具层 (Utils)**: 通用工具函数（加密、解析等）

#### 数据流

```
用户操作 → 前端组件 → Pinia Store → Tauri Command → Rust Service → API 调用
                                                                    ↓
响应 ← 前端组件 ← Pinia Store ← Tauri Command ← Rust Service ← Windsurf API
```

#### 安全架构

**1. 加密存储**
- 账号密码: AES-256-GCM 加密
- 加密密钥: Windows Credential Manager 存储
- Token: 内存缓存 + 自动刷新机制

**2. 认证流程**
- Firebase 认证获取 ID Token
- ID Token 刷新过期 Token
- API 调用携带完整认证头

**3. 隐私保护**
- 所有数据本地存储
- 不上传任何信息到服务器
- 支付窗口使用无痕模式

#### 版本管理

采用 **单一真相源** 策略：
- `src-tauri/tauri.conf.json` 中的 `version` 字段为唯一真相源
- 前端通过 `get_app_version` API 动态获取版本号
- `build.rs` 构建时自动读取 `tauri.conf.json` 生成 Windows manifest
- 使用 `node scripts/sync-version.js` 同步 `package.json` 和 `Cargo.toml`

#### 错误处理

**1. 前端错误处理**
- Try-catch 捕获异步错误
- ElMessage 显示错误提示
- 错误日志记录到操作日志

**2. 后端错误处理**
- Anyhow 统一错误处理
- Result<T, String> 返回类型
- 自定义错误类型 (AppError)
- Connect Protocol 错误码映射

**3. API 错误处理**
- 401 错误触发 Token 刷新
- 网络错误自动重试
- 超时错误提示用户

#### 性能优化

**1. 前端优化**
- 虚拟滚动处理大量账号
- 防抖/节流控制请求频率
- 组件懒加载
- Pinia 持久化缓存

**2. 后端优化**
- 异步并发处理批量操作
- Token 复用避免重复登录
- SQLite 连接池
- Protobuf 高效序列化

**3. 网络优化**
- 轻量 API 模式（GetPlanStatus 替代 GetCurrentUser）
- 请求头缓存
- HTTP/2 支持
- 代理支持

#### 备份文件管理策略

**无感换号补丁备份**：
- **最多保留 3 份备份**：防止备份文件无限累积
- **自动清理旧备份**：当备份数量达到或超过 3 个时，自动删除最早的备份文件
- **按时间排序**：通过文件修改时间排序，确保删除的是最旧的备份
- **备份文件命名格式**：`extension.js.backup.YYYYMMDD_HHMMSS`
- **示例**：`extension.js.backup.20251122_192115`

**实现逻辑**：
```rust
// 查找所有备份文件（以 "extension.js.backup." 开头）
// 按修改时间排序
// 删除超出数量的旧备份（保留最新的 3 份）
```

---

#### UpdateSeats API 响应数据说明

座位更新API返回的解析后JSON包含：
- `success`: 操作是否成功
- `total_seats`: 总座位数（对应int_4字段）
- `used_seats`: 已使用座位数（对应int_5字段）
- `seat_usage`: 座位使用情况（如："1 / 5"）
- `seat_usage_percentage`: 座位使用率百分比（计算得出）
- `price_per_seat`: 每座位月费（对应float_3字段，单位：美元）
- `total_monthly_price`: 总月费（对应float_6字段，单位：美元）
- `billing_start_time`: 当前计费周期开始时间（对应subMesssage_7.int_1时间戳）
- `next_billing_time`: 下次计费时间（对应subMesssage_8.int_1时间戳）
- `billing_start_timestamp`: 计费开始时间戳（Unix时间戳）
- `next_billing_timestamp`: 下次计费时间戳（Unix时间戳）

**Protobuf字段映射**：
- `float_3`: 每座位月费价格（如：60 = $60/月/座位）
- `int_4`: 总座位数（如：5）
- `int_5`: 已使用座位数（如：1）
- `float_6`: 总月费（如：300 = $300/月）
- `subMesssage_7.int_1`: 当前计费周期开始时间戳
- `subMesssage_8.int_1`: 下次计费时间戳

#### GetTeamBilling API 响应数据说明

账单查询API返回的解析后JSON包含：
- `plan_name`: 套餐名称
- `base_quota`: 套餐基础额度（对应int_8字段）
- `extra_credits`: 额外积分（对应int_4字段，可选）
- `total_quota`: 总额度（base_quota + extra_credits）
- `used_quota`: 已使用额度（对应int_6字段，可选，默认0）
- `cache_limit`: 缓存限额（对应int_9字段）
- `payment_method`: 支付方式信息
- `next_billing_date`: 下次扣费日期
- `invoice_url`: 发票链接
- `monthly_price`: 月费价格

**Protobuf字段映射**：
- `int_4`: 额外积分（赠送或购买的额外额度，可能不存在）
- `int_6`: 使用积分（已使用额度，可能不存在）
- `int_8`: 套餐额度（基础套餐额度）
- `int_9`: 套餐缓存限额（超过此限额将无法使用）

**注意**：当缓存使用率达到缓存限额时，即使总额度还有剩余也将无法继续使用。

## 常见问题

### Q: 为什么积分重置失败？
A: 请检查以下事项：
1. **账号密码是否正确** - 确认输入的邮箱和密码无误
2. **网络连接是否正常** - 检查网络连接，必要时配置代理
3. **Token是否过期** - 尝试重新登录刷新 Token
4. **账号是否为团队管理员** - 只有团队管理员才能执行座位更新
5. **API 限流** - 批量操作时注意控制并发数量，避免触发限流

### Q: 如何备份账号数据？
A: 复制以下文件到安全位置：
- **Windows**: `%APPDATA%\com.chao.windsurf-account-manager\accounts.json`
- **注意**: accounts.json 包含加密的敏感信息，备份时请妥善保管

### Q: 忘记了账号密码怎么办？
A: 应用中存储的密码是 AES-256-GCM 加密的，无法查看明文。需要：
1. 在 Windsurf 官网重置密码
2. 在应用中删除旧账号
3. 重新添加账号

### Q: 遇到 "Decryption error: Encoded text cannot have a 6-bit remainder" 错误怎么办？
A: 这是密钥链中的加密密钥已改变或损坏导致的。解决步骤：
1. **删除旧的账号数据文件**：
   ```powershell
   Remove-Item "$env:APPDATA\com.chao.windsurf-account-manager\accounts.json" -Force
   ```
2. **清理 Windows 密钥链中的旧密钥**：
   ```powershell
   # 查找密钥
   cmdkey /list | Select-String "WindsurfAccountManager"
   # 删除密钥
   cmdkey /delete:LegacyGeneric:target=MasterKey.WindsurfAccountManager
   ```
3. **重启应用** - 应用会自动生成新的加密密钥

### Q: 如何配置代理？
A: 在设置中配置 HTTP 代理：
1. 点击侧边栏的"设置"
2. 找到"代理配置"部分
3. 输入代理地址和端口
4. 点击"测试连接"验证代理是否可用
5. 保存设置

### Q: 遇到 "API_KEY_HTTP_REFERRER_BLOCKED" 错误怎么办？
A: 这是 Google Firebase 项目新增的 HTTP Referer 限制导致的：
1. **错误原因**: 所有调用 identitytoolkit.googleapis.com 和 securetoken.googleapis.com 的请求必须携带 `Referer: https://windsurf.com/` 头
2. **解决方案**: 应用已在所有 Firebase API 调用中添加 Referer 头
3. **如果仍然遇到错误**: 请确保使用最新版本的应用（v1.6.6+）
4. **临时解决**: 检查网络环境，某些代理可能会修改 Referer 头

### Q: 什么是轻量 API 模式？
A: 轻量 API 模式使用 `GetPlanStatus` API 替代 `GetCurrentUser` API：
- **优点**: 减少请求数量，提高响应速度
- **缺点**: 获取的信息较少（只有积分和套餐状态）
- **适用场景**: 只需要查看积分余额时
- **开启方式**: 在设置中勾选"轻量 API 模式"

### Q: 一键换号后 Windsurf 无法启动怎么办？
A: 可能的原因和解决方法：
1. **机器 ID 重置失败** - 手动重置 Windows 注册表中的机器 ID
2. **sessions 数据损坏** - 删除 `%APPDATA%\Windsurf\User\globalStorage\state.vscdb` 文件
3. **配置文件错误** - 删除 `%APPDATA%\Windsurf\User\globalStorage\storage.json` 文件
4. **重新登录** - 删除数据后重新登录 Windsurf

### Q: 无感换号补丁安全吗？
A: 补丁的安全性说明：
1. **只修改 extension.js** - 仅移除 180 秒超时限制
2. **自动备份** - 打补丁前自动备份原文件（最多保留 3 份）
3. **可还原** - 随时可以点击"还原补丁"恢复原始状态
4. **开源透明** - 所有代码开源，可以自行审查

### Q: 如何查看详细的 API 响应？
A: 在设置中开启"显示详细结果"：
1. 点击侧边栏的"设置"
2. 勾选"显示详细结果"
3. 执行操作时会显示完整的 API 响应信息
4. 便于调试和问题排查

### Q: 批量操作时为什么会失败？
A: 可能的原因：
1. **并发限制** - 批量重置最多支持 5 个并发，超过会失败
2. **部分账号失效** - 某些账号密码错误或已被封禁
3. **网络不稳定** - 网络波动导致部分请求失败
4. **API 限流** - 短时间内大量请求触发限流
5. **解决方法**: 查看操作日志了解具体失败原因，分批执行

### Q: Devin 账号和普通账号有什么区别？
A: Devin 账号体系的特点：
1. **认证方式** - 使用 auth1_token 和 session_token
2. **Token 管理** - session_token 没有显式过期时间，使用 32 天占位机制
3. **刷新机制** - 基于 401 错误触发强制刷新
4. **多组织支持** - 支持多组织账号管理
5. **关键字段** - 自动补充 product 参数等关键字段

### Q: 如何使用推荐码获取积分？
A: 使用推荐码的步骤：
1. 在账号信息对话框中找到"推荐码"字段
2. 点击旁边的链接图标复制推荐链接
3. 分享链接给好友注册
4. 好友注册后双方各获得 25,000 PROMPT 积分

### Q: 支持哪些 Windsurf 版本？
A: 当前支持的版本：
1. **Windsurf 标准版** - 完整支持官方 Windsurf 客户端
2. **Windsurf - Next** - 支持 Next 版本的独立配置与管理
3. **自动检测** - 应用会自动识别并适配不同版本

### Q: 如何迁移账号数据到其他电脑？
A: 迁移步骤：
1. 备份原电脑的 `accounts.json` 文件
2. 备份 Windows 密钥链中的密钥（使用 `cmdkey /list`）
3. 在新电脑上安装应用
4. 恢复 `accounts.json` 文件
5. 恢复密钥链密钥（使用 `cmdkey /add`）
6. 启动应用验证

### Q: 应用会收集我的数据吗？
A: 数据隐私说明：
1. **本地存储** - 所有数据仅存储在本地
2. **不上传** - 不上传任何信息到服务器
3. **开源透明** - 代码完全开源，可以自行审查
4. **加密保护** - 敏感信息使用 AES-256-GCM 加密

### Q: 如何更新应用？
A: 更新方法：
1. 访问项目 GitHub Releases 页面
2. 下载最新版本的安装包
3. 卸载旧版本（可选）
4. 安装新版本
5. 数据会自动保留

### Q: 遇到问题如何反馈？
A: 反馈渠道：
1. **GitHub Issues** - 在项目 GitHub 页面提交 Issue
2. **交流群** - 扫描 README 中的二维码加入交流群
3. **邮件** - 发送邮件给开发者
4. 反馈时请提供：错误信息、操作步骤、系统版本

## 贡献指南

欢迎提交 Issue 和 Pull Request！

### 开发流程

1. **Fork 项目** - 点击 [GitHub 页面](https://github.com/chaogei/windsurf-account-manager-simple)右上角的 Fork 按钮
2. **克隆仓库** - `git clone https://github.com/your-username/windsurf-account-manager-simple.git`
3. **创建分支** - `git checkout -b feature/your-feature-name`
4. **安装依赖** - `npm install`
5. **开发调试** - `npm run tauri dev`
6. **提交更改** - `git commit -m "Add your feature"`
7. **推送分支** - `git push origin feature/your-feature-name`
8. **创建 PR** - 在 GitHub 页面创建 Pull Request

### 代码规范

**前端代码**：
- 使用 TypeScript 严格模式
- 遵循 Vue 3 Composition API 风格
- 组件命名使用 PascalCase
- 函数命名使用 camelCase
- 常量命名使用 UPPER_SNAKE_CASE

**后端代码**：
- 遵循 Rust 官方代码风格
- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码
- 函数命名使用 snake_case
- 类型命名使用 PascalCase

### 提交信息规范

提交信息格式：
```
<type>(<scope>): <subject>

<body>

<footer>
```

类型（type）：
- `feat`: 新功能
- `fix`: 修复 bug
- `docs`: 文档更新
- `style`: 代码格式调整
- `refactor`: 重构
- `test`: 测试相关
- `chore`: 构建/工具相关

示例：
```
feat(api): add GetTeamCreditEntries API support

- Add GetTeamCreditEntries command
- Add credit history dialog component
- Update API integration documentation

Closes #123
```

### Tauri ACL 权限配置

Tauri v2 使用 ACL (Access Control List) 系统管理权限，每个插件命令都需要在 capabilities 中明确授权。

**权限配置文件**：`src-tauri/capabilities/default.json`

**常用权限示例**：
```json
{
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:default",
    "dialog:allow-open",      // 允许打开文件/文件夹选择对话框
    "dialog:allow-save",      // 允许保存文件对话框
    "dialog:allow-message",   // 允许显示消息对话框
    "dialog:allow-ask",       // 允许询问对话框
    "dialog:allow-confirm"    // 允许确认对话框
  ]
}
```

**权限命名格式**：
- `plugin:command` - 插件命令
- `plugin:allow-command` - 允许执行的插件命令

**注意事项**：
- 如果遇到 `Command plugin:dialog|open not allowed by ACL` 错误，需要在 capabilities 中添加相应权限
- 权限名称格式必须严格遵循规范

### 版本发布流程

1. 更新 `src-tauri/tauri.conf.json` 中的版本号
2. 运行 `node scripts/sync-version.js` 同步其他文件
3. 更新 README.md 中的版本历史
4. 提交更改并打标签：
   ```bash
   git add .
   git commit -m "release: version x.x.x"
   git tag -a vx.x.x -m "Release version x.x.x"
   git push origin main
   git push origin vx.x.x
   ```
5. 在 GitHub 创建 Release

## 许可证

AGPL-3.0

## 免责声明

本工具仅供学习和个人使用，请遵守 Windsurf 服务条款。作者不对因使用本工具产生的任何问题负责。

---

## 项目总结

Windsurf Account Manager - Simple 是一个功能完善的 Windsurf 多账号管理桌面应用，采用 Tauri + Vue 3 + Rust 技术栈开发。项目自 2025 年底开始开发，经过多个版本的迭代，已经实现了以下核心功能：

### 核心功能
- ✅ 完整的账号管理（CRUD、分组、标签）
- ✅ 积分重置（单账号、批量、自动）
- ✅ 团队管理（成员管理、邀请、移除）
- ✅ 一键换号（标准换号、无感换号补丁）
- ✅ 账单查询（订阅状态、支付信息）
- ✅ 使用分析（每日统计、工具使用、模型使用）
- ✅ 订阅支付（虚拟卡生成、隐私支付）
- ✅ 数据安全（AES-256-GCM 加密、系统密钥链）

### 技术亮点
- 🏗️ 清晰的分层架构（命令层、服务层、数据层）
- 🔒 完善的安全机制（加密存储、认证流程、隐私保护）
- ⚡ 高性能优化（异步并发、Token 复用、轻量 API）
- 🎯 多客户端支持（Windsurf 标准版、Windsurf - Next）
- 🤖 Devin 账号体系完整支持（auth1_token、session_token）

### 版本演进
- **v1.6.x**: 基础功能完善、订阅支付、分析数据优化
- **v1.7.0**: 计费策略与配额详情支持、OperationType 扩展
- **v1.7.1**: 多客户端支持（Windsurf 和 Windsurf - Next）
- **v1.7.2**: Devin 账号体系完整支持
- **v1.7.3**: Devin 刷新逻辑修复、关键字段回填

### 未来计划
- [ ] 支持 Linux 和 macOS 平台
- [ ] 添加更多数据分析图表
- [ ] 支持自定义 API 端点
- [ ] 添加自动化测试
- [ ] 优化 UI/UX 设计

---

## 致谢

感谢以下开源项目和服务：

- [Tauri](https://tauri.app/) - 现代化的桌面应用开发框架
- [Vue.js](https://vuejs.org/) - 渐进式 JavaScript 框架
- [Element Plus](https://element-plus.org/) - 基于 Vue 3 的组件库
- [Rust](https://www.rust-lang.org/) - 系统编程语言
- [Reqwest](https://docs.rs/reqwest/) - Rust HTTP 客户端
- [Windsurf](https://windsurf.com/) - AI 编程助手

感谢所有贡献者和用户的反馈与支持！

---

## 联系方式

- **GitHub**: [[项目地址](https://github.com/chaogei/windsurf-account-manager-simple)]
- **作者**: chaogei666
- **许可证**: AGPL-3.0

---

**最后更新**: 2026-04-20
**文档版本**: 1.7.3

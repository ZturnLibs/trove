# 一站式个人工作台技术设计

## 1. 文档状态

- 状态：技术方案基线。
- 适用范围：`v1.0` 及之后的桌面版本。
- UI 技术方向：Web 跨端，不采用原生 UI 作为主方案。
- 首批目标平台：macOS、Windows。
- 后续目标平台：Linux，在核心功能完成后适配和验收。

本文定义工程边界和关键技术决策。功能范围仍以[第一版开发规划](./development-roadmap.md)和[后续迭代详细设计](./post-v1-iteration-design.md)为准。

## 2. 核心技术决策

| 编号 | 决策 | 结论 |
| --- | --- | --- |
| TD-001 | 桌面容器 | Tauri 2 |
| TD-002 | 前端 | React、TypeScript、Vite、Tailwind CSS 4 |
| TD-003 | 本地后端 | Rust，业务逻辑不放在 WebView 中 |
| TD-004 | 核心存储 | SQLite，Rust 独占业务写入 |
| TD-005 | 资源存储 | SQLite 元数据加应用资源目录 |
| TD-006 | 搜索 | SQLite FTS5，中文短词回退查询 |
| TD-007 | 前后端通信 | 类型化 Tauri Command 加无正文事件 |
| TD-008 | 系统能力 | Tauri 官方插件加平台适配层 |
| TD-009 | 浏览器存储 | IndexedDB 和 Local Storage 不保存核心数据 |
| TD-010 | 同步 | `v1.x` 不同步运行中的 SQLite 文件 |
| TD-011 | 测试 | Rust、React 分层测试加 WebdriverIO 桌面端到端测试 |
| TD-012 | JavaScript 包管理 | pnpm，提交锁文件 |
| TD-013 | 样式体系 | Tailwind CSS 4、CSS-first 主题变量、Radix UI 基础组件 |

数据库加密在阶段 0 完成技术验证后锁定。未锁定前，产品不得宣称数据已经进行应用级加密。

## 3. 设计目标

技术方案需要保证：

- 任务、提醒、记忆和剪切板在断网时完整可用。
- 关闭主窗口后，提醒和剪切板监听继续运行。
- 主窗口、快捷窗口和托盘操作保持数据一致。
- 数据迁移、备份和异常退出不会造成静默丢失。
- 系统权限被拒绝时提供明确降级能力。
- macOS 和 Windows 共用业务实现，平台差异集中在适配层。
- 后续增加图片、自动化或 AI 时，不需要重写核心存储和业务层。

不以以下能力为当前设计目标：

- 纯浏览器或 PWA 独立运行全部功能。
- 服务端数据库和用户账号。
- 移动端代码复用最大化。
- 多人实时协作和 CRDT。
- 通过网盘直接同步数据库文件。
- 在 WebView 中直接执行 SQL、Shell 或任意文件访问。

## 4. 总体架构

```text
React Web UI
    |
Typed IPC Commands / Tauri Events
    |
Rust Application Layer
    |
Domain Services
Task / Reminder / Memory / Clipboard / Search
    |
Infrastructure
SQLite / Assets / Backup / Notification / Credential Store
    |
Platform Adapters
macOS / Windows / Linux
```

依赖只能自上而下：

- 前端依赖 IPC 契约，不了解数据库结构。
- Application 层组织用例和事务。
- Domain 层包含规则，不依赖 Tauri、React 或 SQLite。
- Infrastructure 层实现 Repository 和系统接口。
- Platform 层封装操作系统差异。

## 5. 技术栈

### 5.1 桌面容器

- Tauri 2。
- Tauri Core Tray。
- 官方插件优先：notification、clipboard-manager、global-shortcut、autostart、single-instance、opener、window-state、updater。
- 只有官方插件不能满足需求时，才增加社区插件或自研插件。
- 社区插件进入项目前必须检查维护状态、权限范围、许可证和平台实现。

### 5.2 前端

- React。
- TypeScript 严格模式。
- Vite。
- pnpm。
- TanStack Query：管理 IPC 查询、缓存、刷新和错误状态。
- Zustand：只管理窗口级、选择状态等临时 UI 状态。
- React Hook Form 与 Zod：表单和前端输入反馈。
- Tailwind CSS 4 与官方 Vite 插件：唯一的应用样式构建方案。
- Radix UI：无障碍交互基础，不作为视觉主题来源。
- class-variance-authority、clsx、tailwind-merge：组件变体和类名合并。
- Lucide：图标。

不引入 Redux、Sass、Less 或 Stylus。数据库数据不复制成长期前端状态树。

### 5.3 Rust

- Tauri Command：前端调用入口。
- Tokio：后台任务、计时和异步协调。
- Serde：IPC DTO 和版本化导入导出。
- rusqlite 或兼容封装：SQLite 访问、迁移、FTS5 和备份。
- tracing：结构化日志，默认脱敏。
- thiserror：领域和基础设施错误定义。

选择 rusqlite 而不是把 `tauri-plugin-sql` 暴露给前端，原因包括：

- 所有 SQL 和事务留在可信 Rust 进程。
- 可以直接控制 SQLite 编译选项、备份和加密构建。
- IPC 接口表达业务动作，而不是数据库动作。
- 后续调整表结构时不破坏前端接口。

## 6. 仓库结构

```text
/
├── docs/
├── package.json
├── pnpm-lock.yaml
├── src/
│   ├── app/
│   │   ├── router/
│   │   ├── providers/
│   │   └── layouts/
│   ├── features/
│   │   ├── today/
│   │   ├── inbox/
│   │   ├── tasks/
│   │   ├── reminders/
│   │   ├── memory/
│   │   ├── clipboard/
│   │   ├── search/
│   │   └── settings/
│   ├── components/
│   ├── design-system/
│   │   ├── primitives/
│   │   └── patterns/
│   ├── ipc/
│   ├── styles/
│   │   ├── app.css
│   │   └── theme.css
│   ├── stores/
│   └── test/
└── src-tauri/
    ├── migrations/
    ├── capabilities/
    ├── src/
    │   ├── commands/
    │   ├── application/
    │   ├── domain/
    │   ├── infrastructure/
    │   ├── platform/
    │   └── app_state.rs
    ├── tests/
    └── tauri.conf.json
```

第一版保持一个前端包和一个 Rust crate。只有出现明确编译边界或复用需求时，再拆分 workspace package 或 Rust crate。

## 7. 进程与窗口

### 7.1 进程模型

第一版只运行一个 Tauri 主进程：

- Rust 主进程持有数据库、提醒调度器和剪切板监听器。
- WebView 只负责展示和收集用户输入。
- 关闭所有普通窗口不终止主进程。
- 用户从托盘明确选择“退出”时才停止主进程。
- single-instance 插件保证同一用户只运行一个业务进程。

第一版不使用独立守护进程、Sidecar 或本地 HTTP 服务。

### 7.2 窗口模型

只预设两个 WebView 窗口：

| 窗口 | 职责 | 生命周期 |
| --- | --- | --- |
| `main` | 完整工作台 | 按需显示，关闭时隐藏 |
| `quick` | 快速记录、搜索、剪切板浮层 | 启动后预创建并隐藏 |

`quick` 使用内部路由或 mode 切换，不为每个浮层创建新的 WebView。这样可以减少内存占用和首次唤起延迟。

窗口要求：

- 全局快捷键触发后快速显示并聚焦输入框。
- 失去焦点时是否自动隐藏由当前 mode 决定。
- 记住主窗口尺寸和位置。
- 窗口不得自行持有独立业务数据副本。
- 多显示器和系统缩放需要分别测试。

### 7.3 托盘

托盘使用原生菜单，首版提供：

- 打开主窗口。
- 快速记录。
- 打开剪切板浮层。
- 暂停或恢复剪切板记录。
- 展示下一条提醒摘要。
- 设置和退出。

复杂的今日列表仍由 WebView 展示，不将完整页面复制到原生托盘菜单。

## 8. 前端架构

### 8.1 Tailwind CSS

Tailwind CSS 4 是前端唯一的样式构建方案，通过 `@tailwindcss/vite` 接入 Vite。所有 WebView 入口导入同一份 `src/styles/app.css`，不为 `main` 和 `quick` 维护两套样式。

基础入口采用 CSS-first 配置：

```css
@import "tailwindcss";
@import "./theme.css";
```

项目默认不创建 `tailwind.config.js`。只有官方 CSS 配置能力无法满足明确需求时，才增加 JavaScript 配置。

Tailwind CSS 4 面向现代浏览器。最低 macOS 版本所带的 WKWebView 和 Windows WebView2 必须满足其兼容基线；阶段 0 需要在真实 Tauri WebView 中验证，不能只在开发浏览器中测试。对支持范围不明确的新 CSS utility，先检查两端 WebView 支持再使用。

#### 设计令牌

颜色、字号、圆角、阴影和间距通过 `@theme` 及 CSS 自定义属性统一定义。业务组件只使用语义令牌，不直接散布品牌色值：

```css
@theme {
  --color-surface: var(--workbench-surface);
  --color-surface-raised: var(--workbench-surface-raised);
  --color-foreground: var(--workbench-foreground);
  --color-muted: var(--workbench-muted);
  --color-accent: var(--workbench-accent);
  --color-danger: var(--workbench-danger);
  --radius-control: 0.375rem;
  --radius-panel: 0.5rem;
}
```

令牌要求：

- 颜色表达用途，例如 surface、foreground、accent、danger，不表达具体色相。
- 组件不得自行建立另一套颜色或间距体系。
- 固定格式控件使用稳定尺寸，避免加载、选中和悬停造成布局跳动。
- 紧凑桌面工作台默认使用较小的字号和间距，不采用营销页面尺度。
- 卡片圆角不超过 8px，页面区域不包装成多层卡片。

#### 主题

支持 `system`、`light`、`dark` 三种设置：

- 根元素使用 `data-theme` 表达已解析主题。
- Tailwind `dark` variant 映射到 `[data-theme="dark"]`。
- 用户设置保存在 Rust 设置服务中，所有 WebView 共用。
- 可以在 Local Storage 缓存最后一次已解析主题以避免启动闪烁，但它不是事实来源。
- 系统主题变化只在选择 `system` 时立即生效。
- 高对比度、减少动态效果等系统偏好通过媒体查询支持。

#### 类名规则

- Tailwind 类名必须以完整静态字符串出现在源码中。
- 不使用 `bg-${color}-500` 等运行时拼接方式。
- 组件变体通过 class-variance-authority 映射到完整类名。
- `tailwind-merge` 只用于合并受控组件类名，不用于掩盖无边界的外部样式覆盖。
- arbitrary value 只用于确实来自布局计算的值，常用值必须提升为设计令牌。
- 不在 JSX 中使用大段 `style`；数据库或运行时产生的数值可以通过 CSS 自定义属性传入。

#### 组件边界

`src/design-system/primitives` 保存 Button、Input、Checkbox、Dialog、Menu、Tabs、Tooltip 等基础组件；`patterns` 保存 TaskRow、SearchResult、ReminderEditor 等跨功能组合。

Radix UI 只提供交互语义和可访问性，视觉全部由 Tailwind 类和设计令牌控制。功能模块优先复用设计系统，不直接复制一组相似 utility class。

允许编写少量普通 CSS 的场景：

- WebView 和窗口拖动区域等平台样式。
- Markdown 正文排版。
- Tailwind 无法清晰表达的复杂动画或伪元素。
- 全局字体渲染、滚动条和焦点基础规则。

不同时引入 CSS Modules、CSS-in-JS 或另一套组件视觉主题。

### 8.2 数据状态

前端数据分为三类：

1. **服务端式状态**：来自 Rust 和 SQLite，使用 TanStack Query。
2. **表单草稿**：组件或 React Hook Form 管理，提交成功后失效相关查询。
3. **临时 UI 状态**：窗口 mode、选择项、弹层状态，使用 Zustand 或组件状态。

禁止将任务、记忆和剪切板完整镜像到 Zustand。

### 8.3 查询键

查询键按领域和条件组织，例如：

```ts
['tasks', 'today']
['tasks', { listId, status, cursor }]
['task', taskId]
['search', { query, types, cursor }]
['clipboard', { favoritesOnly, cursor }]
```

Rust 完成事务后发出仅包含实体类型、ID 和变更类型的事件，前端据此精确刷新缓存。

### 8.4 编辑策略

- 普通字段采用保存按钮或短延迟自动保存，具体页面保持一致。
- 乐观更新只用于完成、收藏、排序等易回滚操作。
- 提醒规则、周期规则、删除和批量操作先等待 Rust 成功响应。
- 错误提示保留用户草稿，不因 IPC 失败清空输入。
- 所有破坏性操作必须在 Rust 层再次校验。

### 8.5 Markdown

- 记忆正文保存原始 Markdown 文本。
- Markdown 预览必须禁用原始 HTML，或经过严格白名单清理。
- 外部链接通过 Tauri opener 打开，不在当前 WebView 加载远程页面。
- 剪切板 HTML 不直接渲染。

## 9. IPC 契约

### 9.1 Command 规则

Command 使用业务动词命名，例如：

```text
task_create
task_complete
task_reschedule
reminder_snooze
memory_save
clipboard_convert_to_memory
search_query
backup_create
```

禁止向前端提供以下接口：

```text
execute_sql
read_any_file
write_any_file
run_shell
invoke_arbitrary_command
```

### 9.2 返回结构

Rust Command 使用统一错误模型：

```ts
type AppError = {
  code: string;
  message: string;
  fieldErrors?: Record<string, string>;
  retryable: boolean;
};
```

错误码稳定，展示文案可本地化。数据库内部错误和文件路径不直接发送给前端。

### 9.3 类型生成

Rust DTO 是 IPC 契约的事实来源。阶段 0 评估 `tauri-specta` 或等价生成方案：

- 自动生成 TypeScript 类型和调用函数。
- CI 检查生成文件是否与 Rust 定义一致。
- 不使用 TypeScript 手写一套近似 DTO。
- 数据库 Record 不直接作为 IPC DTO。

### 9.4 事件

事件仅用于通知变化，不承担可靠数据传输：

```ts
type DomainChangeEvent = {
  entityType: 'task' | 'reminder' | 'memory' | 'clipboard';
  entityId: string;
  change: 'created' | 'updated' | 'deleted';
  revision: number;
};
```

事件不包含任务正文、记忆正文或剪切板内容。窗口漏掉事件时，可以重新查询数据库恢复正确状态。

## 10. Rust 应用与领域层

### 10.1 Application 层

每个用户动作对应一个用例。用例负责：

- 输入校验。
- 调用领域服务。
- 打开并提交事务。
- 更新搜索文档。
- 安排或取消系统通知。
- 提交成功后发送变化事件。

系统通知属于事务后的外部副作用。若数据库提交成功但系统调度失败，记录待协调状态，由调度器重试，不能回滚已经提交的用户数据。

### 10.2 Domain 层

Domain 层包括：

- 任务状态转换。
- 周期任务实例生成。
- 提醒下一次时间计算。
- 稍后提醒和错过规则。
- 剪切板去重和保留规则。
- 实体转换规则。
- 搜索结果类型和排序策略。

时间相关逻辑依赖抽象 `Clock` 和明确时区，测试中不能直接读取系统当前时间。

### 10.3 Repository

Domain 定义 Repository trait，Infrastructure 提供 SQLite 实现。上层不能依赖具体 SQL：

```text
TaskRepository
ReminderRepository
MemoryRepository
ClipboardRepository
SearchRepository
SettingsRepository
```

跨多个 Repository 的业务操作由同一个 Unit of Work 保证事务一致性。

## 11. 存储总体设计

### 11.1 混合存储

```text
SQLite
├── 任务、清单、标签
├── 提醒、周期规则和提醒实例
├── 记忆正文
├── 文本剪切板
├── 资源元数据和实体关联
├── 搜索文档和 FTS5 索引
└── 迁移及内部状态

assets/
├── clipboard/
├── attachments/
└── thumbnails/

backups/
logs/
config/
```

SQLite 是结构化业务数据唯一事实来源。资源目录中的文件必须有数据库记录；孤立资源通过维护任务清理。

### 11.2 不使用 IndexedDB 的原因

- Rust 提醒和剪切板服务无法直接、可靠地共享数据。
- 多 WebView 协调和迁移更复杂。
- 数据与系统 WebView Profile 生命周期耦合。
- 备份、恢复、诊断和数据库版本控制不够直接。
- 不同平台 WebView 的实现和配额行为存在差异。

Local Storage 只允许保存无敏感性的前端显示偏好，并且这些偏好丢失不能影响业务数据。

### 11.3 不采用文件优先

Markdown 和 JSON 不作为运行时事实来源，因为任务、提醒、标签、转换和剪切板需要事务、关系和高频部分更新。

开放文件格式通过导出提供：

- 完整版本化 JSON。
- 记忆 Markdown。
- 任务 CSV。

### 11.4 选型依据与同类模式

SQLite 适合当前产品的原因：

- 任务、标签、提醒和实体转换具有明确关系。
- 完成周期任务时需要在一个事务内修改当前实例、生成下一实例和更新调度状态。
- 剪切板产生大量小记录和高频局部更新。
- 今日、逾期、等待和组合筛选适合关系查询和索引。
- SQLite 是无服务进程的跨平台文件数据库，适合 Tauri 本地后端。

公开可验证的同类产品呈现两种主要路线：

- Obsidian 以 Markdown 文件夹为事实来源，适合长篇笔记和外部编辑优先的产品。
- AppFlowy 的本地架构使用 SQLite，并通过领域接口隔离数据库实现，更接近本产品的结构化离线工作台。
- Ditto 和 Maccy 等剪切板工具使用本地数据库保存历史，说明高频剪切板数据使用数据库是成熟路径。
- Standard Notes 将设备数据库加密和密钥管理分开，说明存储选择不能替代独立的隐私设计。

本产品的记忆是短信息片段，不是用户直接管理的文档仓库，因此不采用 Obsidian 式文件优先；但通过 Markdown 和 JSON 导出保留用户对数据的长期可读性。

SQLite 的限制也必须进入设计：

- WAL 同一时刻只有一个写入者，因此写事务必须短小。
- SQLite 不提供业务级跨设备同步，不能用网盘复制运行中的数据库。
- SQLite 默认不加密，加密和密钥管理需要单独决策。
- 大型图片和附件不进入主数据库，避免放大 WAL、备份和恢复成本。
- 全文搜索需要针对中文和短关键词设计回退策略。

## 12. SQLite 设计

### 12.1 运行参数

- 使用绑定或明确版本的 SQLite，不无条件依赖系统版本。
- 启用外键。
- 使用 WAL 模式。
- 配置合理的 busy timeout。
- 使用参数化查询。
- 控制长读事务，避免阻塞 checkpoint。
- 后台空闲时执行 checkpoint 和空间维护。
- CI 验证 SQLite 编译选项包含 FTS5。
- SQLite 版本必须包含 2026 年 WAL reset 问题的修复或受维护的回补丁。

### 12.2 连接模型

- Rust 是唯一业务写入进程。
- 一个串行写入执行器处理短事务。
- 使用小型只读连接池处理列表和搜索。
- 所有阻塞数据库操作离开 Tokio 核心线程执行。
- 不允许 WebView、浏览器扩展或外部脚本直接打开运行中的数据库。

### 12.3 核心表

第一版预计包含：

| 表 | 用途 |
| --- | --- |
| `task_lists` | 任务清单 |
| `tasks` | 单次任务和周期任务实例 |
| `task_series` | 周期任务模板及生成游标 |
| `tags` | 标签 |
| `task_tags` | 任务标签关系 |
| `reminders` | 提醒定义 |
| `reminder_occurrences` | 可调度的提醒实例 |
| `memories` | 记忆正文和状态 |
| `clipboard_items` | 文本剪切板及资源引用 |
| `assets` | 图片、文件和缩略图元数据 |
| `entity_links` | 任务、记忆和资源间关联 |
| `search_documents` | 统一搜索的规范化文档 |
| `search_index` | FTS5 虚拟表 |

所有主实体使用 UUID，至少包含：

- `created_at`。
- `updated_at`。
- 单调递增的 `revision`。
- 可选 `deleted_at`。

不为未来同步提前引入完整事件溯源或 CRDT。

### 12.4 周期任务

`task_series` 保存：

- 当前模板数据。
- 结构化重复规则。
- 原始时区。
- 下一次生成时间。
- 启用和结束状态。

`tasks` 保存每个实际实例。完成当前实例后，在同一事务中生成下一实例并推进 series 游标。

重复规则使用版本化结构，不把自由文本作为执行依据：

```json
{
  "version": 1,
  "frequency": "weekly",
  "interval": 1,
  "weekdays": [1, 5],
  "timezone": "Asia/Shanghai",
  "endAt": null
}
```

### 12.5 提醒实例

`reminders` 保存业务定义，`reminder_occurrences` 保存实际安排：

```text
pending -> scheduled -> actioned
                    -> snoozed
                    -> cancelled
                    -> inferred_missed
```

系统未必能可靠报告“用户看见但未操作”，因此数据库不声明精确的 delivered 状态。

## 13. 搜索

### 13.1 索引结构

`search_documents` 统一保存可搜索文本：

- 自增内部 row ID。
- 实体类型和 UUID。
- 标题、正文和规范化文本。
- 更新时间。
- 不参与分词的筛选字段。

`search_index` 使用外部内容 FTS5 表。业务实体与搜索文档在同一事务中更新，索引可从业务表完全重建。

### 13.2 中文策略

- 三个字符以上优先使用 trigram FTS5。
- 一至两个字符使用带转义和限制条件的 `LIKE` 回退。
- 英文和数字做大小写、空白和 Unicode 规范化。
- 剪切板长文本只索引受控长度的规范化内容，原文仍完整保存。
- 后续 OCR 文本作为派生字段进入同一搜索文档。

### 13.3 查询安全

- 用户输入不直接拼接为 SQL。
- FTS 查询表达式由后端构造和转义。
- 限制查询长度、返回数量和摘要长度。
- 空查询不扫描完整剪切板正文。

## 14. 提醒系统

### 14.1 数据流

```text
Reminder Definition
    -> Recurrence Engine
    -> Reminder Occurrences
    -> OS Notification Scheduler
    -> Notification Mapping
    -> User Action
    -> Database Update
```

SQLite 是事实来源，系统通知是投影。

### 14.2 调度器

Rust `ReminderScheduler` 负责：

- 生成有限时间窗口内的提醒实例。
- 调用 Tauri notification 插件安排系统通知。
- 保存业务 UUID 与系统 32 位通知 ID 的映射。
- 取消已经失效或修改的系统通知。
- 应用启动和周期检查时进行协调。
- 恢复休眠后处理已经到期事项。
- 记录调度错误并重试。

复杂重复规则由 Rust 计算，不依赖插件的固定 interval 表达完整业务语义。

### 14.3 事务与外部副作用

通知调度不与 SQLite 事务假装原子：

1. 事务写入 reminder occurrence，并标记 `needs_schedule`。
2. 提交事务。
3. 调用系统通知接口。
4. 成功后记录系统 ID；失败时保留重试状态。

启动协调器通过幂等键避免重复通知。

### 14.4 平台验证

阶段 0 必须分别验证 macOS 和 Windows：

- 一次性通知。
- 周期或未来通知。
- 操作按钮。
- 点击通知唤醒隐藏中的应用。
- 应用退出后的系统调度行为。
- 系统休眠和恢复。
- 通知权限被拒绝后的降级。

## 15. 剪切板系统

### 15.1 监听流程

```text
Clipboard Poller
    -> Detect Change
    -> Inspect Types
    -> Apply Exclusion Rules
    -> Normalize Content
    -> Hash and Deduplicate
    -> Persist in Transaction
    -> Emit Change Event
```

官方 clipboard-manager 插件负责读写，Rust 后台服务负责监听、去重和持久化。

### 15.2 规则

- 第一版只持久化文本。
- 相邻相同内容不创建新条目，更新最近使用信息。
- 内容哈希使用明确的规范化规则和稳定算法。
- 来源应用是可空、非可信的辅助字段。
- 排除来源、暂态类型和用户暂停状态在读取正文前尽可能判断。
- 应用自己写回剪切板时记录短期 suppression token，避免重复采集。
- 自动过期和数量上限由 Rust 后台维护任务执行。

### 15.3 直接粘贴

直接粘贴通过平台适配器实现：

- macOS：辅助功能授权和系统输入事件。
- Windows：前台窗口和系统输入 API。
- Linux：按 X11、Wayland 能力分别判断。

未获得权限时统一降级为“复制到系统剪切板”。直接粘贴失败不能丢失用户原剪切板；需要在短时间后按策略恢复原内容。

## 16. 平台适配层

Rust 定义以下稳定接口：

```text
NotificationAdapter
ClipboardAdapter
GlobalShortcutAdapter
AutostartAdapter
DirectPasteAdapter
ForegroundAppAdapter
CredentialStore
FileOpenAdapter
```

Tauri 官方插件实现通用路径，`platform/macos`、`platform/windows` 和 `platform/linux` 只处理差异。

### 16.1 能力矩阵

| 能力 | macOS | Windows | Linux |
| --- | --- | --- | --- |
| 文本剪切板 | 首批 | 首批 | 后续验证 |
| 来源应用 | 尽力获取 | 尽力获取 | 可为空 |
| 直接粘贴 | 权限后支持 | 支持 | 能力检测后决定 |
| 定时通知 | 首批 | 首批 | 后续验证 |
| 通知操作 | 首批 | 首批 | 依桌面环境降级 |
| 全局快捷键 | 首批 | 首批 | 后续验证 |
| 开机启动 | 首批 | 首批 | 后续验证 |

业务层不得根据平台字符串分支，应根据适配器报告的 capability 判断。

## 17. Tauri 权限与应用安全

### 17.1 Capability

按窗口分配权限：

| 窗口 | 允许能力 |
| --- | --- |
| `main` | 完整业务 Command、窗口控制、打开用户选择的文件 |
| `quick` | 创建、搜索、复制、粘贴和有限窗口控制 |

前端不直接获得：

- SQL 插件权限。
- Shell 权限。
- 任意文件系统权限。
- 任意窗口创建权限。
- 不受限的网络访问。

系统插件尽量由 Rust 调用，不把底层能力暴露给 WebView。

### 17.2 WebView

- 生产环境只加载打包资源。
- 配置严格 CSP。
- 禁止不受控内联脚本。
- 不在应用 WebView 中打开外部站点。
- Markdown 和剪切板 HTML 必须清理。
- 生产版本默认关闭调试入口。
- 依赖更新进入自动安全扫描。

### 17.3 日志

日志可以记录：

- Command 名称和耗时。
- 实体类型和脱敏 ID。
- 数据库迁移版本。
- 调度结果和错误码。
- 系统能力状态。

日志禁止记录：

- 任务和记忆正文。
- 剪切板内容及 OCR 文本。
- 完整文件路径。
- 加密密钥、令牌和导入数据。

## 18. 加密设计门槛

SQLite 本身不默认加密。阶段 0 必须比较两种方案：

### 方案 A：SQLCipher

- 整个数据库页面和 FTS 表一起加密。
- 设备主密钥保存在操作系统凭据库。
- 自动备份也保持加密。
- 需要验证 macOS、Windows 的构建、升级、性能和崩溃恢复。

### 方案 B：依赖系统磁盘保护

- 数据库为普通 SQLite。
- 实现简单，搜索和备份没有额外限制。
- 只能防护由操作系统磁盘加密覆盖的威胁。
- 必须在产品中明确说明数据库不是应用级加密。

不采用正文逐字段加密加明文搜索索引的折中方式，因为搜索索引仍会泄露内容，安全语义难以解释。

若选择 SQLCipher，资源目录也必须设计加密，否则图片和缩略图会成为明文旁路。Linux 无可靠系统凭据库时不得静默退回明文。

## 19. 备份、导出与迁移

### 19.1 备份

- 使用 SQLite Online Backup API 或等价一致性快照，不直接复制运行中的主数据库文件。
- 备份包含数据库、资源清单、资源文件和 manifest。
- manifest 包含应用版本、数据库版本、创建时间、加密方式和校验和。
- 写入临时目录，全部成功后原子移动为正式备份。
- 定期验证最近备份可以打开并读取 manifest。

### 19.2 导出

- 完整 JSON 是跨版本的无损业务导出格式。
- Markdown 用于记忆长期可读性。
- CSV 用于任务迁移，不保证保存全部周期和提醒语义。
- 导出格式具有独立版本，不直接暴露数据库表结构。

### 19.3 迁移

- SQL migration 内嵌在应用包中并带校验和。
- 启动时在展示业务窗口前检查版本。
- 迁移前创建可恢复备份。
- 每个迁移在事务中执行；不能事务化的资源迁移使用可恢复阶段状态。
- 失败时停止写入并提供恢复入口，不能自动创建空数据库掩盖错误。
- CI 从每个已发布 schema 版本测试升级到当前版本。

## 20. 测试策略

### 20.1 Rust 单元测试

- 任务状态转换。
- 周期任务生成。
- 提醒下一次时间。
- 时区和夏令时边界。
- 剪切板去重和过期。
- 实体转换。
- 搜索查询解析。

时间测试使用 Fake Clock 和固定时区。

### 20.2 数据库集成测试

- 全量迁移和逐版本升级。
- 外键和唯一约束。
- 事务回滚。
- FTS5 中文搜索和短词回退。
- 索引重建。
- 备份恢复。
- WAL checkpoint。
- 加密数据库错误密钥和密钥轮换，若启用 SQLCipher。

### 20.3 前端测试

- Vitest：格式化、状态转换适配和组件逻辑。
- Testing Library：表单、键盘操作、错误和空状态。
- Tailwind 生产构建检查：验证 `main`、`quick` 窗口使用的类均进入生成 CSS。
- 设计系统组件覆盖 light、dark、键盘焦点、禁用和错误状态。
- Storybook 或等价组件环境只在需要稳定组件目录时引入，不作为阶段 0 必选项。

### 20.4 契约测试

- Rust DTO 生成 TypeScript 后必须通过类型检查。
- IPC 错误码具有快照测试。
- 前端 mock 与真实 Command 返回结构共享生成类型。

### 20.5 桌面端到端测试

使用 WebdriverIO 的 Tauri service：

- 浏览器模式快速测试 React 与 mock IPC。
- 嵌入式 WebDriver 测试真实 Tauri 应用。
- macOS 和 Windows 都运行核心流程。
- 通知权限、全局快捷键、剪切板和系统休眠保留平台手工测试矩阵。

关键流程包括快速创建、任务完成、提醒稍后处理、剪切板搜索、转换为记忆以及备份恢复。

## 21. CI 与发布

### 21.1 CI

每次变更执行：

- TypeScript lint 和类型检查。
- React 单元测试。
- Rust fmt、clippy 和单元测试。
- 数据库迁移及集成测试。
- IPC 生成文件一致性检查。
- 依赖和许可证检查。
- macOS、Windows 构建检查。

稳定分支额外执行真实 Tauri 端到端测试和安装包冒烟测试。

### 21.2 发布物

- macOS：签名并公证的 DMG，Apple Silicon 为首要架构；是否提供 Intel 构建按用户范围决定。
- Windows：签名的 NSIS 或 MSI 安装包。
- Linux：后续提供 AppImage 或发行版包。
- 更新包：使用 Tauri Updater 签名，更新失败不修改用户数据库。

签名密钥和更新私钥只存在于受控 CI Secret，不进入仓库或开发日志。

## 22. 阶段 0 技术验证

正式业务开发前，用 3 至 5 个工作日验证以下内容：

1. `quick` 窗口预创建、全局快捷键唤起、焦点和多显示器行为。
2. macOS、Windows 后台剪切板监听、去重、暂停和再次复制。
3. 一次性通知、未来通知、操作按钮和应用唤醒。
4. SQLite WAL、迁移、备份和中文 FTS5。
5. SQLCipher 或普通 SQLite 两种构建的体积、性能和升级路径。
6. macOS、Windows 的凭据库存取和权限拒绝处理。
7. Tauri Capability 是否能让 `quick` 窗口只访问有限 Command。
8. WebdriverIO 是否能在两平台驱动真实安装构建。
9. Tailwind CSS 4 生产样式在最低支持的 WKWebView、WebView2 中正确渲染。

验证结束后形成 ADR，锁定：

- SQLite 驱动和加密方案。
- 通知插件能否满足操作回调，是否需要自研平台插件。
- 直接粘贴的实现与权限提示。
- 最低 macOS 和 Windows 版本。
- 安装包格式和自动更新渠道。

## 23. 已知风险

| 风险 | 影响 | 应对方式 |
| --- | --- | --- |
| WebView 平台差异 | 快捷窗口、字体和输入行为不一致 | 双平台从阶段 0 持续测试，不在发布末期集中适配 |
| 系统通知语义差异 | 操作按钮或重复提醒不一致 | 数据库作为事实来源，使用 occurrence 和协调器 |
| 剪切板敏感信息 | 本地隐私泄露 | 排除规则、暂停、过期、加密决策和脱敏日志 |
| SQLite WAL 与版本问题 | 数据损坏或备份不一致 | 绑定修复版本、单写入进程、官方备份 API、迁移测试 |
| SQLCipher 跨平台构建 | 包体、签名和升级复杂 | 阶段 0 先验证，未验证前不做产品承诺 |
| 社区插件停止维护 | 系统能力不可升级 | 优先官方插件，平台适配接口隔离依赖 |
| 前端权限过宽 | WebView 漏洞扩大影响 | 按窗口 Capability、无远程内容、Rust 再校验 |
| Tailwind CSS 4 与旧 WebView 不兼容 | 样式缺失或布局异常 | 阶段 0 锁定最低系统和 WebView 版本，避免未验证的新 CSS 能力 |
| Linux 桌面环境碎片化 | 功能降级和测试量增加 | Linux 后置并建立能力检测，不假设统一行为 |

## 24. 参考资料

- [Tauri 官方插件](https://v2.tauri.app/plugin/)
- [Tauri 通知 API](https://v2.tauri.app/reference/javascript/notification/)
- [Tauri Permissions](https://v2.tauri.app/security/permissions/)
- [Tauri Capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri WebDriver 测试](https://v2.tauri.app/develop/tests/webdriver/)
- [Tailwind CSS Vite 安装](https://tailwindcss.com/docs/installation/using-vite)
- [Tailwind CSS 主题变量](https://tailwindcss.com/docs/theme)
- [Tailwind CSS 暗色模式](https://tailwindcss.com/docs/dark-mode)
- [Tailwind CSS 源码扫描](https://tailwindcss.com/docs/detecting-classes-in-source-files)
- [SQLite 事务](https://www.sqlite.org/transactional.html)
- [SQLite WAL](https://www.sqlite.org/wal.html)
- [SQLite FTS5](https://www.sqlite.org/fts5.html)
- [SQLite Online Backup API](https://www.sqlite.org/backup.html)
- [SQLite 作为应用文件格式](https://www.sqlite.org/appfileformat.html)
- [AppFlowy SQLite 架构](https://docs.appflowy.io/docs/documentation/software-contributions/architecture/backend/database)
- [Obsidian 数据存储](https://obsidian.md/help/data-storage)
- [Ditto 剪切板工具](https://github.com/sabrogden/Ditto)
- [Maccy 本地存储说明](https://github.com/p0deje/Maccy/issues/1335)
- [Standard Notes 设备数据库加密](https://standardnotes.com/help/79/how-does-standard-notes-encrypt-data-on-my-device)

# Design: 专注模式与每日收尾

> 体验目标：**Everdo 单条 Focus 的沉浸 + Will Be Done 无 guilt 收尾 + Trove 键盘/撤销一致性**。  
> 归档前须通过父任务 [`quality-bar.md`](../08-16-v1-3-gap-roadmap/quality-bar.md)。  
> **硬依赖**：[`08-16-gtd-workflow-states`](../08-16-gtd-workflow-states/design.md) 的今日重点 / 推迟 / 等待语义。

---

## 1. 对标场景（极致验收）

| # | 场景 | 竞品参考 | Trove 目标 |
| --- | --- | --- | --- |
| F1 | 从今日重点选一条进入全屏专注 | Everdo Focus Review | 今日页 `Enter` 或 `⌘↵` → 专注层；仅当前任务 + 关联记忆/附件 |
| F2 | 25 分钟倒计时结束 | 番茄钟 | 本地通知「时间到」；**不**锁屏、不强制停应用、不自动完成任务 |
| F3 | 专注中 App 崩溃 | — | 重启后任务仍为 todo；未保存的进展草稿丢失可接受；会话标记 `abandoned` |
| F4 | 下班每日收尾 | GTD daily review | 5 步向导可跳过；未完成重点逐项决策；**禁止** silent 批量延期 |
| F5 | 收尾误操作 | — | 每步 recent-actions 可撤销 |

---

## 2. 概念边界

| 概念 | 说明 | 与 gtd-workflow-states 关系 |
| --- | --- | --- |
| **专注会话** `FocusSession` | 一次「正在做某任务」的时间盒 | 引用 `task_id`；不修改 task status 直到用户明确完成 |
| **每日收尾** `DailyWrapRun` | 一次向导式流程实例 | 调用 defer/wait/focus API；不写新 task 字段 |
| **进展备注** | 专注退出时可选文本 | 存入 `FocusSession.progress_note`；可选追加到 task.notes（默认**不**追加） |
| **分心洞察** | 前台应用切换记录 | **本任务 follow-up**；默认关闭（roadmap §5.6） |

---

## 3. 数据模型

### 3.1 Migration `0011_focus_sessions.sql`（在 0010 之后）

```sql
CREATE TABLE focus_sessions (
  id TEXT PRIMARY KEY NOT NULL,
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  planned_minutes INTEGER,          -- null = 无倒计时
  outcome TEXT NOT NULL DEFAULT 'in_progress'
    CHECK (outcome IN ('in_progress', 'completed', 'kept_todo', 'abandoned')),
  progress_note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_focus_sessions_task ON focus_sessions (task_id, started_at DESC);

CREATE TABLE daily_wrap_runs (
  id TEXT PRIMARY KEY NOT NULL,
  wrap_date TEXT NOT NULL,          -- YYYY-MM-DD local
  started_at TEXT NOT NULL,
  completed_at TEXT,
  steps_completed INTEGER NOT NULL DEFAULT 0,
  summary_json TEXT,                -- 完成数/决策计数等
  created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_daily_wrap_date ON daily_wrap_runs (wrap_date)
  WHERE completed_at IS NOT NULL;
```

**不建**分心表（follow-up 任务）。

### 3.2 Domain 类型

```rust
pub enum FocusOutcome {
    InProgress,
    Completed,   // 用户专注内点完成
    KeptTodo,    // 退出时保持待办
    Abandoned,   // 异常/强退
}

pub struct FocusSession { id, task_id, started_at, ended_at, planned_minutes, outcome, progress_note, ... }

pub struct DailyWrapRun { id, wrap_date, started_at, completed_at, steps_completed, summary, ... }

pub struct DailyWrapSnapshot {
  unfinished_focus: Vec<Task>,
  tomorrow_due: Vec<Task>,
  inbox_unprocessed: Vec<Task>,
  completed_today_count: i64,
}
```

### 3.3 IPC

| 命令 | 说明 |
| --- | --- |
| `focus_start(task_id, planned_minutes?)` | 开始；若已有 in_progress 则先 abandon |
| `focus_end(session_id, outcome, progress_note?)` | 结束；outcome=completed 时调 task_complete |
| `focus_active()` | 当前 in_progress 会话或 null |
| `focus_list(task_id?, limit?)` | 历史（weekly-review 复用） |
| `daily_wrap_snapshot(wrap_date?)` | 收尾第 0 步数据 |
| `daily_wrap_start(wrap_date?)` | 创建 run |
| `daily_wrap_complete(run_id, summary)` | 标记完成 |

---

## 4. 专注模式 UX（极致）

### 4.1 入口

| 入口 | 条件 |
| --- | --- |
| Today 重点行 `Enter` / 工具栏「专注」 | 任务 todo |
| TaskDetailPanel 按钮「开始专注」 | 任意 todo |
| 键盘 `⌘⇧F`（Focus session，Today 页） | 选中任务时 |

### 4.2 专注层 UI（全屏 overlay，非新窗口）

```
┌─────────────────────────────────────────────┐
│  [ Esc 退出专注 ]          [ 25:00 ▼ ]      │  ← 计时可选 15/25/45/无
│                                             │
│  ○  任务标题                                 │
│     清单 · 截止 · 优先级                      │
│                                             │
│  ── 说明 ──                                  │
│  notes 区（只读，链接「在主窗口编辑」）         │
│                                             │
│  ── 相关 ──                                  │
│  EntityLink 记忆/附件列表（最多 5，可展开）    │
│                                             │
│  [ 进展备注…………………… optional textarea ]       │
│                                             │
│  [ 完成任务 ]  [ 保持待办并退出 ]              │
└─────────────────────────────────────────────┘
```

**极致细节**

- 背景 `surface` 略提亮，**无** blur 全屏（性能 + 可访问性）
- 倒计时用 `requestAnimationFrame` + 可见时校准；后台 tab 依赖系统通知到点
- `Esc` → 确认「保持待办并退出」vs「继续专注」（防误触）
- 完成任务 = `focus_end(completed)` + 庆祝无 confetti（muted toast）
- 禁止：全屏阻断其他 App、自动 DND 系统

### 4.3 通知

- 标题：「专注时间到」
- 正文：任务标题
- 操作：打开 Trove（导航 Today）、稍后 5 分钟（仅重排本地计时，不 snooze 任务）

### 4.4 异常退出

- `beforeunload` / 进程退出：best-effort `focus_end(abandoned)`
- 启动时 `focus_active()` 若 in_progress → 横幅「上次专注未正常结束，已保存为放弃」

---

## 5. 每日收尾 UX（极致）

### 5.1 入口

- Today 页顶部「每日收尾」按钮（日落后或任意时刻可点）
- 若今日已有 completed wrap → 显示「今日已收尾 · 查看摘要」

### 5.2 五步向导（Modal 分步，非全屏）

| Step | 标题 | 内容 | 可跳过 |
| --- | --- | --- | --- |
| 1 | 今日重点未完成 | 列表；每项：保留 / 推迟 / 等待 / 完成 / 移出重点 | ✅ |
| 2 | 明日预览 | 明天 due 任务只读列表 + 「逐条调整」链到 Tasks | ✅ |
| 3 | 收件箱 | inbox todo 计数；快捷进入 Inbox | ✅ |
| 4 | 当日摘要 | 完成数、重点完成率、提醒处理数（数字 only） | — |
| 5 | 完成 | 「收尾完成」写入 run | — |

**极致细节**

- 每步底部：**跳过此步** | **下一步**；第一步不可一键「全部延期」
- Step 1 每次决策立即调 API + recent-actions
- 文案：「还有 N 项重点未处理 — 选一个对你最合适的动作」；**禁止**「你又失败了」类 copy
- Step 4 数字可点击跳转列表

### 5.3 与推迟/等待整合

Step 1 选「推迟」→ 内联 DeferPicker（复用 gtd-workflow-states 组件）  
选「等待」→ 内联 waitingFor + followUpDate

---

## 6. 键盘（Today + 专注层）

| 上下文 | 键 | 动作 |
| --- | --- | --- |
| Today | `⌘⇧F` | 专注选中任务 |
| 专注层 | `Esc` | 退出确认 |
| 专注层 | `⌘↵` | 完成任务 |
| 收尾向导 | `1-5` | 对当前项快捷动作（可选增强） |
| 收尾向导 | `Esc` | 保存进度退出（run 未完成） |

---

## 7. 测试策略

### Rust

- focus_start/end 状态机；重复 start abandon 前会话
- focus_end(completed) 才 complete task
- daily_wrap snapshot 数字与 today_tasks 一致

### 前端

- 专注层 Esc 确认流
- 收尾 Step1 决策调 mock IPC

### 手动 S1–S5

---

## 8. 不做（本任务）

- 分心洞察 / Accessibility 监听（独立 follow-up）
- 子任务 checklist 在专注层（无 parent task 模型）
- Pomodoro 统计排行

---

## 9. 开放问题（已决）

| 问题 | 决定 |
| --- | --- |
| 进展备注是否写入 task.notes | **默认否**；设置项「退出专注时追加到备注」默认关 |
| 每日收尾是否强制 | **否**；入口可见但不 nag |
| 专注是否必须来自今日重点 | **否**；任意 todo 可专注，但 Today 入口优先重点 |

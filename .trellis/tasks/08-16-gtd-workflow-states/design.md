# Design: GTD 工作流（今日重点 / 推迟显示 / 等待事项）

> 体验目标：**Everdo 级工作流语义 + Things 级今日页克制美学 + Trove 键盘优先**。  
> 归档前须通过父任务 [`quality-bar.md`](../08-16-v1-3-gap-roadmap/quality-bar.md)。

---

## 1. 对标场景（极致验收用）

| # | 场景 | Everdo/Things 参考 | Trove 目标 |
| --- | --- | --- | --- |
| S1 | 早上从 20 条待办中选出 3 条今日重点 | Things Today 手动 pin | 列表内 `F` 或右键「加入今日重点」，≤2 键；拖拽排序 |
| S2 | 某任务「下周再看」，不想每天出现在今日 | Everdo 推迟/Someday | 「推迟显示」至日期；今日/收件箱不再出现；搜索+「已推迟」视图可找回 |
| S3 | 等同事回复，设跟进日 | Everdo Waiting | 标记等待 + 填写对象；跟进日进「等待跟进」区，不自动变回普通待办 |
| S4 | 误推迟，24h 内修正 | — | recent-actions 撤销；详情页一键「取消推迟」 |

---

## 2. 概念模型与术语（全应用冻结）

| 中文 UI | 英文/domain | 含义 | **不是** |
| --- | --- | --- | --- |
| **今日重点** | `DailyFocus` | 用户手动选入「今天要推进」的任务子集，有独立排序 | 不是改 due date；不是自动排程 |
| **推迟显示** | `availableAt` | 此日期之前任务不参与「活跃视图」 | 不是延期（`postpone` 改 due）；不是完成 |
| **等待** | `workflowState=waiting` | 任务 blocked，可选 `waitingFor` + `followUpDate` | 不是归档；不是提醒 snooze |
| **延期** | `postpone_task` | 修改 `dueDate`（已有） | 不改变 `availableAt` |
| **活跃任务** | `ActiveTaskPredicate` | 后端单一函数定义：哪些 todo 出现在 Inbox/Today/智能列表 | 各页面禁止手写 SQL 差异 |

### 2.1 活跃任务谓词（单一真相源）

```rust
// domain/task_activity.rs — 所有列表 MUST 调用
pub fn is_active_task(task: &Task, local_date: NaiveDate) -> bool {
    task.status == TaskStatus::Todo
        && task.deleted_at.is_none()
        && task.available_at.map_or(true, |d| d <= local_date)
        // waiting 任务：不在普通活跃列表，除非 follow_up 到期（见 §2.3）
        && !matches!(task.workflow_state, TaskWorkflowState::Waiting { .. })
}

pub fn is_waiting_follow_up_due(task: &Task, local_date: NaiveDate) -> bool {
    matches!(task.workflow_state, TaskWorkflowState::Waiting { follow_up_date: Some(d), .. } if d <= local_date)
}
```

**规则表**

| workflowState | availableAt | followUpDate | 收件箱/任务列表 | 今日页 | 今日重点区 |
| --- | --- | --- | --- | --- | --- |
| active | null 或 ≤today | — | ✅ | 按 due 规则 | 若已 pin 则 ✅ |
| active | >today | — | ❌ | ❌ | ❌（自动从今日重点移除，见 §4.2） |
| waiting | 任意 | null | ❌ | ❌ | ❌ |
| waiting | 任意 | >today | ❌ | ❌ | ❌ |
| waiting | 任意 | ≤today | ❌ | ✅「等待跟进」区 | 若已 pin 则 ✅ |

推迟与等待可叠加：`availableAt` 优先于 follow-up 进入活跃视图；两者皆满足时才出现在对应区。

### 2.2 与截止日冲突

保存时若 `dueDate < availableAt`（均有值）：

- **阻塞保存**，内联错误：「截止日期早于推迟显示日，任务会在此之前到期。请调整截止日期或推迟日。」
- 提供快捷动作：「改为与截止日相同」「清除推迟」

若 `waiting` 且 `followUpDate > dueDate`（均有值）：

- **警告非阻塞**：「跟进日晚于截止日期，到期时任务仍在等待中。」

---

## 3. 数据模型

### 3.1 Migration `0010_gtd_workflow.sql`

**tasks 表扩展**

```sql
ALTER TABLE tasks ADD COLUMN workflow_state TEXT NOT NULL DEFAULT 'active'
  CHECK (workflow_state IN ('active', 'waiting'));
ALTER TABLE tasks ADD COLUMN available_at TEXT;          -- YYYY-MM-DD, nullable
ALTER TABLE tasks ADD COLUMN waiting_for TEXT;             -- nullable, max 500
ALTER TABLE tasks ADD COLUMN follow_up_date TEXT;          -- YYYY-MM-DD, nullable

CREATE INDEX idx_tasks_available_active
  ON tasks (available_at, status, deleted_at)
  WHERE deleted_at IS NULL AND status = 'todo';

CREATE INDEX idx_tasks_waiting_followup
  ON tasks (follow_up_date, workflow_state, deleted_at)
  WHERE deleted_at IS NULL AND workflow_state = 'waiting';
```

**daily_focus 表**

```sql
CREATE TABLE daily_focus (
  focus_date TEXT NOT NULL,           -- YYYY-MM-DD local
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  sort_order REAL NOT NULL DEFAULT 0,
  added_at TEXT NOT NULL,
  carried_from_date TEXT,             -- 若从昨日结转，记录源日期
  PRIMARY KEY (focus_date, task_id)
);

CREATE INDEX idx_daily_focus_date_order
  ON daily_focus (focus_date, sort_order);
```

**迁移策略**

- 现有 todo：`workflow_state='active'`，`available_at=NULL`
- 不自动 pin 任何任务到今日重点

### 3.2 Domain 类型（Rust + TS 镜像）

```rust
pub enum TaskWorkflowState {
    Active,
    Waiting { waiting_for: Option<String>, follow_up_date: Option<NaiveDate> },
}
```

`Task` / `UpdateTaskInput` 扩展字段：`workflowState`, `availableAt`, `waitingFor`, `followUpDate`。

`TodayTasks` 扩展：

```rust
pub struct TodayTasks {
    // existing...
    pub focus: Vec<Task>,              // 今日重点（按 daily_focus.sort_order）
    pub waiting_follow_up: Vec<Task>, // follow_up_date <= today 的 waiting
    pub focus_carry_suggestions: Vec<Task>, // 昨日未完成重点，建议结转（仅 UI 提示）
}
```

### 3.3 新 IPC 命令

| 命令 | 说明 |
| --- | --- |
| `daily_focus_add(task_id, focus_date?)` | 加入今日重点；默认今天 |
| `daily_focus_remove(task_id, focus_date?)` | 移除 |
| `daily_focus_reorder(task_ids[])` | 排序 |
| `daily_focus_carry(from_date, to_date)` | 批量结转未完成重点 |
| `task_set_defer(id, available_at \| null)` | 推迟显示 |
| `task_set_waiting(id, waiting_for, follow_up_date \| null)` | 进入等待 |
| `task_clear_waiting(id)` | 结束等待 → active |

`task_update` 也可携带 workflow 字段，但**快捷动作走专用命令**以便撤销栈记录清晰。

`TaskQuery` 扩展：

```typescript
workflowState?: 'active' | 'waiting';
availableBefore?: string;  // available_at <= date（含 null 视为已可用）
deferredOnly?: boolean;    // available_at > today
waitingFollowUpDue?: boolean;
inFocusDate?: string;      // join daily_focus
```

新增智能列表：`SmartListKind::Deferred`、`SmartListKind::WaitingFollowUp`（可选，或 Tasks 页 chip）。

---

## 4. 体验设计（UI/UX 极致）

### 4.1 今日页信息架构（TodayPage）

自上而下：

```
[ 通知权限横幅 — 已有 ]

── 今日重点 (focus) ──  计数 badge · 建议「3–5 项」轻提示（非强制）
   [ 空状态：从下方拖入或按 F 加入 ]
   SortableTaskRow × N
   [ 昨日结转横幅：「2 项未完成，是否加入今日？」  加入 / 忽略 ]

── 等待跟进 (waiting_follow_up) ──
   TaskRow + 次要行「等待：张三 · 跟进日今天」

── 逾期 (overdue) ──  已有；**无红色羞辱徽章**，仅 muted「逾期」小字

── 今天到期 (due_today) ──

── 今日提醒 (reminders) ──

── 今天已完成 (completed_today) ──  默认折叠
```

**极致细节**

- 今日重点区**始终置顶**，即使为空也占最小高度，避免布局跳动
- 从 due_today 拖入 focus 区 = `daily_focus_add` + 乐观 UI
- 在 focus 区拖回 due 区 = `daily_focus_remove`（不改变 due）
- 完成任务时：若在该日 focus 中，自动 remove + 撤销栈记录

### 4.2 今日重点交互

| 操作 | 入口 | 键位 |
| --- | --- | --- |
| 加入重点 | 任务行右键 / 详情按钮 / 列表快捷键 | `F`（Focus） |
| 移出重点 | 同上 / 拖出 focus 区 | `Shift+F` |
| 排序 | focus 区内拖拽 | 拖拽手柄 |
| 结转 | 顶部横幅 | `Enter` 确认 |

**软限制 UX**：超过 5 项时顶部出现**温和**提示「已选 6 项，聚焦过多可能降低完成率」——可关闭，不阻断。

**跨日行为**

- 每日 0 点本地：focus 表不自动复制；启动时若存在「昨日未完成 focus」→ 显示结转横幅（一次/日）
- 用户选「忽略」→ 写入 `settings.lastFocusCarryDismissedDate`

### 4.3 推迟显示交互

**入口（统一组件 `DeferPicker`）**

- TaskDetailPanel 区「推迟显示」
- TaskRow 右键 / 快捷菜单：明天、下周一、自定义…
- QuickWindow 命令：`推迟` + 选中任务

**DeferPicker 预设**

| 选项 | availableAt |
| --- | --- |
| 明天 | today+1 |
| 下周一 | 下个周一 |
| 下周 | today+7 |
| 自定义 | 日期选择器 |
| 取消推迟 | null |

执行后 toast + recent-actions：`推迟显示至 8/20`，undo 恢复 null。

**已推迟视图**

- Tasks 页筛选 chip「已推迟」+ 智能列表
- 空状态：「没有推迟的任务 — 推迟显示可以让任务暂时让路，到日期会自动回来。」

### 4.4 等待事项交互

**进入等待**

- TaskDetailPanel：切换「等待中」→ 展开 `waitingFor` 输入 + 可选跟进日
- 快捷：右键「标记等待…」弹轻量表单（单屏完成）

**等待中任务展示**

- 列表行：muted 图标 `⏸` + 「等待：{waitingFor}」
- 不在收件箱/今日 due 区出现

**跟进日到期**

- 出现在 Today「等待跟进」区（非 due 区）
- 行内动作：**结束等待** | **继续等待**（改跟进日）| **完成** | **打开详情**

**结束等待**

- 清除 `workflow_state` → active；保留 `waitingFor` 历史在 notes 可选追加（默认不追加，避免污染）

### 4.5 与「延期」的区分（极致可理解性）

TaskDetailPanel 中**并列但分区**：

```
截止日期     [ date ] [ time ]     ← 这件事什么时候到期
推迟显示     [ DeferPicker ]       ← 什么时候再出现在列表里
```

首次使用推迟时一次性 coach mark（可永久关闭）：「推迟显示不会修改截止日期。」

### 4.6 键盘模型（须写入 keyboard-shortcuts.md）

Today 页列表焦点时：

| 键 | 动作 |
| --- | --- |
| `F` | 切换今日重点 |
| `D` | 打开 DeferPicker |
| `W` | 标记/编辑等待 |
| `Space` | 完成/取消完成（已有） |
| `↑↓` | 移动选择（已有） |
| `⌘Z` / 撤销 toast | recent-actions pop |

### 4.7 视觉（遵循 ui-layout-interaction.md）

- 今日重点：左侧 2px accent 竖条，无整行背景色
- 等待：muted 图标，不用红色
- 推迟：仅在「已推迟」视图显示「8/20 再显示」小字
- 禁止：OVERDUE 计数大红徽章、未完成 guilt 文案

---

## 5. 后端实现要点

### 5.1 修改面

| 层 | 文件 |
| --- | --- |
| migration | `0010_gtd_workflow.sql` |
| domain | `task.rs`, 新 `task_activity.rs` |
| application | `tasks.rs`：`query_tasks`, `today_tasks`, 新 focus 服务 |
| commands | `mod.rs`, `lib.rs` 注册 |
| frontend | `TodayPage`, `TaskDetailPanel`, `TaskRow`, `TasksPage`, `client.ts` |

### 5.2 `query_tasks` 改造

所有 WHERE 子句通过 `ActiveTaskPredicate` 生成 SQL fragment，避免复制。

`deferredOnly`: `available_at IS NOT NULL AND available_at > ?today`

`search`: **不受** availableAt 过滤（搜索必须能找到推迟任务 — quality-bar 找回成本）

### 5.3 `today_tasks` 改造

1. overdue / due_today 查询增加 active 谓词
2. 追加 focus 查询（join daily_focus）
3. 追加 waiting_follow_up 查询
4. focus_carry_suggestions：查昨日 focus 中仍 todo 且不在今日 focus 的项

### 5.4 撤销栈契约

以下操作必须 `useRecentActions.push`：

- daily_focus_add/remove/reorder
- task_set_defer
- task_set_waiting / clear_waiting

undo 调用对应逆命令。

---

## 6. 边界与不做

- 子任务/检查项（另任务）
- 自动把 due 最近任务 pin 到 focus（仅**建议**结转昨日 focus）
- 等待对象的联系人系统 / 发消息
- 批量推迟（留给 daily-wrap 子任务）

---

## 7. 测试策略

### 7.1 Rust 单测（必须）

- `is_active_task` 全组合表（property-style 枚举）
- `dueDate < availableAt` 保存拒绝
- focus 跨日结转逻辑
- waiting + follow_up 出现在 today 的正确分区
- migration 0009→0010 升级后旧任务行为不变

### 7.2 前端 vitest

- DeferPicker 预设日期计算（时区边界用固定 locale mock）
- focus 键盘 `F` handler（组件测试）

### 7.3 手动极致验收

- [ ] S1–S4 逐步键击计数并记录
- [ ] 升级安装：有 500+ 任务时 Today 首屏 <200ms
- [ ] 搜索推迟任务标题 1 步命中

---

## 8. 风险

| 风险 | 缓解 |
| --- | --- |
| 筛选语义不一致 | 强制 `task_activity.rs` 单测 + grep CI 检查无裸 SQL |
| Today 页区块过多 | 完成区默认折叠；等待区无项时隐藏 |
| 用户混淆推迟 vs 延期 | DeferPicker coach mark + 详情分区 + 文档 |

---

## 9. 开放问题（implement 前确认）

1. **结转默认行为**：建议「提示而非自动」，已写入 §4.2 — 无需用户决策
2. **focus 是否可含 waiting 任务**：建议 **否**；标记等待时自动 remove focus — 需在 UI 明确 toast

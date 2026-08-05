# 技术设计：模板能力补齐

## 1. 通用创建表单（SettingsPage 模板区）

在「添加示例」按钮旁新增「新建模板」表单（折叠式，点击展开）：

```
名称 [____]  类型 [task|reminder|memory]
task:     标题 [____]  相对截止天数 [0]  优先级 [none|low|medium|high]
reminder: 标题 [____]  相对触发小时 [0]  每天重复 [x]
memory:   标题 [____]  正文 [____]
[创建]
```

- 提交组装 payload：
  - task：`{ title, relativeDueDays, priority }`
  - reminder：`{ title, relativeFireHours, recurrence: recurring ? {version:1, frequency:"daily", interval:1, timezone} : undefined }`
  - memory：`{ title, body }`
- 调 `ipc.templateCreate({ kind, name, payload })`，成功失效 `["templates"]` + `setMessage`。

## 2. 应用前预览确认

- state：`previewing: ItemTemplate | null`、`previewData: TemplatePreview | null`。
- 列表「应用」onClick → `ipc.templatePreview(tpl.id)` → 存 `previewData`，渲染预览面板：
  - 头部：模板名 + 类型标签；
  - 主体：标题、正文（body 非空时）、截止/触发时间（dueDate/dueTime/fireAt）、优先级、标签、周期（recurrence 可读文案）；
  - 底部：「取消」（关闭）与「确认应用」（`ipc.templateApply` → 失效 `["templates"]` + `["tasks"]`/`["reminders"]`/`["memories"]` + `setMessage` + 关闭面板）。
- 预览面板用 AttachmentsSection/Onboarding 同款遮罩 + 面板样式（fixed inset-0 z-50 + bg-surface rounded border）。

## 3. 复用与边界

- 不新增后端命令；`templatePreview`/`templateApply`/`templateCreate` 均已存在。
- QuickWindow 命令面板一键应用保留（快路径，PRD 已注明边界）。
- 保留「添加示例：周报/报销」按钮（快捷示例），通用表单与其并存。
- 类型选择切换时清空对应字段值，避免 payload 串味。

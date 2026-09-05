# Journal - zouyingjun (Part 1)

> AI development session journal
> Started: 2026-07-30

---



## Session 1: 今日页底部快速输入任务 & 修复 confirm 失效

**Date**: 2026-08-05
**Task**: 今日页底部快速输入任务 & 修复 confirm 失效
**Branch**: `main`

### Summary

今日页内容区底部新增快速输入任务入口（nlParseCapture 自然语言解析，回车创建，默认截止今天）；修复 Tauri webview 中 window.confirm/alert 失效导致归档/删除不可用，统一改为 ConfirmButton 两步内联确认，并沉淀为前端组件规范

### Git Commits

| Hash | Message |
|------|---------|
| `04c02d2` | (see git log) |
| `aedae3f` | (see git log) |
| `a12eda1` | (see git log) |

### Status

[OK] **Completed**


## Session 2: v1.4 统一动作层并发布 v1.4.0

**Date**: 2026-08-18
**Task**: v1.4 统一动作层并发布 v1.4.0
**Branch**: `feat-improve`

### Summary

完成动作层 / trove-cli / 规则自动化 / 快捷指令查询 / 任务 CSV，合并 PR #10 并发布 v1.4.0；补上 default-run 与 universal lipo 后安装包才出齐。

### Main Changes

- 统一 WorkbenchAction 与 trove-cli、规则自动化、快捷指令 query、任务 CSV 导入导出
- 合并 PR #10，在 main 上 release.sh minor 打出 v1.4.0
- 修复 default-run、universal lipo、beforeBundleCommand 路径后发版流水线转绿
- 把发版与动作层合同写入 .trellis/spec/tauri/ 并归档任务 08-17-v1-4-action-layer

### Git Commits

| Hash | Message |
|------|---------|
| `c2fb1f8` | (see git log) |
| `34577ef` | (see git log) |
| `82796c3` | (see git log) |
| `afdb004` | (see git log) |
| `b9ef347` | (see git log) |
| `a9c894c` | (see git log) |
| `f7f19fa` | (see git log) |
| `4ebd3e3` | (see git log) |
| `f52648d` | (see git log) |
| `8892c69` | (see git log) |
| `bcc5633` | (see git log) |

### Testing

- [OK] cargo test --lib 108/108；PR #10 frontend/rust CI 绿
- [OK] GitHub Release v1.4.0 含 dmg、exe/msi、tar.gz+.sig、latest.json（darwin + windows）

### Status

[OK] **Completed**

### Next Steps

- 如需让 main 也没有活跃任务，把 feat-improve 上的 spec/archive/journal 提交合入 main


## Session 3: v2.0 启动评估与路线图规划

**Date**: 2026-08-19
**Task**: v2.0 启动评估与路线图规划
**Branch**: `feat-improve`

### Summary

对齐 main 的 v1.4.0 后，完成 §9.1 启动门槛评估（门槛1/2 满足、3 冻结数据边界、4 转为切片1 AC），创建父任务 v2-0-ai-assist-roadmap 与切片 1 v2-0-ai-service-boundary 的 PRD，发布 docs/v2-ai-assist-roadmap.md

### Git Commits

| Hash | Message |
|------|---------|
| `3476f53` | (see git log) |

### Status

[OK] **Completed**


## Session 4: v2.0 切片 1：AI 服务边界实现交付

**Date**: 2026-08-21
**Task**: v2.0 切片 1：AI 服务边界实现交付
**Branch**: `feat-improve`

### Summary

实现 0019 迁移、AIConfig/domain 校验、Off+Http provider（key 库外文件隔离）、AISuggestionService 管线（sanitize 红线/结构化校验/审计台账）、7 个 IPC 命令、设置页智能辅助区块与记忆敏感标记、ai-eval 固定样本与离线回归 runner；docs/ai-assist.md 与 privacy 增补。133 lib + 8 eval + 42 前端测试全绿（1 个已知无关 flaky）

### Git Commits

| Hash | Message |
|------|---------|
| `4075ae2` | (see git log) |

### Status

[OK] **Completed**


## Session 5: v2.0 切片 2：长文本提取任务草稿

**Date**: 2026-08-22
**Task**: v2.0 切片 2：长文本提取任务草稿
**Branch**: `feat-improve`

### Summary

交付记忆详情→AI 提取→草稿勾选→批量创建全链路：pending 幂等（不重复打 provider）、apply 门禁与来源引用/EntityLink、ambiguous 日期不猜测写入；ExtractSuggestionsPanel UI；新增 6 服务单测 + 在线评估用例；139 lib + 8 eval + 42 前端测试全绿

### Git Commits

| Hash | Message |
|------|---------|
| `55b177b` | (see git log) |

### Status

[OK] **Completed**


## Session 6: v2.0 切片 3：每周回顾 AI 摘要

**Date**: 2026-08-23
**Task**: v2.0 切片 3：每周回顾 AI 摘要
**Branch**: `feat-improve`

### Summary

交付 weekly_review AI 摘要：Summary prompt 开放（禁评价约束固化）、request_weekly_summary（确定性数字+仅标题上下文、重新生成置 dismissed）、完成回顾清理 pending、WeeklySummaryCard 三态 UI + 数字徽标对照；142 lib + 9 eval 全绿

### Git Commits

| Hash | Message |
|------|---------|
| `14ae648` | (see git log) |

### Status

[OK] **Completed**


## Session 7: v2.0 切片 4：任务相关内容建议

**Date**: 2026-08-23
**Task**: v2.0 切片 4：任务相关内容建议
**Branch**: `feat-improve`

### Summary

交付任务→记忆/剪贴板相关推荐：CJK bigram 候选检索 + 三重过滤 + 模型标题精确回配防编造；逐条 关联（幂等 related 链接）/不相关（拒绝降频）UI；146 lib + 10 eval + 42 前端测试全绿

### Git Commits

| Hash | Message |
|------|---------|
| `7f3627f` | (see git log) |

### Status

[OK] **Completed**


## Session 8: v2.0 切片 5：今日工作建议

**Date**: 2026-08-23
**Task**: v2.0 切片 5：今日工作建议
**Branch**: `feat-improve`

### Summary

交付今日页 AI 工作建议：确定性候选池（排除重点/等待/跳过配对）+ 特征引用理由 + 回配防编造；加入重点走既有撤销链路；跨天自动收口；顺手修复 reject 配对 feature 硬编码 bug；150 lib + 11 eval + 42 前端全绿

### Git Commits

| Hash | Message |
|------|---------|
| `18f7ba8` | (see git log) |

### Status

[OK] **Completed**


## Session 9: v2.0 切片 6：任务检查项数据模型

**Date**: 2026-08-23
**Task**: v2.0 切片 6：任务检查项数据模型
**Branch**: `feat-improve`

### Summary

交付一层检查项模型：0020 迁移、TaskService 五方法（冻结/级联/归一化/上限）、检查项并入任务搜索索引、ChecklistSection UI + TaskRow 进度徽标；为切片 7 AI 任务拆分铺路；153 lib + 11 eval + 42 前端全绿

### Git Commits

| Hash | Message |
|------|---------|
| `af39963` | (see git log) |

### Status

[OK] **Completed**


## Session 10: v2.0 切片 7：AI 任务拆分

**Date**: 2026-08-23
**Task**: v2.0 切片 7：AI 任务拆分
**Branch**: `feat-improve`

### Summary

交付任务→检查项 AI 拆分：依据子串防编造校验、薄任务/完成冻结短路、apply 零任务字段写入；复用切片 6 检查项模型；156 lib + 12 eval + 42 前端全绿

### Git Commits

| Hash | Message |
|------|---------|
| `c55581e` | (see git log) |

### Status

[OK] **Completed**


## Session 11: v2.0 切片 8 语义检索 + 全路线图收官

**Date**: 2026-08-24
**Task**: v2.0 切片 8 语义检索 + 全路线图收官
**Branch**: `feat-improve`

### Summary

交付语义检索（0021 向量索引/embed 批量/排除范围与上限/双列展示/索引管理）并归档全部 8/8 切片与父任务；v2-ai-assist-roadmap 标记已全部交付；164 lib + 13 eval + 42 前端全绿

### Git Commits

| Hash | Message |
|------|---------|
| `83f75aa` | (see git log) |

### Status

[OK] **Completed**


## Session 12: v2.0.0 发版：PR 合并、CI 修复与 Release 流水线

**Date**: 2026-08-24
**Task**: v2.0.0 发版：PR 合并、CI 修复与 Release 流水线
**Branch**: `feat-improve`

### Summary

修复历史 reminder_stats flaky（相对日期化）后 PR #11/#12 CI 双绿合并；scripts/release.sh major 发布 v2.0.0（schema v21），macOS universal + Windows 安装包与更新器 latest.json 全部出齐，Release notes 已挂载

### Git Commits

| Hash | Message |
|------|---------|
| `98c67df` | (see git log) |

### Status

[OK] **Completed**


## Session 13: v2.0 真实模型在线回归与 v2.0.1 发版

**Date**: 2026-08-24
**Task**: v2.0 真实模型在线回归与 v2.0.1 发版
**Branch**: `feat-improve`

### Summary

本地 Ollama（gemma3 + bge-m3）跑通 7 项在线回归，发现并修复三个真实问题（回配《》包裹误杀、20s 超时临界抖动、ambiguous 缺字段）；PR #14 合并后发布 v2.0.1（全平台安装包 + 更新器产物）；docs 建议 bge-m3 为中文 embedding 首选

### Git Commits

| Hash | Message |
|------|---------|
| `420baa6` | (see git log) |

### Status

[OK] **Completed**

# Research: v2.0 启动门槛评估与技术选型

- **Query**: `post-v1-iteration-design.md` §9.1 四条启动门槛是否满足；本地/远程模型集成技术选型
- **Scope**: internal（代码勘察 + 既有调研报告复核）+ 技术方案桌面调研
- **Date**: 2026-08-19
- **产出**: 本文件（startup-gate-assessment.md），供父任务 `prd.md` 与切片 1 PRD 引用

---

## 1. §9.1 启动门槛逐条评估

### 门槛 1：v1.x 核心数据模型和动作层稳定 — **✅ 满足**

- 数据模型：tasks / reminders / memories / clipboard / assets / entity_links / automations 等表自 v1.2 起无破坏性变更，迁移链完整（`db/mod.rs` MIGRATIONS + 迁移前自动备份 + `PRAGMA quick_check` 恢复校验）。
- 动作层：v1.4.0 已交付 `WorkbenchAction`（`domain/workbench_action.rs`）统一导航/搜索/创建预览，`trove://`、`trove-cli`、规则自动化（`application/automation.rs`）三层共用，108 个 Rust 单测全绿。
- 结论：AI 建议层可以直接挂接在动作层之上（建议 → 用户确认 → dispatch），复用已有确认门禁。

### 门槛 2：已观察到仅靠关键词搜索、固定规则或模板无法解决的高频问题 — **✅ 满足（代码级证据）**

规则类能力边界（当前实现）：

| 能力 | 现状 | 无法解决的高频问题 |
| --- | --- | --- |
| NL 解析（`domain/nl_parse.rs`） | 确定性关键词 strip（"每天"/"工作日"/"p1"…），模糊日期标记 ambiguous 不猜测（设计如此） | 会议记录/邮件/长文本中提取**多个**候选任务、负责人、相对日期；"下周三之前找老张确认合同"这类组合语义 |
| 搜索（`application/search.rs`） | SQLite FTS5 关键词 MATCH + LIKE 兜底 | 语义检索：搜"报销流程"找不到标题为"差旅票据整理"的记忆；OCR 文本口语与查询词不一致 |
| 今日建议（`today_smart_sort`，v1.3.x） | 确定性排序 + 固定理由文案（"今天到期"/"已延期 2 次"） | 跨模块关联建议（"这个任务和你上周的记忆 X 有关"超出规则能力，`links.rs` 仅做显式 EntityLink） |
| 模板 / 规则自动化 | 用户手写模板 + 触发器-动作规则 | 内容本身的理解与组织（周报复盘文字、任务拆分检查项） |

用户调研佐证（`08-08-user-feedback-research/research/user-feedback-report.md`）：

- §3.1 记录成本：Raycast #15993 要求 NLP 快速录入（`tomorrow 3pm #Work p1`）——短捕获 NL 已覆盖，长文本未覆盖。
- §3.3 找回成本："搜不到 = 没存过"（★★★★★）——语义检索是品类级刚需。
- §3.4 维护成本："每周复盘别像苦差"——weekly_review 已有确定性统计，组织成自然文字超出模板能力。

### 门槛 3：能清楚说明哪些数据本地处理、哪些发送远程 — **⚠️ 需设计冻结（本任务产出）**

当前代码无任何远程模型调用。需在切片 1 冻结数据边界（见 §3 选型 + 切片 1 PRD）：

- **永远本地**：分类规则、确定性统计、建议合并/排序、确认门禁、审计记录。
- **发送前必须过滤**：密码管理器来源剪贴板、标记敏感的记忆、排除应用列表内容（`sanitize_context` 过滤器）。
- **可能发送远程**（仅当用户选择远程 provider）：任务/记忆标题与正文片段、OCR 文本，最小必要上下文。
- 用户可见：设置页展示当前 provider 与数据去向，可随时关闭。

### 门槛 4：AI 失败或关闭时基础工作流完整可用 — **✅ 架构性满足（需切片 1 兑现）**

设计约束（写入切片 1 AC）：`AIService` 未配置/调用失败 → 全部 v1.x 功能路径不经过 AI 分支；建议入口隐藏或显示降级文案，无静默失败。

**综合结论：门槛 1/2 满足，门槛 3 由本路线图冻结，门槛 4 作为切片 1 验收标准。可以进入 v2.0 规划。**

---

## 2. §8 版本进入流程核对（五步）

1. **上一版本 3 个使用阻力**（来自 v1.4 验收与用户调研）：
   a. 长文本（会议/邮件）→ 任务仍需手工逐条拆；
   b. 语义找不回（关键词搜不到换个说法的内容）；
   c. 复盘/建议文字组织成本高（统计有了，人话要自己写）。
2. **对应五类成本**：a→记录成本；b→找回成本；c→维护成本。均命中用户调研 ★★★★☆ 以上指标。
3. **范围冻结**：见父任务 `prd.md`「任务地图」与「明确不做」。
4. **数据迁移/权限/降级**：切片 1 新增 `ai_suggestions` 表与 provider 配置；无新系统权限（本地 Ollama 无权限，远程仅网络）；降级 = 完全关闭。
5. **验收用例**：§9.6 要求固定评估样本（日期提取/任务提取/搜索相关性），切片 1 建立 `fixtures/ai-eval/` 样本集，后续切片复用。

---

## 3. 技术选型：模型服务边界

### 3.1 方案对比

| 方案 | 集成方式 | 体积/构建影响 | 隐私 | 结论 |
| --- | --- | --- | --- | --- |
| **A. 本地 Ollama（HTTP）** | reqwest 调 `localhost:11434`，OpenAI-compatible | 零（用户自装 Ollama） | 全本地 | **v2.0 起步方案**：无二进制负担，CI 不受影响，mac/win 一致 |
| B. 内嵌推理（llama.cpp / candle / onnx） | Rust FFI / 纯 Rust | 二进制 +30~80MB，universal mac + Windows CI 复杂度大，模型分发/许可问题 | 全本地 | 暂不内嵌；若 v2.x 需要开箱即用再评估 ONNX 小模型（分类/嵌入场景） |
| C. 远程 OpenAI-compatible | 用户配置 base_url + key + model | 零 | 数据出机器，需 §9.4 红线 | **可选 provider**，默认关闭 |
| D. 供应商 SDK（openai crate 等） | 编译依赖 | 中 | 同 C | 不用：锁死供应商；OpenAI-compatible HTTP 已覆盖 Ollama/LM Studio/OpenAI/DeepSeek/本地网关 |

**决策**：统一 provider 抽象 = "OpenAI-compatible HTTP endpoint"（本地 Ollama 或远程皆可）+ `off`。用户在设置页三选一：本地 / 自定义远程 / 关闭。

### 3.2 服务边界设计（§9.5 合同）

```
UI/业务模块 ──请求建议──▶ AISuggestionService（application 层）
                              │ 1. sanitize_context（敏感过滤，纯本地）
                              │ 2. 组装最小上下文 prompt
                              │ 3. 结构化输出校验（JSON schema，失败即丢弃）
                              ▼
                         AIProvider trait（infrastructure 层）
                              ├─ OffProvider（返回 None，零开销）
                              └─ HttpProvider（OpenAI-compatible，可配置超时/重试）
                              │ 4. 落库 AISuggestion + SuggestionSource（可审计、可拒绝）
                              ▼
UI ◀──结构化建议（草稿/候选/摘要）──  用户逐项确认 ──▶ WorkbenchActionService（复用 v1.4 确认门禁）
```

关键合同：

- 业务模块**只接收结构化 `AISuggestion`**，自由文本不允许直接变成数据库写操作。
- 每条建议携带 `SuggestionSource`（引用的实体 id + 文本位置），UI 必须可跳原文。
- provider 请求/响应正文**不写入普通日志**；审计只记录提供方、模型、功能类型、接受/拒绝（§9.4）。
- 派生数据（建议历史、未来语义索引）**不纳入主备份**，可一键清空（§10）。

### 3.3 风险与缓解

| 风险 | 缓解 |
| --- | --- |
| Ollama 未安装/未运行 | 探活失败 → 设置页给出安装指引 + 功能入口显示"AI 未配置"降级文案（语气遵循 empty-states-and-permissions.md） |
| 模型输出不稳定/幻觉 | 结构化 schema 校验 + 日期字段强制 ambiguous 标记 + 逐项确认；固定评估样本回归 |
| 远程 key 泄露 | key 存本机（keyring 或本地配置文件），不进备份导出、不进日志 |
| 功能蔓延成"聊天助手" | §9.7 红线：不建聊天主页；每切片 PRD 明确"不做"清单 |

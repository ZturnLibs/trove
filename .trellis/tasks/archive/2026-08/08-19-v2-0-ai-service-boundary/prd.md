# v2.0 AI 服务边界（切片 1）

> 父任务：`08-19-v2-0-ai-assist-roadmap`（全局合同见父 PRD，本切片必须全部满足）

## Goal

建立 v2.0 全部 AI 功能共用的服务边界：provider 抽象、`AISuggestion` 数据模型、设置页授权/敏感范围/审计、固定评估样本。本切片**不含任何面向用户的 AI 建议功能**——交付的是"可审计、可关闭、可降级"的地基。

## Requirements

1. **Provider 抽象**（`infrastructure` 层）
   - `AIProvider` trait：`probe()`（探活/连通性）、`complete(request) -> Option<StructuredOutput>`。
   - 实现 `OffProvider`（默认，恒返回 None，零开销）与 `HttpProvider`（OpenAI-compatible，base_url + model + api_key + 超时）。
   - 探活：设置页「测试连接」按钮；Ollama 未运行给出安装指引（文案遵循 empty-states-and-permissions.md）。
2. **数据模型**（新迁移）
   - `ai_suggestions`：id、feature_type（extract/related/summary/suggest/split）、source_entity_type/id、payload（结构化 JSON）、status（pending/accepted/rejected/dismissed）、provider、model、created_at、decided_at。
   - `SuggestionSource`：建议引用的实体 id + 文本位置（offset/摘要片段），随 payload 落库。
   - 用户级配置：provider 模式、endpoint、model、敏感范围（标记敏感的记忆列表引用、永不发送的来源），存本地配置，**key 不进备份导出**。
3. **AISuggestionService**（`application` 层）
   - `sanitize_context`：按 §9.4 红线过滤（密码管理器来源、敏感记忆、排除应用内容）后再组装最小上下文。
   - 结构化输出校验：JSON schema 不符即丢弃并记录审计（不进普通日志）。
   - 审计：仅记录 provider/model/feature_type/接受与否；prompt 与完整输出不落普通日志。
4. **设置页**：AI 区块（模式三选一 off/本地/自定义远程、测试连接、数据去向说明、敏感范围管理、建议历史一键清空、审计查看）。
5. **评估样本**：`src-tauri/tests/fixtures/ai-eval/` 固定样本集（中文日期提取、任务提取、模糊表述各 ≥10 条），附确定性回归 runner（不依赖真实模型，校验 sanitize/schema/降级路径）。

## Acceptance Criteria

- [x] provider=off 时全部 v1.x 功能路径不经过 AI 分支；探活失败仅显示降级文案（门槛 4 实测）
- [x] 敏感内容经 `sanitize_context` 后绝不出现在 provider 请求中（单测用敏感样本验证）
- [x] schema 校验失败的模型输出被丢弃且审计可见，业务数据零污染
- [x] key 不出现在备份导出、日志、`ai_suggestions` 表（key 存库外文件，eval 断言不入 settings JSON）
- [x] 建议历史可一键清空；清空不影响任何原始数据
- [x] `cargo test` 全绿（新增：sanitize 边界、schema 校验、OffProvider 零调用、key 隔离；已知无关 flaky 见任务 notes）
- [x] docs：`docs/ai-assist.md`（配置指引 + 数据边界说明）+ privacy-and-data.md 增补 AI 小节

## 明确不做（本切片）

- 任何用户可见的 AI 建议功能（extract/related/summary 等属切片 2+）
- 内嵌模型推理（llama.cpp/candle/onnx）
- 语义向量索引
- 流式输出 / 多轮对话

## 依据

- `docs/post-v1-iteration-design.md` §9.4 隐私与安全、§9.5 数据与技术影响
- 父任务 `research/startup-gate-assessment.md` §3 服务边界设计

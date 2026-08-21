# Implement: v2.0 AI 服务边界（切片 1）

> 每步完成后跑对应验证命令；每步独立 commit（回滚点）。全部完成后走 Phase 3 检查单。

## 1. 数据层

- [ ] `migrations/0019_ai_suggestions.sql`（表 + `memories.sensitive` 列 + 两索引），`db/mod.rs` MIGRATIONS 追加、`schema_version` 断言 18→19
- [ ] `AppSettings.ai: AIConfig`（serde default 全关：Off 模式、features 五开关 false）
- 验证：`cargo test --lib db::`（迁移链 0001→0019 逐级 + 旧设置 JSON 反序列化默认值）

## 2. 领域层

- [ ] `domain/ai_suggestion.rs`：`AISuggestion` / `SuggestionSource` / `CompletionRequest` / `CompletionOutput` / `AIFeature` / prompt 模板常量
- [ ] `validate()`：枚举合法、日期格式或 `ambiguous=true`、来源引用非空；单测覆盖坏输出拒绝路径
- 验证：`cargo test --lib domain::ai_suggestion`

## 3. Provider（infrastructure/ai）

- [ ] `AIProvider` trait（async probe/complete）+ `OffProvider`（恒 None）
- [ ] `HttpProvider`：reqwest 直依赖（`Cargo.toml`，features=["json"]）、OpenAI-compatible、20s 超时、失败 None、日志只记元信息
- [ ] key 文件读写（`app_data_dir/ai_provider_key`，0600；exists/clear；内容不回读）
- 验证：`cargo test --lib infrastructure::ai`（Off 零调用、key 隔离）；`cargo build` 确认无破坏

## 4. 服务层（application/ai_suggestions.rs）

- [ ] `sanitize_context`：sensitive memory / excluded_apps ∪ default_excluded_apps 来源过滤（单测含敏感样本）
- [ ] `request()`：feature 开关短路 → sanitize → provider → validate → 落库 pending / 坏输出落 dismissed 审计行
- [ ] `list / decide / clear_history`
- [ ] `AppState` 注入 `Arc<AISuggestionService>`（provider 以 `Arc<dyn AIProvider>` 注入，测试用替身）
- 验证：`cargo test --lib application::ai_suggestions`

## 5. IPC + 前端

- [ ] 命令：`ai_config_get/save`、`ai_provider_key_set/clear`、`ai_provider_probe`、`ai_suggestion_list/decide/clear`；`memory_update` 加 `sensitive`
- [ ] `client.ts` 封装；设置页「智能辅助」区块（模式/测试连接/key/feature 开关/建议历史/清空两步确认/空状态文案）；MemoryPage「标记敏感」
- 验证：`pnpm test:unit`、`pnpm build`、`cargo clippy -- -D warnings`

## 6. 评估样本

- [ ] `tests/fixtures/ai-eval/`：extract_dates / extract_tasks / search_semantics 各 ≥10 条（含敏感样本）
- [ ] 离线 runner：sanitize 命中、schema 拒绝、OffProvider 全链、关 feature 零调用、key 不入 settings 导出
- [ ] `#[ignore]` 在线档（Ollama 手动回归）
- 验证：`cargo test`（含 fixtures runner）

## 7. 文档与收尾

- [ ] `docs/ai-assist.md`（配置指引 + 数据边界 + 关闭/清空说明）
- [ ] `docs/privacy-and-data.md` 增补「智能辅助与数据边界」小节；`docs/README.md` 索引
- [ ] PRD AC 逐条勾选；对照父任务全局合同 7 条复核
- 验证：`cargo test` 全绿 + `pnpm test:unit` + `pnpm build` + 手动走查（off 默认态无 AI 痕迹；标记敏感记忆在 sanitize 测试中可见被过滤）

## Review gates

1. 步骤 3 后：确认无日志/payload 泄漏（代码审查点）
2. 步骤 5 后：设置页文案语气审查（对照 empty-states-and-permissions.md）
3. 步骤 7 后：父任务全局合同逐条勾选，才可 `task.py finish` + archive

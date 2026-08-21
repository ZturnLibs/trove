# Design: v2.0 AI 服务边界（切片 1）

> 依据：PRD + 父任务 `research/startup-gate-assessment.md` §3 服务边界设计
> 架构事实基线：迁移最新 `0018`；`AppSettings` 单 JSON 存 settings 表（serde default 向后兼容）；密码管理器名单已有 `domain/clipboard.rs default_excluded_apps()`；导出白名单 `data_port.rs EXPORT_TABLES`；设置页 section 模式（SettingsPage.tsx:364+）；reqwest 需在 Cargo.toml 显式添加（lock 中已有传递依赖）。

## 1. 模块边界

```
domain/ai_suggestion.rs          类型 + 结构化输出校验（无 I/O）
infrastructure/ai/mod.rs         AIProvider trait + OffProvider + HttpProvider + key 文件存取
application/ai_suggestions.rs    AISuggestionService：sanitize / request / record / decide / clear
commands（mod.rs 追加）          ai_config_* / ai_provider_probe / ai_suggestion_*
migrations/0019_ai_suggestions   ai_suggestions 表 + memories.sensitive 列
src/features/settings/           「智能辅助」区块 + 建议历史
src/features/memory/MemoryPage  「标记敏感」开关
tests/fixtures/ai-eval/          评估样本 + 离线回归 runner
```

**依赖方向**：commands → application → domain；infrastructure 仅被 application 经 trait 对象引用（`Arc<dyn AIProvider>` 注入 service，与 `Clock` 注入模式一致，便于测试替身）。

## 2. 数据模型

### 2.1 迁移 `0019_ai_suggestions.sql`

```sql
CREATE TABLE ai_suggestions (
  id TEXT PRIMARY KEY,
  feature_type TEXT NOT NULL,            -- extract | related | summary | suggest | split
  source_entity_type TEXT NOT NULL,      -- memory | task | clipboard | review
  source_entity_id TEXT NOT NULL,
  payload TEXT NOT NULL,                 -- 结构化建议 JSON（含 SuggestionSource[]）
  status TEXT NOT NULL DEFAULT 'pending',-- pending | accepted | rejected | dismissed
  provider TEXT NOT NULL,                -- off | ollama | custom
  model TEXT NOT NULL,
  created_at TEXT NOT NULL,
  decided_at TEXT
);
ALTER TABLE memories ADD COLUMN sensitive INTEGER NOT NULL DEFAULT 0;
```

- `SuggestionSource { entityType, entityId, textOffset, excerpt }` 存于 payload JSON 内，与建议一体导出/审计。
- 索引：`(feature_type, status)`、`(source_entity_id)`。

### 2.2 配置存储（三层分离）

| 数据 | 位置 | 理由 |
| --- | --- | --- |
| AI 模式/endpoint/model/feature 开关 | `AppSettings.ai`（serde default 全关） | 随既有 settings 表，升级零迁移 |
| **api_key** | `app_data_dir/ai_provider_key`（0600 纯文本，与库分离） | settings 表随整库备份/迁移走，key 必须隔离；导出白名单 `EXPORT_TABLES` 不含 ai_suggestions（天然不导出），key 文件更不在库内 |
| 敏感范围 | `memories.sensitive` 字段 + 既有 `clipboard_excluded_apps` + `default_excluded_apps()` | 敏感随条目走、可导出语义（导出含 sensitive 标记，恢复后仍受保护） |

`AppSettings` 新增（全部 `#[serde(default)]`，旧库/旧导出兼容）：

```rust
pub struct AIConfig {
  pub mode: AIMode,               // Off（默认）| Ollama | Custom
  pub ollama_url: String,         // 默认 http://localhost:11434
  pub custom_endpoint: String,    // OpenAI-compatible base_url
  pub custom_model: String,
  pub features: AIFeatureToggles, // extract/related/summary/suggest/split 全默认 false
}
```

## 3. Provider 抽象（infrastructure/ai）

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    async fn probe(&self) -> ProbeReport;                  // 连通性 + 模型名 + 延迟
    async fn complete(&self, req: CompletionRequest) -> Option<CompletionOutput>;
}
```

- **OffProvider**：两个方法恒返 None/离线报告，零网络零开销——门槛 4 的实现载体。
- **HttpProvider**：reqwest（新直接依赖，`features=["json"]`），OpenAI-compatible `POST {base}/v1/chat/completions`，`response_format: json_object` 尽力设置；超时 20s；失败返 None（不重试，建议类场景无必要）。Ollama 模式即 `ollama_url` 的 HttpProvider，无第三实现。
- Key 读取：每次调用时读文件（用户换 key 免重启），读不到等同未配置 → None。
- 命令层用 `tauri::async_runtime` 驱动 async；**provider 请求/响应正文不落日志**，仅 `tracing::info` 记录 provider/model/feature/字节数。

## 4. AISuggestionService（application）

```
sanitize_context(items) -> Vec<ContextItem>
    过滤：memory.sensitive == true；source_app ∈ excluded_apps ∪ default_excluded_apps()；payload 为空
request(feature, entity, context) -> Option<AISuggestionRow>
    1. features.<feature> 开关 off → None（零 provider 调用）
    2. sanitize → 空则 None
    3. 组装最小上下文 prompt（模板常量，位于 domain/ai_suggestion.rs）
    4. provider.complete → CompletionOutput（原始 JSON 文本）
    5. 结构化校验：serde 解析 + validate()（枚举合法/日期格式或标记 ambiguous/来源引用存在）
       失败 → 丢弃，落一行 status='dismissed', payload={"invalid":true} 供审计，业务零污染
    6. 成功 → 落库 pending 行，返回
decide(id, accept|reject|dismiss) / list(filter) / clear_history()
```

**合同落点**：
- 业务模块只拿 `AISuggestionRow`（结构化），自由文本进不了写路径（§9.5）。
- `validate()` 强制：日期字段必须带 `ambiguous: bool`；未确认 ambiguous 日期在 UI 侧标记待确认（切片 2 消费）。
- 清空建议历史 = `DELETE FROM ai_suggestions`，不动任何业务表。

## 5. 命令面（lib.rs 注册）

| 命令 | 说明 |
| --- | --- |
| `ai_config_get` / `ai_config_save` | 读写 AppSettings.ai（save 走既有 settings_update 全量路径） |
| `ai_provider_key_set` / `ai_provider_key_clear` | key 文件写/删（内容不回读，UI 用「已设置」状态位 `ai_provider_key_exists`） |
| `ai_provider_probe` | 探活，返回 ProbeReport（模式、可达、模型、延迟、指引文案 key） |
| `ai_suggestion_list` / `ai_suggestion_decide` / `ai_suggestion_clear` | 设置页建议历史管理 |
| `memory_update` 扩展 | 增加 `sensitive` 字段（domain Memory + 迁移列） |

前端 `client.ts` 对应封装；`ipc.aiConfigGet()` 等。

## 6. UI

### 设置页「智能辅助」区块（SettingsPage.tsx 新 section，模式同「软件更新」区块）

- 模式三选一（关闭/本地 Ollama/自定义远程）→ 关闭时其余控件收起并显示数据去向说明文案。
- 「测试连接」按钮 → ProbeReport 展示（可达/模型/延迟；失败给 Ollama 安装指引链接，文案遵循 empty-states-and-permissions.md 语气）。
- key 输入（Custom 模式）：写入后显示「已设置」，提供清除。
- 五个 feature 开关（本切片默认关，标注「随后续版本逐步开放」）。
- 建议历史列表（feature/状态/时间）+「清空建议历史」两步确认（ConfirmButton 模式）。
- 空状态：「AI 功能尚未开启」+ 一句说明（不 guilt-trip、不推销）。

### MemoryPage「标记敏感」复选（编辑区 quickInsert 旁）

- 标签提示：「标记后内容不会发送给任何 AI 服务」。

## 7. 评估样本（tests/fixtures/ai-eval/）

- `extract_dates.json` / `extract_tasks.json` / `search_semantics.json`（中文样本各 ≥10 条，含敏感样本）。
- Runner（`#[test]` 离线）：sanitize 过滤命中、schema 校验拒绝坏输出、OffProvider 全链路 None、关闭 feature 零 provider 调用、key 不出现在 settings 导出与日志断言。
- `#[ignore]` 在线档：需本地 Ollama 才跑（`cargo test -- --ignored`），供切片 2+ 前手动回归。

## 8. 兼容与回滚

- 迁移 0019 纯增量 + `ALTER ADD COLUMN DEFAULT`，老库升级零风险；db/mod.rs `schema_version` 断言 18→19。
- `AppSettings.ai` serde default：旧导出 JSON 恢复后 AI 保持全关（符合"默认关闭"）。
- 回滚点：迁移/域层/provider/service/UI/docs 各自独立 commit，可单独 revert；不动任何既有表结构（除 memories 增列）。

## 9. 明确不做（重申）

无用户可见 AI 建议功能；无向量索引；无内嵌推理；无流式/对话；无远程遥测。

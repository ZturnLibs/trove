# Design: v2.0 语义检索（切片 8）

## 1. 模块边界

```
migrations/0021_semantic_index.sql          表（派生数据，可清空）
infrastructure/ai/mod.rs                    embed() 扩展（Ollama /api/embed；Custom /embeddings）
application/semantic_index.rs               SemanticIndexService（rebuild/search/status/clear）
domain/ai_suggestion.rs                     SemanticHit / AIFeatureToggles.semanticSearch / AIConfig.semanticExclusions
commands/mod.rs                             semantic_status/rebuild/clear；search_query 扩展响应
QuickWindow.tsx                             语义匹配分区
AIAssistSection.tsx                         开关 + 索引管理 + 排除范围
```

## 2. Embedding 协议

- **Ollama**：`POST {url}/api/embed` `{model, input: [texts]}` → `{embeddings: [[f32]]}`（新 API）。
- **Custom**：`POST {endpoint}/embeddings` `{model, input: [texts]}` → `{data: [{embedding}]}`
- 批 32、超时 60s（重建耗时容忍）；失败返 None（rebuild 中止报错，search 降级空）。
- 模型默认：Ollama `nomic-embed-text`；Custom 由用户填（`AIConfig.embeddingModel`，空则语义检索不可用并提示）。

## 3. 向量与相似度

- f32 序列化 little-endian BLOB；dims 记录行内。
- 查询：全表载入（`SELECT entity_type, entity_id, embedding`）内存余弦；Top-10 阈值 0.35。
  数据规模：本产品个人库 <5 万行 × 768 维 × 4B ≈ 150MB 上限不可接受 → **决策：限制索引行数上限 20000**（超出按 updated_at 截断，状态行提示"索引覆盖最近 2 万条"）。
- 查询向量与库内向量必须同模型：`search` 读表内 `model`，与当前配置不一致时返回 `model_mismatch` 状态（前端提示重建）。

## 4. 排除范围

`AIConfig.semantic_exclusions: { listIds: [], tagIds: [], clipboardTypes: [] }`（serde default 空）。
- rebuild 时 SQL 侧过滤：任务 `list_id NOT IN`；记忆 `id NOT IN (SELECT memory_id FROM memory_tags WHERE tag_id IN …)`；剪贴板 `kind NOT IN`。
- 排除的是"进入索引"，不影响关键词搜索（关键词是全量语义之外的既有行为）。

## 5. search_query 集成

```rust
// commands::search_query 尾部：
let semantic = if settings.ai.features.semantic_search {
    state.semantic.search(&query, 5).ok().unwrap_or_default()  // 失败静默降级
} else { Vec::new() };
```
`SearchResults` 增加 `semantic: Vec<SemanticHit>`（serde default 兼容旧前端缓存）。`SemanticHit` 复用 `SearchHit` 字段 + `score: f32` + `matched_type: "semantic"`。

## 6. 重建时机

- 手动 [重建索引]；
- 开启开关时若索引空 → 自动触发一次（后台，UI 显示进度行数）；
- 无监听自动重建（内容变更靠用户偶尔 rebuild——可接受的 v1 语义）。

## 7. 风险

| 风险 | 对策 |
| --- | --- |
| rebuild 慢（万条 × 网络 embed） | 批 32、进度可中断（行级写入，重跑续）；上限 2 万行 |
| 模型输出维度不一致 | 写入前 dims 校验；mismatch 状态引导重建 |
| 内存峰值 | 查询时按行流式计算余弦，不整表驻留 |

## 8. 回滚

新表派生可删；provider/服务/命令/UI/docs 独立 commit；关闭开关即回到 v1.x 行为。

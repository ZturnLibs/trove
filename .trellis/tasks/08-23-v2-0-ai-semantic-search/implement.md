# Implement: v2.0 语义检索（切片 8）

## 1. 数据与领域层

- [x] `migrations/0021_semantic_index.sql` + 断言 20→21；`AIFeatureToggles.semanticSearch` + `AIConfig.{embeddingModel, semanticExclusions}`
- 验证：`cargo test --lib infrastructure::db`

## 2. Provider embed

- [x] `AIProvider::embed(&self, texts) -> Option<Vec<Vec<f32>>>`（Ollama `/api/embed`、Custom `/embeddings`，批 32/60s/失败 None）
- [x] 单测：Off 返 None；批量切分正确性（用 fake provider 在服务层测）
- 验证：`cargo test --lib infrastructure::ai`

## 3. SemanticIndexService

- [x] `rebuild`（排除范围 SQL 过滤 + 上限 2 万 + 全量替换 + 幂等）
- [x] `search`（余弦 Top-K 阈值 0.35 + 模型一致性检查）、`status`、`clear`
- [x] 单测：rebuild 幂等/排除生效/clear 清空/模型 mismatch 降级
- 验证：`cargo test --lib application::semantic_index`

## 4. 搜索集成 + 前端

- [x] `SearchResults.semantic`（serde default）；`search_query` 条件计算
- [x] QuickWindow「语义匹配」分区（得分+徽标+跳转，关/空降级）
- [x] AIAssistSection：开关 + 索引状态 + 重建/清空 + 排除范围三选
- [x] client.ts 类型/封装（semanticStatus/rebuild/clear）
- 验证：`pnpm test:unit` + `pnpm build` + `tsc`

## 5. 评估与文档

- [x] ai-eval 语义样本在线档：≥7/10 命中（`#[ignore]`）
- [x] ai-assist.md「语义检索」+ privacy 派生索引可删小节；PRD/implement 勾选
- 验证：`cargo test`

## Review gates

1. 步骤 3 后：排除范围与上限代码审查
2. 步骤 4 后：语义分区文案（「语义匹配」不夸大准确性）
3. 完成后：父任务全局合同复核 → finish + archive（Wave C 收官 + v2.0 全切片完成）

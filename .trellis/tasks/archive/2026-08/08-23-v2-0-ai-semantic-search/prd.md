# v2.0 语义检索（切片 8）

> 父任务：`08-19-v2-0-ai-assist-roadmap`（Wave C 收官项）。
> §9.3 对应：语义检索——「对任务、记忆、OCR 文本建立**可重建**语义索引；搜索结果同时展示**关键词命中与语义匹配类型**；每项结果可跳转原始条目；用户可以排除指定清单、标签或内容类型」；§9.4：「关闭 AI 后可删除本地语义索引」。

## Goal

QuickWindow 搜索结果在关键词命中下方新增「语义匹配」分区：本地可重建向量索引 + Ollama embedding，用户开启才建索引；双列展示、来源可跳、索引可清空可排除范围。

## Requirements

1. **可重建索引**（新表 `0021_semantic_index`）：
   ```sql
   CREATE TABLE semantic_index (
     entity_type TEXT NOT NULL, entity_id TEXT NOT NULL,
     embedding BLOB NOT NULL,            -- f32 数组 little-endian
     model TEXT NOT NULL, dims INTEGER NOT NULL,
     indexed_at TEXT NOT NULL,
     PRIMARY KEY (entity_type, entity_id)
   );
   ```
   **派生数据**：不进 JSON 导出白名单、不进备份语义（随库走但可随时清空重建）、`rebuild` 幂等。
2. Provider 扩展（infrastructure/ai）：`embed(texts: &[&str]) -> Option<Vec<Vec<f32>>>`（Ollama `/api/embed`，批量 ≤32；Custom 模式 OpenAI 兼容 `/embeddings`）。Off 模式恒 None。
3. 服务（新 `SemanticIndexService`）：
   - `index_status()`：行数/模型/最后构建时间/覆盖比例（对 search_documents 计数）；
   - `rebuild()`：读 search_documents（Task/Memory/Clipboard，**应用排除范围**：排除清单的任务、排除标签的记忆、排除类型的剪贴板不进索引）→ 批量 embed → 全量替换写入；中断可重跑（先删后插按批次）；
   - `search(query, limit)`：embed 查询 → 余弦相似 Top-K（K≤10，阈值 0.35 过滤弱匹配）；
   - `clear()`：DELETE 全表（§9.4 关闭即删）。
4. 搜索流集成（`search_query` 扩展）：响应增加 `semantic: Vec<SemanticHit {entityType, entityId, title, score, matchedType}>`；**仅当** `ai.features.semanticSearch && 索引就绪` 时计算（关了零开销）；关键词结果保持不变。
5. UI（QuickWindow 结果区）：
   - 关键词命中之后渲染「语义匹配」小节（得分 + 标题 + 类型徽标），空结果时该节也显示（语义能补位「换个说法也能找到」，最多 5 条）；
   - 结果点击跳转复用既有 hit 导航。
6. 设置页：AI 区块新增「语义检索」开关（默认关）+ 索引状态行（N 条 · 模型 · 时间）+ [重建索引][清空索引] 按钮；排除范围展示（清单/标签/类型三个多选，存 `AIConfig.semanticExclusions`）。

## Acceptance Criteria

- [x] 关闭开关：`search_query` 响应无 semantic 字段开销，行为与 v1.x 完全一致（单测）
- [x] 索引可全量重建且幂等（两次 rebuild 行数一致，单测）；清空后表为空（单测）
- [x] 排除范围生效：排除清单的任务不出现在语义索引（单测）
- [x] 双列展示：关键词命中与语义匹配分区渲染，语义项带 matchedType 徽标，点击可跳（手测脚本）
- [x] ai-eval 语义样本（10 对非关键词查询）在 Ollama 在线档命中 ≥7 对（`#[ignore]` 回归）
- [x] embedding 模型名写入索引行；换模型后 rebuild 全量替换（旧模型行不留）
- [x] Off/失败降级：无 semantic 节，不阻塞关键词搜索
- [x] cargo test / pnpm test / build 全绿；docs/ai-assist.md + privacy（派生索引可删）更新

## 明确不做（本切片）

- 增量实时索引（rebuild 手动/开启时触发；内容变更不自动重嵌）
- 提醒/OCR 之外的类型（任务/记忆/剪贴板三种，OCR 文本已在剪贴板 body）
- 量化/ANN 库（万级以下暴力余弦即可，f32 全扫描 <10ms）
- 移动端/远程向量库

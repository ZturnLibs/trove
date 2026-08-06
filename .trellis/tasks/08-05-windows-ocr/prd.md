# Windows OCR 降级文案（windows-ocr）

## Goal

明确「图片 OCR 仅 macOS 可用」的降级策略：把 OCR 能力纳入能力清单并在 UI 中如实展示；在非 macOS 平台的剪切板页给出提示，避免用户误以为图片可按文字搜索。

## 背景（勘察确认）

- `platform/ocr.rs:27-34`：非 macOS 平台 `recognize_png` 直接返回空文本（无 OCR）。
- `PlatformCapabilities`（platform/mod.rs:9-16）**无 ocr 字段**；设置页能力区按 `Object.entries(health.capabilities)` 泛化渲染（SettingsPage.tsx:1017），label 映射在 :225-228。
- 剪切板搜索：图片条目用 `ocrText`（client.ts:388）/ search 用 ocr_text 组 body；无 OCR 时只有 `[图片]` 占位，按文字搜索无效。
- ClipboardPage 的 healthQuery 刚在 direct-paste 任务中被移除，需按需重建。

## Requirements

- R1 后端 `PlatformCapabilities` 增加 `ocr: CapabilityStatus`：`available = cfg!(target_os = "macos")`；notes 文案「macOS 使用本机 Vision 识别图片文字；其他平台暂不支持，图片无法按文字搜索。」
- R2 client.ts `PlatformCapabilities` 增加 `ocr` 字段。
- R3 设置页能力 label 映射增加 `ocr: "图片识别（OCR）"`。
- R4 剪切板页在 OCR 不可用（非 macOS）时，搜索区附近显示一行小字提示「当前平台不支持图片文字识别（OCR），图片无法按文字搜索。」（需重建 healthQuery 查询 `["app","health"]`）。
- R5 不改变 OCR 执行行为（非 macOS 仍返回空文本）；不引入 Windows OCR 引擎。

## Acceptance Criteria

- [ ] AC1 能力清单含 `ocr`，macOS available=true、其它平台 false；notes 文案诚实。
- [ ] AC2 设置页能力列表出现「图片识别（OCR）」条目。
- [ ] AC3 非 macOS 剪切板页显示 OCR 不可用提示；macOS 不显示。
- [ ] AC4 `cargo check`、`pnpm typecheck`、`pnpm build` 通过。

## Notes

- 轻量任务：PRD + 简要 design。改动为 Rust（platform/mod.rs）+ 前端（client.ts、SettingsPage.tsx、ClipboardPage.tsx）。
- 决策已确认：降级文案（不实现 Windows/Linux OCR）。

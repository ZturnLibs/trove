# 技术设计：Windows OCR 降级文案

## 1. 后端：能力清单加 ocr

`platform/mod.rs` `PlatformCapabilities` 增加字段并填充：

```rust
pub ocr: CapabilityStatus,
// ...
ocr: CapabilityStatus {
    available: cfg!(target_os = "macos"),
    notes: "macOS 使用本机 Vision 识别图片文字；其他平台暂不支持，图片无法按文字搜索。".into(),
},
```

## 2. 前端类型

client.ts `PlatformCapabilities` 增加 `ocr: CapabilityStatus;`（camelCase 序列化 `ocr`）。

## 3. 设置页

SettingsPage.tsx 能力 label 映射（:225-228 附近）增加 `ocr: "图片识别（OCR）"`（能力区已泛化渲染，自动出现条目与 notes）。

## 4. 剪切板页提示

- ClipboardPage 重建 `healthQuery = useQuery(["app","health"], () => ipc.appHealth())`（与设置页/主框架同 key，复用缓存）。
- 在搜索框/工具区附近（如搜索 Input 下或 actions 区）当 `healthQuery.data?.capabilities.ocr.available === false` 时渲染一行小字：
  `<p className="text-[11px] text-muted">当前平台不支持图片文字识别（OCR），图片无法按文字搜索。</p>`
- macOS 不显示（available=true）。

## 5. 边界

- 不改 ocr.rs 行为（非 macOS 仍返回空）；不引入新引擎。
- healthQuery 仅用于该提示，不恢复 direct-paste 横幅逻辑。

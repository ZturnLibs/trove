# Windows OCR 本地识别（v1.2.1 · 4.3）

## 目标

Windows 剪切板图片接入 `Windows.Media.Ocr`，恢复图片文字搜索；识别本地完成，失败不影响图片保存。

## 验收标准

1. Windows 构建通过，`recognize_png` 调用 WinRT OCR
2. `PlatformCapabilities.ocr.available` 在 Windows 为 true
3. Linux 仍为 false，剪切板页保留降级提示
4. macOS 行为不变

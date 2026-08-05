# 技术设计：Logo 设计与全量更新

## 1. Logo 视觉规格

**意象**：宝藏箱 + 字母 T。设计上让箱体轮廓直接构成字母 T：
- 横杠 = 箱盖（圆弧顶/水平条）
- 竖笔 = 箱体前壁，中下方一个钥匙孔圆点强调"宝藏"

**色彩**：品牌蓝单色。基础 `#2563eb`；应用内组件用 `currentColor` 继承主题色（亮 `#2563eb` / 暗 `#5b8def`），保证明暗主题可辨。

**几何约束**：
- 源图为 1024×1024 方形、透明背景、主体占画面约 60–70%。
- 形状由矩形/圆角矩形 + 圆弧构成，不使用照片级细节，保证 16px 托盘图标仍可辨识（粗线宽、大对比）。
- 单色填充 + 少量负空间镂空（钥匙孔、箱缝）。

## 2. 资源生成与分发链路

```
design/logo.svg（1024×1024 源图，单色）
        │
        ├─ pnpm tauri icon design/logo.svg  ──►  src-tauri/icons/* 全平台图标
        │       （32/128/128@2x png、icon.icns、icon.ico、icon.png、
        │         Square*Logo.png、StoreLogo.png；覆盖 bundle.icon 引用）
        │
        ├─ public/logo.svg  ──►  index.html <link rel="icon">（favicon）
        │
        └─ src/components/BrandLogo.tsx（内联 SVG，currentColor）
                └─ MainShell 侧边栏、OnboardingOverlay
```

- `tauri icon` 输入要求：方形透明 PNG 或 SVG。用 SVG 保证无损；若 CLI 对 SVG 支持不佳，退路是先用 `sips`/`sharp` 渲染 1024px PNG 再喂给 CLI。
- 托盘图标 `lib.rs:87` 使用 `app.default_window_icon()`，由 bundle 图标（`icons/32x32.png` 等）提供，再生成后自动生效，无需改 Rust。
- 窗口图标（macOS Dock / Windows 任务栏）由 `icon.icns` / `icon.ico` 提供，同样随生成更新。

## 3. 应用内品牌组件

新增 `src/components/BrandLogo.tsx`：

```tsx
// 内联 SVG 宝藏箱+T，fill="currentColor"，随主题/前景色自适应
export function BrandLogo({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 32 32" className={className} fill="none" aria-hidden="true">
      {/* 箱盖横条 = T 横杠；箱体竖壁 = T 竖笔；钥匙孔镂空 */}
    </svg>
  );
}
```

用法：
- `MainShell.tsx` 侧边栏品牌区：`<BrandLogo className="h-5 w-5 text-accent" /> <span>Trove</span>`（`text-accent` 继承 `--color-accent`，亮暗自动切换）。
- `OnboardingOverlay.tsx` 标题上方：`<BrandLogo className="h-10 w-10 text-accent" />`。

## 4. 资源清理

- `index.html`：`<link rel="icon" href="/logo.svg">`（替换 `/vite.svg`）。
- 删除 `public/vite.svg`、`public/tauri.svg`（无其他引用，用 grep 确认后删）。
- `src-tauri/icons/` 直接由 `tauri icon` 覆盖，旧默认图标一并替换。

## 5. 兼容性/回归注意

- `tauri.conf.json` `bundle.icon` 引用的文件名不变（`32x32.png`、`128x128.png`、`128x128@2x.png`、`icon.icns`、`icon.ico`），无需改配置。
- 侧边栏布局：品牌区固定 `h-11`，Logo 20px + 文本不换行，宽度 `w-[200px]` 不变。
- Onboarding 弹层只加图片，不动既有列表与按钮。
- 图标 PNG 需要透明背景（托盘/窗口透明角）；不引入照片底色。

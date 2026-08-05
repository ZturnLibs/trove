# 实施计划：Logo 设计与全量更新

## 执行清单（有序）

- [ ] 1. **设计源图**：创建 `design/logo.svg`（1024×1024，宝藏箱+T，品牌蓝 `#2563eb`，透明背景）。
  - 1a. 先做 32×32 viewBox 的简洁几何标记；扩展为 1024 画布。
  - 1b. 自检：16px 缩小可辨、箱盖= T 横杠、钥匙孔镂空清晰。
- [ ] 2. **再生成平台图标**：`pnpm tauri icon design/logo.svg`（输出到 `src-tauri/icons/`）。
  - 2a. 若 SVG 输入不被接受，渲染 1024 PNG（`sips -s format png`）后重试。
  - 2b. 核对生成的 `32x32.png`、`128x128.png`、`128x128@2x.png`、`icon.icns`、`icon.ico`、`icon.png` 与 `Square*Logo.png`、`StoreLogo.png` 存在且非默认图标。
- [ ] 3. **favicon 与默认资源**：
  - 3a. 复制 `public/logo.svg`（或精简版）。
  - 3b. `index.html`：`<link rel="icon" type="image/svg+xml" href="/logo.svg">`。
  - 3c. grep 确认 `vite.svg`/`tauri.svg` 无引用后删除。
- [ ] 4. **应用内品牌组件**：新建 `src/components/BrandLogo.tsx`（内联 SVG，`currentColor`）。
- [ ] 5. **侧边栏**：`MainShell.tsx` 品牌区插入 `<BrandLogo className="h-5 w-5 text-accent" />` + "Trove"。
- [ ] 6. **Onboarding**：`OnboardingOverlay.tsx` 标题上方插入 `<BrandLogo className="h-10 w-10 text-accent" />`。
- [ ] 7. **验证门禁（review gate）**：
  - `pnpm typecheck`
  - `pnpm build`
  - `pnpm test:unit`
  - 运行 `pnpm tauri:dev` 目视检查侧边栏、托盘、Dock 图标（开发者自测）。
- [ ] 8. **回归自检**：grep 确认无残留 `/vite.svg`、`/tauri.svg`、`tauri.svg` 引用；`bundle.icon` 文件名未变。

## 验证命令

```bash
pnpm tauri icon design/logo.svg
pnpm typecheck
pnpm build
pnpm test:unit
```

## 回滚点

- 步骤 2 之前：未改任何文件，直接丢弃新增源图即可。
- 步骤 3/4/5/6 之前：图标集已换新但前端未引用；git 恢复前端文件即可。
- 全量回滚：`git checkout -- src-tauri/icons public index.html src` + 删除新增文件。

## 风险与对策

- `tauri icon` 对 SVG 兼容性 → 用 `sips`/Rust image 先转 1024 PNG（含 alpha）。
- 小尺寸可辨性不足 → 加粗轮廓、放大主体占比；16px 目检。
- 暗色主题下蓝色偏深 → 组件用 `currentColor` 继承主题 accent（暗色 `#5b8def`）。

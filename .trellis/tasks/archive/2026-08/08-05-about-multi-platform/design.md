# 技术设计：About 跨平台

## 1. Rust：帮助菜单加「关于」

menu_bar.rs `help_menu` 增加：

```rust
let help_about = MenuItem::with_id(app, "menu.help.about", "关于 Trove", true, None::<&str>)?;
let help_menu = Submenu::with_items(app, "帮助", true, &[&help_shortcuts, &help_privacy, &help_about])?;
```

`handle_menu_event` 增加分支：

```rust
"menu.help.about" => {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("menu://about", ());
    }
},
```

## 2. 前端：AboutDialog

`src/components/AboutDialog.tsx`：

```tsx
export function AboutDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const health = useQuery({ queryKey: ["app","health"], queryFn: () => ipc.appHealth() });
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-6" onClick={onClose}>
      <div className="w-72 rounded-[var(--radius-panel)] border border-border bg-surface p-6 text-center shadow-lg" onClick={(e) => e.stopPropagation()}>
        <BrandLogo className="mx-auto h-14 w-14 text-accent" />
        <h2 className="mt-3 text-[16px] font-semibold">Trove</h2>
        <p className="mt-1 text-[12px] text-muted">版本 {health.data?.appVersion ?? "…"}</p>
        <p className="mt-2 text-[12px] text-foreground">本地优先的个人工作台</p>
        <p className="mt-1 text-[11px] text-muted">© 2026 Trove</p>
        <Button size="sm" className="mt-4" onClick={onClose}>关闭</Button>
      </div>
    </div>
  );
}
```

- `useQuery` key `["app","health"]` 已在 MainShell/Settings 使用，可复用缓存。

## 3. MainShell 接线

```tsx
const [aboutOpen, setAboutOpen] = useState(false);
useEffect(() => {
  let un: (() => void) | undefined;
  void listen("menu://about", () => setAboutOpen(true)).then((f) => (un = f));
  return () => un?.();
}, []);
// 渲染 <AboutDialog open={aboutOpen} onClose={() => setAboutOpen(false)} />
```

- `listen` 已在 MainShell 中用于 `backup://failed`，沿用同一模式（`@tauri-apps/api/event`）。

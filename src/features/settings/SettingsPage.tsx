import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { PageScaffold } from "@/components/PageScaffold";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import { ipc, type AppSettings, type ThemePreference } from "@/ipc/client";

export function SettingsPage() {
  const queryClient = useQueryClient();
  const healthQuery = useQuery({
    queryKey: ["app", "health"],
    queryFn: () => ipc.appHealth(),
  });
  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: () => ipc.settingsGet(),
  });
  const notesQuery = useQuery({
    queryKey: ["smoke-notes"],
    queryFn: () => ipc.smokeNoteList(),
  });

  const [draft, setDraft] = useState("");
  const [excludedText, setExcludedText] = useState("");
  const [retentionText, setRetentionText] = useState("");
  const [maxItemsText, setMaxItemsText] = useState("");

  const settings = settingsQuery.data;
  const health = healthQuery.data;

  useEffect(() => {
    if (!settings) return;
    setExcludedText(settings.clipboardExcludedApps.join("\n"));
    setRetentionText(String(settings.clipboardRetentionDays));
    setMaxItemsText(String(settings.clipboardMaxItems));
  }, [settings]);

  const saveSettings = useMutation({
    mutationFn: (next: AppSettings) => ipc.settingsSave(next),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["settings"] });
      void queryClient.invalidateQueries({ queryKey: ["app", "health"] });
    },
  });

  const createNote = useMutation({
    mutationFn: (body: string) => ipc.smokeNoteCreate(body),
    onSuccess: () => {
      setDraft("");
      void queryClient.invalidateQueries({ queryKey: ["smoke-notes"] });
    },
  });

  const deleteNote = useMutation({
    mutationFn: (id: string) => ipc.smokeNoteDelete(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["smoke-notes"] });
    },
  });

  const updateTheme = (theme: ThemePreference) => {
    if (!settings) return;
    saveSettings.mutate({ ...settings, theme });
    document.documentElement.dataset.theme =
      theme === "system"
        ? window.matchMedia("(prefers-color-scheme: dark)").matches
          ? "dark"
          : "light"
        : theme;
  };

  const persistClipboardFields = () => {
    if (!settings) return;
    saveSettings.mutate({
      ...settings,
      clipboardRetentionDays: Number(retentionText) || 30,
      clipboardMaxItems: Number(maxItemsText) || 500,
      clipboardExcludedApps: excludedText
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean),
    });
  };

  return (
    <PageScaffold title="设置" description="应用健康、主题、剪切板与持久化">
      <div className="mx-auto flex max-w-3xl flex-col gap-6 p-4">
        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">应用状态</h2>
          {healthQuery.isLoading ? (
            <p className="mt-2 text-[12px] text-muted">检查中…</p>
          ) : healthQuery.isError ? (
            <p className="mt-2 text-[12px] text-danger">
              无法连接本地后端（浏览器预览模式也可能出现此提示）
            </p>
          ) : health ? (
            <dl className="mt-3 grid grid-cols-2 gap-x-4 gap-y-2 text-[12px]">
              <div>
                <dt className="text-muted">版本</dt>
                <dd>{health.appVersion}</dd>
              </div>
              <div>
                <dt className="text-muted">Schema</dt>
                <dd>v{health.database.schemaVersion}</dd>
              </div>
              <div>
                <dt className="text-muted">Journal</dt>
                <dd>{health.database.journalMode}</dd>
              </div>
              <div>
                <dt className="text-muted">FTS5</dt>
                <dd>{health.database.fts5Available ? "可用" : "不可用"}</dd>
              </div>
              <div className="col-span-2">
                <dt className="text-muted">数据库路径</dt>
                <dd className="break-all">{health.database.path}</dd>
              </div>
            </dl>
          ) : null}
        </section>

        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">主题</h2>
          <div className="mt-3 flex gap-2">
            {(["system", "light", "dark"] as const).map((theme) => (
              <Button
                key={theme}
                size="sm"
                variant={settings?.theme === theme ? "default" : "secondary"}
                onClick={() => updateTheme(theme)}
                disabled={!settings || saveSettings.isPending}
              >
                {theme === "system" ? "跟随系统" : theme === "light" ? "浅色" : "深色"}
              </Button>
            ))}
          </div>
        </section>

        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">剪切板</h2>
          <p className="mt-1 text-[12px] text-muted">
            文本历史仅保存在本机。无法仅凭内容准确识别所有密码或验证码；请用暂停与排除应用保护敏感复制。
          </p>
          {settings ? (
            <div className="mt-3 space-y-3 text-[12px]">
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={settings.clipboardCaptureEnabled}
                  onChange={(e) =>
                    saveSettings.mutate({
                      ...settings,
                      clipboardCaptureEnabled: e.target.checked,
                    })
                  }
                />
                启用剪切板采集
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-muted">保留天数（收藏不过期）</span>
                <Input
                  type="number"
                  min={1}
                  max={3650}
                  value={retentionText}
                  onChange={(e) => setRetentionText(e.target.value)}
                  onBlur={persistClipboardFields}
                />
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-muted">最大条数</span>
                <Input
                  type="number"
                  min={10}
                  max={20000}
                  value={maxItemsText}
                  onChange={(e) => setMaxItemsText(e.target.value)}
                  onBlur={persistClipboardFields}
                />
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-muted">排除应用（每行一个）</span>
                <textarea
                  className="min-h-24 w-full rounded-[var(--radius-control)] border border-border bg-surface p-2 text-[13px] outline-none focus:ring-2 focus:ring-accent/35"
                  value={excludedText}
                  onChange={(e) => setExcludedText(e.target.value)}
                  onBlur={persistClipboardFields}
                />
              </label>
              {health?.capabilities.directPaste ? (
                <p className="text-muted">
                  直接粘贴：
                  {health.capabilities.directPaste.available
                    ? "可用"
                    : "未授权，再次使用将写入系统剪切板"}{" "}
                  — {health.capabilities.directPaste.notes}
                </p>
              ) : null}
            </div>
          ) : (
            <p className="mt-2 text-[12px] text-muted">加载设置…</p>
          )}
        </section>

        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">持久化冒烟测试</h2>
          <p className="mt-1 text-[12px] text-muted">
            写入本地 SQLite，重启应用后仍应可见。用于验收阶段 0 数据层。
          </p>
          <form
            className="mt-3 flex gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              if (!draft.trim()) return;
              createNote.mutate(draft.trim());
            }}
          >
            <Input
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              placeholder="输入一条测试笔记…"
            />
            <Button type="submit" disabled={createNote.isPending}>
              保存
            </Button>
          </form>
          <ul className="mt-3 divide-y divide-border border-t border-border">
            {(notesQuery.data ?? []).map((note) => (
              <li
                key={note.id}
                className="flex items-center justify-between gap-3 py-2 text-[13px]"
              >
                <div className="min-w-0">
                  <p className="truncate">{note.body}</p>
                  <p className="text-[11px] text-muted">{note.updatedAt}</p>
                </div>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => deleteNote.mutate(note.id)}
                >
                  删除
                </Button>
              </li>
            ))}
            {notesQuery.data?.length === 0 ? (
              <li className="py-3 text-[12px] text-muted">暂无测试笔记</li>
            ) : null}
          </ul>
        </section>

        {health ? (
          <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
            <h2 className="text-[13px] font-semibold">平台能力</h2>
            <ul className="mt-3 space-y-2 text-[12px]">
              {Object.entries(health.capabilities).map(([key, value]) => (
                <li key={key} className="flex gap-3">
                  <span className="w-28 shrink-0 text-muted">{key}</span>
                  <span>
                    {value.available ? "可用" : "不可用"} — {value.notes}
                  </span>
                </li>
              ))}
            </ul>
          </section>
        ) : null}
      </div>
    </PageScaffold>
  );
}

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
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

  const saveSettings = useMutation({
    mutationFn: (settings: AppSettings) => ipc.settingsSave(settings),
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

  const settings = settingsQuery.data;
  const health = healthQuery.data;

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

  return (
    <PageScaffold title="设置" description="阶段 0：应用健康、主题与持久化冒烟测试">
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
            <h2 className="text-[13px] font-semibold">平台能力（阶段 0）</h2>
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

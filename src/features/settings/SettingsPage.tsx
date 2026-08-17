import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { PageScaffold } from "@/components/PageScaffold";
import { Button } from "@/design-system/primitives/Button";
import { RecurrencePicker } from "@/design-system/patterns/RecurrencePicker";
import { ConfirmButton } from "@/design-system/patterns/ConfirmButton";
import { Input } from "@/design-system/primitives/Input";
import { ShortcutRow } from "@/features/settings/ShortcutRow";
import { AutomationRulesSection } from "@/features/settings/AutomationRulesSection";
import {
  ipc,
  type AppSettings,
  type ItemTemplate,
  type RecurrenceRule,
  type TaskPriority,
  type TemplateKind,
  type TemplatePreview,
  type ThemePreference,
} from "@/ipc/client";
import { recurrenceLabel } from "@/lib/recurrence";
import { QUICK_CAPTURE_SYNTAX } from "@/lib/nl-capture";
import {
  isAppUpdaterSupported,
  useAppUpdater,
} from "@/stores/app-updater";

function formatBytes(n: number) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

const KIND_LABEL: Record<TemplateKind, string> = {
  task: "任务",
  reminder: "提醒",
  memory: "记忆",
};

const PRIORITY_LABEL: Record<TaskPriority, string> = {
  none: "无",
  low: "低",
  medium: "中",
  high: "高",
};

export function SettingsPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const fileRef = useRef<HTMLInputElement>(null);
  const healthQuery = useQuery({
    queryKey: ["app", "health"],
    queryFn: () => ipc.appHealth(),
  });
  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: () => ipc.settingsGet(),
  });
  const backupsQuery = useQuery({
    queryKey: ["backups"],
    queryFn: () => ipc.backupList(),
  });
  const templatesQuery = useQuery({
    queryKey: ["templates"],
    queryFn: () => ipc.templateList(),
  });
  const updaterPhase = useAppUpdater((s) => s.phase);
  const updaterVersion = useAppUpdater((s) => s.availableVersion);
  const updaterError = useAppUpdater((s) => s.error);
  const updaterProgress = useAppUpdater((s) => s.progress);
  const updaterLastCheckedAt = useAppUpdater((s) => s.lastCheckedAt);
  const checkForUpdates = useAppUpdater((s) => s.checkForUpdates);
  const installUpdate = useAppUpdater((s) => s.installUpdate);

  const [excludedText, setExcludedText] = useState("");
  const [retentionText, setRetentionText] = useState("");
  const [maxItemsText, setMaxItemsText] = useState("");
  const [backupKeepText, setBackupKeepText] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [pendingImportFile, setPendingImportFile] = useState<File | null>(null);

  // 新建模板表单
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [templateName, setTemplateName] = useState("");
  const [templateKind, setTemplateKind] = useState<TemplateKind>("task");
  const [taskTitle, setTaskTitle] = useState("");
  const [relativeDueDays, setRelativeDueDays] = useState("0");
  const [priority, setPriority] = useState<TaskPriority>("none");
  const [reminderTitle, setReminderTitle] = useState("");
  const [relativeFireHours, setRelativeFireHours] = useState("0");
  const [reminderRecurrence, setReminderRecurrence] =
    useState<RecurrenceRule | null>(null);
  const [memoryTitle, setMemoryTitle] = useState("");
  const [memoryBody, setMemoryBody] = useState("");

  // 应用前预览
  const [previewing, setPreviewing] = useState<ItemTemplate | null>(null);
  const [previewData, setPreviewData] = useState<TemplatePreview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);

  const settings = settingsQuery.data;
  const health = healthQuery.data;

  useEffect(() => {
    if (!settings) return;
    setExcludedText(settings.clipboardExcludedApps.join("\n"));
    setRetentionText(String(settings.clipboardRetentionDays));
    setMaxItemsText(String(settings.clipboardMaxItems));
    setBackupKeepText(String(settings.backupRetentionCount));
  }, [settings]);

  const saveSettings = useMutation({
    mutationFn: (next: AppSettings) => ipc.settingsSave(next),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["settings"] });
      void queryClient.invalidateQueries({ queryKey: ["app", "health"] });
      setMessage("设置已保存");
    },
    onError: (err) => {
      setMessage(err instanceof Error ? err.message : "保存失败");
    },
  });

  const resetShortcuts = useMutation({
    mutationFn: () => ipc.settingsResetShortcuts(),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["settings"] });
      setMessage("快捷键已恢复默认并重新注册");
    },
    onError: (err) => {
      setMessage(err instanceof Error ? err.message : "恢复快捷键失败");
    },
  });

  const saveShortcut = (key: keyof AppSettings["shortcuts"], value: string) => {
    if (!settings) return;
    const next = {
      ...settings,
      shortcuts: { ...settings.shortcuts, [key]: value },
    };
    saveSettings.mutate(next, {
      onSuccess: () => setMessage("快捷键已保存并重新注册"),
      onError: (err) => {
        setMessage(
          err instanceof Error
            ? err.message
            : "快捷键注册失败，请更换组合或恢复默认",
        );
      },
    });
  };

  const createBackup = useMutation({
    mutationFn: () => ipc.backupCreate(),
    onSuccess: (info) => {
      void queryClient.invalidateQueries({ queryKey: ["backups"] });
      void queryClient.invalidateQueries({ queryKey: ["app", "health"] });
      setMessage(`备份已创建：${info.fileName}`);
    },
    onError: (err) => {
      setMessage(err instanceof Error ? err.message : "备份失败");
    },
  });

  const restoreBackup = useMutation({
    mutationFn: (fileName: string) => ipc.backupRestore(fileName),
    onSuccess: () => {
      void queryClient.invalidateQueries();
      setMessage("已从备份恢复，数据已刷新");
    },
    onError: (err) => {
      setMessage(err instanceof Error ? err.message : "恢复失败");
    },
  });

  const exportData = useMutation({
    mutationFn: () => ipc.dataExport(),
    onSuccess: (json) => {
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `workbench-export-${new Date().toISOString().slice(0, 10)}.json`;
      a.click();
      URL.revokeObjectURL(url);
      setMessage("已导出 JSON 文件");
    },
    onError: (err) => {
      setMessage(err instanceof Error ? err.message : "导出失败");
    },
  });

  const importData = useMutation({
    mutationFn: (json: string) => ipc.dataImport(json),
    onSuccess: (result) => {
      void queryClient.invalidateQueries();
      setMessage(`导入完成：${result.tables} 张表，${result.rows} 行（已先自动备份）`);
    },
    onError: (err) => {
      setMessage(err instanceof Error ? err.message : "导入失败");
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
      backupRetentionCount: Number(backupKeepText) || 10,
    });
  };

  const capLabel: Record<string, string> = {
    notifications: "通知",
    globalShortcuts: "全局快捷键",
    clipboardRead: "剪切板读取",
    directPaste: "直接粘贴",
    autostart: "开机启动",
    tray: "托盘",
    ocr: "图片识别（OCR）",
  };

  const handleKindChange = (kind: TemplateKind) => {
    setTemplateKind(kind);
    setTaskTitle("");
    setReminderTitle("");
    setMemoryTitle("");
    setMemoryBody("");
    // 清空各类型专属字段，避免切换后残留值串入其他类型
    setRelativeDueDays("0");
    setPriority("none");
    setRelativeFireHours("0");
    setReminderRecurrence(null);
  };

  const resetCreateForm = () => {
    setTemplateName("");
    setTemplateKind("task");
    setTaskTitle("");
    setRelativeDueDays("0");
    setPriority("none");
    setReminderTitle("");
    setRelativeFireHours("0");
    setReminderRecurrence(null);
    setMemoryTitle("");
    setMemoryBody("");
  };

  const buildTemplatePayload = (): Record<string, unknown> => {
    if (templateKind === "task") {
      return {
        title: taskTitle.trim(),
        relativeDueDays: Number(relativeDueDays) || 0,
        priority,
      };
    }
    if (templateKind === "reminder") {
      return {
        title: reminderTitle.trim(),
        relativeFireHours: Number(relativeFireHours) || 0,
        ...(reminderRecurrence ? { recurrence: reminderRecurrence } : {}),
      };
    }
    return {
      title: memoryTitle.trim(),
      body: memoryBody,
    };
  };

  const createTemplate = useMutation({
    mutationFn: (input: {
      kind: TemplateKind;
      name: string;
      payload: Record<string, unknown>;
    }) => ipc.templateCreate(input),
    onSuccess: (tpl) => {
      void queryClient.invalidateQueries({ queryKey: ["templates"] });
      setMessage(`已创建模板「${tpl.name}」`);
      setShowCreateForm(false);
      resetCreateForm();
    },
    onError: (err) => {
      setMessage(err instanceof Error ? err.message : "创建模板失败");
    },
  });

  const openPreview = (tpl: ItemTemplate) => {
    setPreviewing(tpl);
    setPreviewData(null);
    setPreviewError(null);
    void ipc
      .templatePreview(tpl.id)
      .then((data) => setPreviewData(data))
      .catch((err) =>
        setPreviewError(err instanceof Error ? err.message : "解析模板失败"),
      );
  };

  const closePreview = () => {
    setPreviewing(null);
    setPreviewData(null);
    setPreviewError(null);
  };

  const applyTemplate = useMutation({
    mutationFn: (id: string) => ipc.templateApply(id),
    onSuccess: () => {
      const name = previewing?.name ?? "";
      void queryClient.invalidateQueries({ queryKey: ["templates"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      void queryClient.invalidateQueries({ queryKey: ["reminders"] });
      void queryClient.invalidateQueries({ queryKey: ["memories"] });
      setMessage(`已应用模板「${name}」`);
      closePreview();
    },
    onError: (err) => {
      setMessage(err instanceof Error ? err.message : "应用模板失败");
    },
  });

  const currentTitle =
    templateKind === "task"
      ? taskTitle
      : templateKind === "reminder"
        ? reminderTitle
        : memoryTitle;
  const canCreate =
    Boolean(templateName.trim()) &&
    Boolean(currentTitle.trim()) &&
    !createTemplate.isPending;

  return (
    <PageScaffold title="设置" description="备份、权限、快捷键与隐私说明">
      <div className="mx-auto flex max-w-3xl flex-col gap-6 p-4">
        {message ? (
          <div className="rounded-[var(--radius-panel)] border border-border bg-surface-raised px-3 py-2 text-[12px]">
            {message}
          </div>
        ) : null}

        {health?.backup.lastError ? (
          <div className="rounded-[var(--radius-panel)] border border-danger/40 bg-danger/5 px-3 py-2 text-[12px] text-danger">
            备份异常：{health.backup.lastError}
          </div>
        ) : null}

        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">工作节奏</h2>
          <p className="mt-1 text-[12px] text-muted">
            每周回顾汇总收件箱、逾期、等待等待整理信号，不含效率评分。
          </p>
          {settings ? (
            <label className="mt-3 flex items-center gap-2 text-[12px]">
              <input
                type="checkbox"
                checked={settings.todaySmartSortEnabled}
                onChange={(e) =>
                  saveSettings.mutate({
                    ...settings,
                    todaySmartSortEnabled: e.target.checked,
                  })
                }
              />
              今日页智能排序建议（本地算法，可采纳或忽略）
            </label>
          ) : null}
          <div className="mt-3 flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="secondary"
              onClick={() => navigate("/weekly-review")}
            >
              打开每周回顾
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => navigate("/health")}
            >
              打开健康仪表盘
            </Button>
          </div>
        </section>

        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">应用状态</h2>
          {healthQuery.isLoading ? (
            <p className="mt-2 text-[12px] text-muted">检查中…</p>
          ) : healthQuery.isError ? (
            <p className="mt-2 text-[12px] text-danger">无法连接本地后端</p>
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
                <dt className="text-muted">备份数量</dt>
                <dd>{health.backup.count}</dd>
              </div>
              <div>
                <dt className="text-muted">最近备份</dt>
                <dd>{health.backup.latest?.createdAt ?? "无"}</dd>
              </div>
              <div className="col-span-2">
                <dt className="text-muted">数据库路径</dt>
                <dd className="break-all">{health.database.path}</dd>
              </div>
            </dl>
          ) : null}
        </section>

        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">通用</h2>
          <div className="mt-3 space-y-3 text-[12px]">
            <div>
              <p className="mb-2 text-muted">主题</p>
              <div className="flex gap-2">
                {(["system", "light", "dark"] as const).map((theme) => (
                  <Button
                    key={theme}
                    size="sm"
                    variant={settings?.theme === theme ? "default" : "secondary"}
                    onClick={() => updateTheme(theme)}
                    disabled={!settings || saveSettings.isPending}
                  >
                    {theme === "system"
                      ? "跟随系统"
                      : theme === "light"
                        ? "浅色"
                        : "深色"}
                  </Button>
                ))}
              </div>
            </div>
            {settings ? (
              <label className="flex items-start gap-2">
                <input
                  type="checkbox"
                  className="mt-0.5"
                  checked={settings.launchAtLogin}
                  onChange={(e) =>
                    saveSettings.mutate({
                      ...settings,
                      launchAtLogin: e.target.checked,
                    })
                  }
                />
                <span>
                  <span className="block font-medium">开机启动</span>
                  <span className="text-muted">
                    登录系统后后台运行，以便提醒与剪切板采集继续工作。关闭主窗口不会退出应用。
                  </span>
                </span>
              </label>
            ) : null}
          </div>
        </section>

        {isAppUpdaterSupported() ? (
          <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
            <h2 className="text-[13px] font-semibold">软件更新</h2>
            <p className="mt-1 text-[12px] text-muted">
              从 GitHub Release 检查签名更新包。更新仅替换应用本身，不会修改本地数据库。
            </p>
            <div className="mt-3 space-y-3 text-[12px]">
              {settings ? (
                <label className="flex items-start gap-2">
                  <input
                    type="checkbox"
                    className="mt-0.5"
                    checked={settings.autoCheckUpdates}
                    onChange={(e) =>
                      saveSettings.mutate({
                        ...settings,
                        autoCheckUpdates: e.target.checked,
                      })
                    }
                  />
                  <span>
                    <span className="block font-medium">自动检查更新</span>
                    <span className="text-muted">
                      启动约 30 秒后后台检查，且每 24 小时最多检查一次。
                    </span>
                  </span>
                </label>
              ) : null}
              <div className="flex flex-wrap items-center gap-2">
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={
                    updaterPhase === "checking" ||
                    updaterPhase === "downloading" ||
                    updaterPhase === "installing"
                  }
                  onClick={() => void checkForUpdates({ force: true })}
                >
                  {updaterPhase === "checking" ? "检查中…" : "检查更新"}
                </Button>
                {updaterPhase === "available" && updaterVersion ? (
                  <Button size="sm" onClick={() => void installUpdate()}>
                    安装 {updaterVersion}
                  </Button>
                ) : null}
              </div>
              <p className="text-muted">
                {updaterPhase === "upToDate"
                  ? "当前已是最新版本。"
                  : updaterPhase === "available" && updaterVersion
                    ? `发现新版本 ${updaterVersion}。`
                    : updaterPhase === "downloading" || updaterPhase === "installing"
                      ? "正在下载或安装更新…"
                      : updaterLastCheckedAt
                        ? `上次检查：${updaterLastCheckedAt}`
                        : "尚未检查更新。"}
              </p>
              {updaterProgress !== null &&
              (updaterPhase === "downloading" || updaterPhase === "installing") ? (
                <div>
                  <div className="mb-1 text-muted">进度 {updaterProgress}%</div>
                  <div className="h-1 overflow-hidden rounded-full bg-border">
                    <div
                      className="h-full bg-accent transition-[width]"
                      style={{ width: `${updaterProgress}%` }}
                    />
                  </div>
                </div>
              ) : null}
              {updaterError ? (
                <p className="text-danger">{updaterError}</p>
              ) : null}
            </div>
          </section>
        ) : null}

        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">快捷键</h2>
          <p className="mt-1 text-[12px] text-muted">
            全局快捷键在后台也可唤起。点击「更改」后按下新组合；保存后立即重新注册，无需重启。
          </p>
          {settings ? (
            <div className="mt-3 space-y-3 text-[12px]">
              {(
                [
                  "quickCapture",
                  "search",
                  "clipboard",
                  "focusMain",
                  "screenshotRegion",
                ] as const
              ).map((key) => (
                <ShortcutRow
                  key={key}
                  id={key}
                  value={settings.shortcuts[key]}
                  disabled={saveSettings.isPending || resetShortcuts.isPending}
                  onChange={(next) => saveShortcut(key, next)}
                />
              ))}
              <Button
                size="sm"
                variant="secondary"
                className="mt-1"
                onClick={() => resetShortcuts.mutate()}
                disabled={resetShortcuts.isPending}
              >
                恢复默认快捷键
              </Button>
            </div>
          ) : null}
        </section>

        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">快速记录语法</h2>
          <p className="mt-1 text-[12px] text-muted">
            在快速窗口「记录 → 任务/提醒」中输入自然语言，Trove 会解析日期、标签与优先级。完整说明见帮助文档。
          </p>
          <dl className="mt-3 space-y-2 text-[12px]">
            {QUICK_CAPTURE_SYNTAX.map((row) => (
              <div key={row.syntax} className="grid grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)] gap-3">
                <dt className="font-mono text-[11px] text-foreground">{row.syntax}</dt>
                <dd className="text-muted">{row.desc}</dd>
              </div>
            ))}
          </dl>
        </section>

        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">模板</h2>
          <p className="mt-1 text-[12px] text-muted">
            在命令面板搜索「模板」可一键应用。相对日期在应用时解析为当天偏移。
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="secondary"
              onClick={() =>
                void ipc
                  .templateCreate({
                    kind: "task",
                    name: "周报",
                    payload: {
                      title: "写周报",
                      relativeDueDays: 0,
                      priority: "medium",
                    },
                  })
                  .then(() => {
                    void queryClient.invalidateQueries({ queryKey: ["templates"] });
                    setMessage("已创建「周报」任务模板");
                  })
              }
            >
              添加示例：周报
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={() =>
                void ipc
                  .templateCreate({
                    kind: "task",
                    name: "报销",
                    payload: {
                      title: "提交报销",
                      relativeDueDays: 2,
                      priority: "low",
                    },
                  })
                  .then(() => {
                    void queryClient.invalidateQueries({ queryKey: ["templates"] });
                    setMessage("已创建「报销」任务模板");
                  })
              }
            >
              添加示例：报销
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => setShowCreateForm((v) => !v)}
            >
              {showCreateForm ? "收起表单" : "新建模板"}
            </Button>
          </div>
          {showCreateForm ? (
            <div className="mt-3 space-y-2 rounded-[var(--radius-control)] border border-border bg-surface p-3 text-[12px]">
              <div className="grid grid-cols-2 gap-2">
                <label className="flex flex-col gap-1">
                  <span className="text-muted">名称</span>
                  <Input
                    value={templateName}
                    onChange={(e) => setTemplateName(e.target.value)}
                    placeholder="模板名称"
                  />
                </label>
                <label className="flex flex-col gap-1">
                  <span className="text-muted">类型</span>
                  <select
                    className="h-8 w-full rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[13px] text-foreground outline-none focus:ring-2 focus:ring-accent/35"
                    value={templateKind}
                    onChange={(e) =>
                      handleKindChange(e.target.value as TemplateKind)
                    }
                  >
                    <option value="task">任务</option>
                    <option value="reminder">提醒</option>
                    <option value="memory">记忆</option>
                  </select>
                </label>
              </div>
              {templateKind === "task" ? (
                <div className="grid grid-cols-2 gap-2">
                  <label className="flex flex-col gap-1">
                    <span className="text-muted">标题</span>
                    <Input
                      value={taskTitle}
                      onChange={(e) => setTaskTitle(e.target.value)}
                      placeholder="任务标题"
                    />
                  </label>
                  <label className="flex flex-col gap-1">
                    <span className="text-muted">相对截止天数</span>
                    <Input
                      type="number"
                      min={0}
                      value={relativeDueDays}
                      onChange={(e) => setRelativeDueDays(e.target.value)}
                    />
                  </label>
                  <label className="flex flex-col gap-1">
                    <span className="text-muted">优先级</span>
                    <select
                      className="h-8 w-full rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[13px] text-foreground outline-none focus:ring-2 focus:ring-accent/35"
                      value={priority}
                      onChange={(e) =>
                        setPriority(e.target.value as TaskPriority)
                      }
                    >
                      <option value="none">无</option>
                      <option value="low">低</option>
                      <option value="medium">中</option>
                      <option value="high">高</option>
                    </select>
                  </label>
                </div>
              ) : null}
              {templateKind === "reminder" ? (
                <div className="grid grid-cols-2 gap-2">
                  <label className="flex flex-col gap-1">
                    <span className="text-muted">标题</span>
                    <Input
                      value={reminderTitle}
                      onChange={(e) => setReminderTitle(e.target.value)}
                      placeholder="提醒标题"
                    />
                  </label>
                  <label className="flex flex-col gap-1">
                    <span className="text-muted">相对触发小时</span>
                    <Input
                      type="number"
                      min={0}
                      value={relativeFireHours}
                      onChange={(e) => setRelativeFireHours(e.target.value)}
                    />
                  </label>
                  <div className="col-span-2">
                    <RecurrencePicker
                      value={reminderRecurrence}
                      onChange={setReminderRecurrence}
                      compact
                    />
                  </div>
                </div>
              ) : null}
              {templateKind === "memory" ? (
                <div className="space-y-2">
                  <label className="flex flex-col gap-1">
                    <span className="text-muted">标题</span>
                    <Input
                      value={memoryTitle}
                      onChange={(e) => setMemoryTitle(e.target.value)}
                      placeholder="记忆标题"
                    />
                  </label>
                  <label className="flex flex-col gap-1">
                    <span className="text-muted">正文</span>
                    <textarea
                      rows={3}
                      value={memoryBody}
                      onChange={(e) => setMemoryBody(e.target.value)}
                      className="w-full resize-none rounded-[var(--radius-control)] border border-border bg-surface-raised p-2 text-[13px] outline-none focus:ring-2 focus:ring-accent/35"
                      placeholder="记忆正文（可选）"
                    />
                  </label>
                </div>
              ) : null}
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-muted">
                  相对日期在应用时解析为当天偏移
                </span>
                <Button
                  size="sm"
                  disabled={!canCreate}
                  onClick={() =>
                    createTemplate.mutate({
                      kind: templateKind,
                      name: templateName.trim(),
                      payload: buildTemplatePayload(),
                    })
                  }
                >
                  创建
                </Button>
              </div>
            </div>
          ) : null}
          <ul className="mt-3 divide-y divide-border border-t border-border text-[12px]">
            {(templatesQuery.data ?? []).map((tpl) => (
              <li
                key={tpl.id}
                className="flex items-center justify-between gap-3 py-2"
              >
                <div>
                  <span className="font-medium">{tpl.name}</span>
                  <span className="ml-2 text-muted">{tpl.kind}</span>
                </div>
                <div className="flex gap-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => openPreview(tpl)}
                  >
                    应用
                  </Button>
                  <ConfirmButton
                    size="sm"
                    confirmLabel={`删除模板「${tpl.name}」？`}
                    onConfirm={() => {
                      void ipc.templateDelete(tpl.id).then(() => {
                        void queryClient.invalidateQueries({
                          queryKey: ["templates"],
                        });
                      });
                    }}
                    resetKey={tpl.id}
                  >
                    删除
                  </ConfirmButton>
                </div>
              </li>
            ))}
            {(templatesQuery.data ?? []).length === 0 ? (
              <li className="py-3 text-muted">暂无模板，可添加示例</li>
            ) : null}
          </ul>
        </section>

        {previewing ? (
          <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
            <button
              type="button"
              aria-label="关闭预览"
              className="absolute inset-0 cursor-default bg-black/40"
              onClick={closePreview}
            />
            <div
              className="relative max-h-[70vh] w-full max-w-md overflow-auto rounded-[var(--radius-panel)] border border-border bg-surface p-4 shadow-xl"
              role="dialog"
              aria-modal="true"
              aria-label={`预览模板「${previewing.name}」`}
            >
              <div className="mb-3 flex items-center justify-between">
                <h4 className="text-[13px] font-medium">
                  {previewing.name}
                  <span className="ml-2 text-muted">
                    {KIND_LABEL[previewing.kind]}
                  </span>
                </h4>
                <Button size="sm" variant="ghost" onClick={closePreview}>
                  关闭
                </Button>
              </div>
              {previewError ? (
                <p className="text-[12px] text-danger">{previewError}</p>
              ) : previewData ? (
                <>
                  <dl className="space-y-2 text-[12px]">
                    <div>
                      <dt className="text-muted">标题</dt>
                      <dd>{previewData.title}</dd>
                    </div>
                    {previewData.body ? (
                      <div>
                        <dt className="text-muted">正文</dt>
                        <dd className="whitespace-pre-wrap">
                          {previewData.body}
                        </dd>
                      </div>
                    ) : null}
                    {previewData.dueDate || previewData.dueTime ? (
                      <div>
                        <dt className="text-muted">截止时间</dt>
                        <dd>
                          {previewData.dueDate}
                          {previewData.dueDate && previewData.dueTime
                            ? " "
                            : ""}
                          {previewData.dueTime ?? ""}
                        </dd>
                      </div>
                    ) : null}
                    {previewData.fireAt ? (
                      <div>
                        <dt className="text-muted">触发时间</dt>
                        <dd>{previewData.fireAt.replace("T", " ")}</dd>
                      </div>
                    ) : null}
                    {previewData.priority ? (
                      <div>
                        <dt className="text-muted">优先级</dt>
                        <dd>{PRIORITY_LABEL[previewData.priority]}</dd>
                      </div>
                    ) : null}
                    {previewData.tagNames.length > 0 ? (
                      <div>
                        <dt className="text-muted">标签</dt>
                        <dd>{previewData.tagNames.join("、")}</dd>
                      </div>
                    ) : null}
                    {previewData.recurrence ? (
                      <div>
                        <dt className="text-muted">周期</dt>
                        <dd>{recurrenceLabel(previewData.recurrence)}</dd>
                      </div>
                    ) : null}
                  </dl>
                  <div className="mt-4 flex justify-end gap-2">
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={closePreview}
                      disabled={applyTemplate.isPending}
                    >
                      取消
                    </Button>
                    <Button
                      size="sm"
                      onClick={() => applyTemplate.mutate(previewing.id)}
                      disabled={applyTemplate.isPending}
                    >
                      确认应用
                    </Button>
                  </div>
                </>
              ) : (
                <p className="p-4 text-center text-[12px] text-muted">
                  解析中…
                </p>
              )}
            </div>
          </div>
        ) : null}

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
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={settings.clipboardSmartActionsEnabled}
                  onChange={(e) =>
                    saveSettings.mutate({
                      ...settings,
                      clipboardSmartActionsEnabled: e.target.checked,
                    })
                  }
                />
                启用智能行动（类型识别与行动气泡，全程本地）
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
            </div>
          ) : null}
        </section>

        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">备份与数据</h2>
          <p className="mt-1 text-[12px] text-muted">
            自动本地备份保存在应用数据目录。导入会覆盖当前业务数据，导入前会自动备份。
          </p>
          {settings ? (
            <div className="mt-3 space-y-3 text-[12px]">
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={settings.autoBackupOnLaunch}
                  onChange={(e) =>
                    saveSettings.mutate({
                      ...settings,
                      autoBackupOnLaunch: e.target.checked,
                    })
                  }
                />
                启动时自动备份
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-muted">保留备份数量</span>
                <Input
                  type="number"
                  min={1}
                  max={100}
                  value={backupKeepText}
                  onChange={(e) => setBackupKeepText(e.target.value)}
                  onBlur={persistClipboardFields}
                />
              </label>
              <div className="flex flex-wrap gap-2">
                <Button
                  size="sm"
                  onClick={() => createBackup.mutate()}
                  disabled={createBackup.isPending}
                >
                  立即备份
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => exportData.mutate()}
                  disabled={exportData.isPending}
                >
                  导出全部数据…
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => fileRef.current?.click()}
                  disabled={importData.isPending}
                >
                  导入 JSON…
                </Button>
                <input
                  ref={fileRef}
                  type="file"
                  accept="application/json,.json"
                  className="hidden"
                  onChange={(e) => {
                    const file = e.target.files?.[0];
                    e.target.value = "";
                    if (!file) return;
                    setPendingImportFile(file);
                  }}
                />
                {pendingImportFile ? (
                  <div className="flex flex-wrap items-center gap-2 rounded-[var(--radius-control)] border border-danger/40 bg-danger/5 p-2">
                    <span className="text-[12px]">
                      导入将覆盖当前任务、提醒、记忆与剪切板等业务数据。确认导入{" "}
                      {pendingImportFile.name}？
                    </span>
                    <Button
                      size="sm"
                      variant="danger"
                      disabled={importData.isPending}
                      onClick={() => {
                        void pendingImportFile
                          .text()
                          .then((text) => importData.mutate(text));
                        setPendingImportFile(null);
                      }}
                    >
                      确认导入
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={importData.isPending}
                      onClick={() => setPendingImportFile(null)}
                    >
                      取消
                    </Button>
                  </div>
                ) : null}
              </div>
              <ul className="divide-y divide-border border-t border-border">
                {(backupsQuery.data ?? []).slice(0, 8).map((item) => (
                  <li
                    key={item.fileName}
                    className="flex items-center justify-between gap-3 py-2"
                  >
                    <div className="min-w-0">
                      <p className="truncate font-mono text-[12px]">
                        {item.fileName}
                      </p>
                      <p className="text-[11px] text-muted">
                        {item.createdAt} · {formatBytes(item.sizeBytes)} ·{" "}
                        {item.reason}
                      </p>
                    </div>
                    <ConfirmButton
                      size="sm"
                      confirmLabel={`恢复 ${item.fileName}？`}
                      confirmTitle={`恢复备份 ${item.fileName}？当前数据会先自动备份。`}
                      onConfirm={() => restoreBackup.mutate(item.fileName)}
                      resetKey={item.fileName}
                    >
                      恢复
                    </ConfirmButton>
                  </li>
                ))}
                {(backupsQuery.data ?? []).length === 0 ? (
                  <li className="py-3 text-muted">暂无备份</li>
                ) : null}
              </ul>
              {health ? (
                <p className="break-all text-[11px] text-muted">
                  备份目录：{health.backup.directory}
                </p>
              ) : null}
            </div>
          ) : null}
        </section>

        <AutomationRulesSection
          settings={settings}
          onSaveSettings={(next) => saveSettings.mutate(next)}
          onMessage={setMessage}
        />

        <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
          <h2 className="text-[13px] font-semibold">权限与隐私</h2>
          <p className="mt-1 text-[12px] text-muted">
            所有业务数据保存在本机 SQLite，不上传云端。日志不记录任务正文、记忆正文或剪切板内容。
          </p>
          {health ? (
            <ul className="mt-3 space-y-3 text-[12px]">
              {Object.entries(health.capabilities).map(([key, value]) => (
                <li key={key} className="rounded border border-border p-2">
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-medium">
                      {capLabel[key] ?? key}
                    </span>
                    <span className={value.available ? "text-muted" : "text-danger"}>
                      {value.available ? "可用 / 可降级" : "受限"}
                    </span>
                  </div>
                  <p className="mt-1 text-muted">{value.notes}</p>
                </li>
              ))}
            </ul>
          ) : null}
        </section>
      </div>
    </PageScaffold>
  );
}

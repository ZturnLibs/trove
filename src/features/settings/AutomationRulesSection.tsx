import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Button } from "@/design-system/primitives/Button";
import { Input } from "@/design-system/primitives/Input";
import {
  ipc,
  type AppSettings,
  type AutomationAction,
  type AutomationRule,
  type AutomationRuleDefinition,
  type AutomationTrigger,
} from "@/ipc/client";

const TRIGGER_LABEL: Record<AutomationTrigger["kind"], string> = {
  taskCreated: "创建任务后",
  reminderCreated: "创建提醒后",
  memoryCreated: "创建记忆后",
  clipboardFavorited: "剪切板收藏后",
  reminderFired: "提醒触发后",
  taskMovedToList: "任务移入清单后",
  taskTagAdded: "任务添加标签后",
};

const ACTION_LABEL: Record<AutomationAction["kind"], string> = {
  setPriority: "设为高优先级",
  moveToList: "移入清单",
  addTag: "添加标签",
  pinMemory: "置顶记忆",
  notify: "显示通知",
};

type Props = {
  settings: AppSettings | undefined;
  onSaveSettings: (next: AppSettings) => void;
  onMessage: (msg: string) => void;
};

export function AutomationRulesSection({
  settings,
  onSaveSettings,
  onMessage,
}: Props) {
  const queryClient = useQueryClient();
  const rulesQuery = useQuery({
    queryKey: ["automation", "rules"],
    queryFn: () => ipc.automationList(),
  });
  const runsQuery = useQuery({
    queryKey: ["automation", "runs"],
    queryFn: () => ipc.automationRunsList(null, 20),
  });

  const [showCreate, setShowCreate] = useState(false);
  const [name, setName] = useState("");
  const [triggerKind, setTriggerKind] =
    useState<AutomationTrigger["kind"]>("taskCreated");
  const [keyword, setKeyword] = useState("");
  const [actionKind, setActionKind] =
    useState<Extract<AutomationAction["kind"], "setPriority" | "addTag" | "notify">>(
      "setPriority",
    );
  const [tagName, setTagName] = useState("");
  const [notifyTitle, setNotifyTitle] = useState("自动化");
  const [notifyBody, setNotifyBody] = useState("规则已匹配");

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["automation"] });
  };

  const createRule = useMutation({
    mutationFn: (definition: AutomationRuleDefinition) =>
      ipc.automationCreate({ name: name.trim(), definition }),
    onSuccess: () => {
      invalidate();
      setShowCreate(false);
      setName("");
      setKeyword("");
      onMessage("规则已创建");
    },
    onError: (err) =>
      onMessage(err instanceof Error ? err.message : "创建规则失败"),
  });

  const toggleRule = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      ipc.automationSetEnabled(id, enabled),
    onSuccess: () => invalidate(),
    onError: (err) =>
      onMessage(err instanceof Error ? err.message : "更新规则失败"),
  });

  const deleteRule = useMutation({
    mutationFn: (id: string) => ipc.automationDelete(id),
    onSuccess: () => {
      invalidate();
      onMessage("规则已删除");
    },
    onError: (err) =>
      onMessage(err instanceof Error ? err.message : "删除规则失败"),
  });

  const buildDefinition = (): AutomationRuleDefinition | null => {
    const trigger = buildTrigger(triggerKind);
    const conditions = keyword.trim()
      ? [
          {
            kind: "titleContains" as const,
            text: keyword.trim(),
            caseInsensitive: true,
          },
        ]
      : [];
    let action: AutomationAction | null = null;
    if (actionKind === "setPriority") {
      action = { kind: "setPriority", priority: "high" };
    } else if (actionKind === "addTag") {
      if (!tagName.trim()) return null;
      action = { kind: "addTag", tagName: tagName.trim() };
    } else if (actionKind === "notify") {
      if (!notifyTitle.trim()) return null;
      action = {
        kind: "notify",
        title: notifyTitle.trim(),
        body: notifyBody.trim(),
      };
    }
    if (!action) return null;
    return { trigger, conditions, actions: [action] };
  };

  const handleCreate = () => {
    if (!name.trim()) {
      onMessage("请填写规则名称");
      return;
    }
    const definition = buildDefinition();
    if (!definition) {
      onMessage("请完善动作参数");
      return;
    }
    createRule.mutate(definition);
  };

  return (
    <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
      <h2 className="text-[13px] font-semibold">规则自动化</h2>
      <p className="mt-1 text-[12px] text-muted">
        本地规则引擎：创建/收藏/提醒等事件触发后自动执行已注册动作（不执行任意脚本）。
      </p>

      {settings ? (
        <label className="mt-3 flex items-center gap-2 text-[12px]">
          <input
            type="checkbox"
            checked={settings.automationEnabled}
            onChange={(e) =>
              onSaveSettings({
                ...settings,
                automationEnabled: e.target.checked,
              })
            }
          />
          启用规则自动化（全局开关）
        </label>
      ) : null}

      <div className="mt-3 flex flex-wrap gap-2">
        <Button
          size="sm"
          variant="secondary"
          onClick={() => setShowCreate((v) => !v)}
        >
          {showCreate ? "取消新建" : "新建规则"}
        </Button>
      </div>

      {showCreate ? (
        <div className="mt-3 space-y-3 rounded border border-border p-3 text-[12px]">
          <Input
            placeholder="规则名称"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <label className="flex flex-col gap-1">
            <span className="text-muted">触发器</span>
            <select
              className="rounded border border-border bg-surface px-2 py-1"
              value={triggerKind}
              onChange={(e) =>
                setTriggerKind(e.target.value as AutomationTrigger["kind"])
              }
            >
              {Object.entries(TRIGGER_LABEL).map(([kind, label]) => (
                <option key={kind} value={kind}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          <Input
            placeholder="标题关键词（可选，留空则匹配全部）"
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
          />
          <label className="flex flex-col gap-1">
            <span className="text-muted">动作</span>
            <select
              className="rounded border border-border bg-surface px-2 py-1"
              value={actionKind}
              onChange={(e) =>
                setActionKind(
                  e.target.value as typeof actionKind,
                )
              }
            >
              <option value="setPriority">设为高优先级</option>
              <option value="addTag">添加标签</option>
              <option value="notify">显示本地通知</option>
            </select>
          </label>
          {actionKind === "addTag" ? (
            <Input
              placeholder="标签名"
              value={tagName}
              onChange={(e) => setTagName(e.target.value)}
            />
          ) : null}
          {actionKind === "notify" ? (
            <>
              <Input
                placeholder="通知标题"
                value={notifyTitle}
                onChange={(e) => setNotifyTitle(e.target.value)}
              />
              <Input
                placeholder="通知正文"
                value={notifyBody}
                onChange={(e) => setNotifyBody(e.target.value)}
              />
            </>
          ) : null}
          <Button size="sm" onClick={handleCreate} disabled={createRule.isPending}>
            保存规则
          </Button>
        </div>
      ) : null}

      <ul className="mt-3 divide-y divide-border border-t border-border">
        {(rulesQuery.data ?? []).map((rule) => (
          <RuleRow
            key={rule.id}
            rule={rule}
            onToggle={(enabled) =>
              toggleRule.mutate({ id: rule.id, enabled })
            }
            onDelete={() => deleteRule.mutate(rule.id)}
          />
        ))}
        {(rulesQuery.data ?? []).length === 0 ? (
          <li className="py-3 text-[12px] text-muted">暂无规则</li>
        ) : null}
      </ul>

      {(runsQuery.data ?? []).length > 0 ? (
        <div className="mt-4">
          <h3 className="text-[12px] font-medium">最近执行</h3>
          <ul className="mt-2 max-h-40 space-y-1 overflow-y-auto text-[11px] text-muted">
            {(runsQuery.data ?? []).map((run) => (
              <li key={run.id}>
                {run.createdAt} · {run.ruleName} · {run.status}
                {run.errorSummary ? ` · ${run.errorSummary}` : ""}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </section>
  );
}

function RuleRow({
  rule,
  onToggle,
  onDelete,
}: {
  rule: AutomationRule;
  onToggle: (enabled: boolean) => void;
  onDelete: () => void;
}) {
  const triggerLabel = TRIGGER_LABEL[rule.definition.trigger.kind] ?? rule.definition.trigger.kind;
  const actionLabels = rule.definition.actions
    .map((a) => ACTION_LABEL[a.kind] ?? a.kind)
    .join("、");

  return (
    <li className="flex flex-wrap items-center justify-between gap-2 py-2 text-[12px]">
      <div className="min-w-0">
        <p className="font-medium">{rule.name}</p>
        <p className="text-muted">
          {triggerLabel}
          {rule.definition.conditions.length > 0 ? " · 含条件" : ""} → {actionLabels}
        </p>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <label className="flex items-center gap-1">
          <input
            type="checkbox"
            checked={rule.enabled}
            onChange={(e) => onToggle(e.target.checked)}
          />
          启用
        </label>
        <Button size="sm" variant="ghost" onClick={onDelete}>
          删除
        </Button>
      </div>
    </li>
  );
}

function buildTrigger(kind: AutomationTrigger["kind"]): AutomationTrigger {
  switch (kind) {
    case "taskCreated":
      return { kind: "taskCreated" };
    case "reminderCreated":
      return { kind: "reminderCreated" };
    case "memoryCreated":
      return { kind: "memoryCreated" };
    case "clipboardFavorited":
      return { kind: "clipboardFavorited" };
    case "reminderFired":
      return { kind: "reminderFired" };
    case "taskMovedToList":
      return { kind: "taskMovedToList" };
    case "taskTagAdded":
      return { kind: "taskTagAdded" };
  }
}

import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc, type AIConfig, type AIMode, type AppSettings, type ProbeReport } from "@/ipc/client";
import { Button } from "@/design-system/primitives/Button";
import { ConfirmButton } from "@/design-system/patterns/ConfirmButton";

const HINT_COPY: Record<string, string> = {
  "ai.off": "AI 功能未开启。",
  "ai.probe.endpoint-missing": "请先填写接口地址。",
  "ai.probe.model-missing": "请先填写模型名。",
  "ai.probe.key-missing": "请先设置 API Key。",
  "ai.probe.ollama-guide":
    "无法连接本地 Ollama。请确认已安装并运行（默认 http://localhost:11434），模型可用 ollama pull 拉取。",
  "ai.probe.unreachable": "无法连接服务，请检查地址、密钥与网络。",
};

const FEATURES: { key: keyof AIConfig["features"]; label: string }[] = [
  { key: "extract", label: "长文本提取任务草稿（已开放）" },
  { key: "related", label: "相关内容建议（已开放）" },
  { key: "summary", label: "回顾摘要组织文字（已开放）" },
  { key: "suggest", label: "每日工作建议（已开放）" },
  { key: "split", label: "任务拆分检查项" },
];

const STATUS_COPY: Record<AISuggestionStatus, string> = {
  pending: "待处理",
  accepted: "已接受",
  rejected: "已拒绝",
  dismissed: "已忽略",
};

type AISuggestionStatus = "pending" | "accepted" | "rejected" | "dismissed";

function probeText(report: ProbeReport): string {
  if (report.reachable) {
    return `连接正常（${report.model ?? "未知模型"}，${report.latencyMs ?? "?"}ms）`;
  }
  return (report.hint && HINT_COPY[report.hint]) || "连接失败。";
}

export function AIAssistSection({ settings }: { settings: AppSettings }) {
  const queryClient = useQueryClient();
  const [probe, setProbe] = useState<ProbeReport | null>(null);
  const [keyInput, setKeyInput] = useState("");
  const [message, setMessage] = useState<string | null>(null);

  const keyStatusQuery = useQuery({
    queryKey: ["ai", "provider-key"],
    queryFn: () => ipc.aiProviderKeyStatus(),
  });
  const historyQuery = useQuery({
    queryKey: ["ai", "suggestions"],
    queryFn: () => ipc.aiSuggestionList(),
  });

  const saveMutation = useMutation({
    mutationFn: (patch: Partial<AIConfig>) =>
      ipc.settingsSave({
        ...settings,
        ai: { ...settings.ai, ...patch },
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
      setProbe(null);
    },
  });

  const probeMutation = useMutation({
    mutationFn: () => ipc.aiProviderProbe(),
    onSuccess: (report) => setProbe(report),
  });

  const keySetMutation = useMutation({
    mutationFn: (key: string) => ipc.aiProviderKeySet(key),
    onSuccess: () => {
      setKeyInput("");
      setMessage("API Key 已保存到本机文件（不进入数据库与备份）。");
      queryClient.invalidateQueries({ queryKey: ["ai", "provider-key"] });
    },
  });

  const keyClearMutation = useMutation({
    mutationFn: () => ipc.aiProviderKeyClear(),
    onSuccess: () => {
      setMessage("API Key 已清除。");
      queryClient.invalidateQueries({ queryKey: ["ai", "provider-key"] });
    },
  });

  const clearHistoryMutation = useMutation({
    mutationFn: () => ipc.aiSuggestionClear(),
    onSuccess: (count) => {
      setMessage(`已清空 ${count} 条建议历史（不影响任何业务数据）。`);
      queryClient.invalidateQueries({ queryKey: ["ai", "suggestions"] });
    },
  });

  const ai = settings.ai;
  const mode = ai.mode;
  const history = historyQuery.data ?? [];

  return (
    <section className="rounded-[var(--radius-panel)] border border-border bg-surface-raised p-4">
      <h2 className="text-[13px] font-semibold">智能辅助</h2>
      <p className="mt-1 text-[12px] text-muted">
        AI 只作为任务、记忆、搜索流程中的辅助：建议必带来源、修改必须经你确认、每个功能可单独关闭。关闭或不可用时，全部既有功能不受影响。
      </p>

      <div className="mt-3 space-y-3 text-[12px]">
        <div className="flex flex-wrap gap-3">
          {(
            [
              ["off", "关闭（默认）"],
              ["ollama", "本地 Ollama"],
              ["custom", "自定义远程"],
            ] as [AIMode, string][]
          ).map(([value, label]) => (
            <label key={value} className="flex items-center gap-1">
              <input
                type="radio"
                name="ai-mode"
                checked={mode === value}
                onChange={() => saveMutation.mutate({ mode: value })}
              />
              {label}
            </label>
          ))}
        </div>

        {mode !== "off" ? (
          <div className="space-y-2 rounded border border-border p-2">
            {mode === "ollama" ? (
              <>
                <label className="block">
                  <span className="text-muted">Ollama 地址</span>
                  <input
                    className="mt-0.5 w-full rounded border border-border bg-surface px-2 py-1"
                    defaultValue={ai.ollamaUrl}
                    onBlur={(e) =>
                      e.target.value !== ai.ollamaUrl &&
                      saveMutation.mutate({ ollamaUrl: e.target.value })
                    }
                  />
                </label>
                <label className="block">
                  <span className="text-muted">模型名（如 qwen3:4b）</span>
                  <input
                    className="mt-0.5 w-full rounded border border-border bg-surface px-2 py-1"
                    defaultValue={ai.ollamaModel}
                    placeholder="ollama pull 拉取后填写"
                    onBlur={(e) =>
                      e.target.value !== ai.ollamaModel &&
                      saveMutation.mutate({ ollamaModel: e.target.value })
                    }
                  />
                </label>
              </>
            ) : (
              <>
                <label className="block">
                  <span className="text-muted">接口地址（OpenAI 兼容，含 /v1）</span>
                  <input
                    className="mt-0.5 w-full rounded border border-border bg-surface px-2 py-1"
                    defaultValue={ai.customEndpoint}
                    placeholder="https://api.example.com/v1"
                    onBlur={(e) =>
                      e.target.value !== ai.customEndpoint &&
                      saveMutation.mutate({ customEndpoint: e.target.value })
                    }
                  />
                </label>
                <label className="block">
                  <span className="text-muted">模型名</span>
                  <input
                    className="mt-0.5 w-full rounded border border-border bg-surface px-2 py-1"
                    defaultValue={ai.customModel}
                    onBlur={(e) =>
                      e.target.value !== ai.customModel &&
                      saveMutation.mutate({ customModel: e.target.value })
                    }
                  />
                </label>
                <div>
                  <span className="text-muted">API Key（只写入本机文件，不回显）</span>
                  <div className="mt-0.5 flex gap-2">
                    <input
                      type="password"
                      className="w-full rounded border border-border bg-surface px-2 py-1"
                      value={keyInput}
                      placeholder={
                        keyStatusQuery.data?.exists ? "已设置（输入可覆盖）" : "未设置"
                      }
                      onChange={(e) => setKeyInput(e.target.value)}
                    />
                    <Button
                      size="sm"
                      variant="secondary"
                      disabled={!keyInput.trim()}
                      onClick={() => keySetMutation.mutate(keyInput.trim())}
                    >
                      保存
                    </Button>
                    {keyStatusQuery.data?.exists ? (
                      <ConfirmButton
                        size="sm"
                        confirmLabel="确认清除"
                        onConfirm={() => keyClearMutation.mutate()}
                      >
                        清除
                      </ConfirmButton>
                    ) : null}
                  </div>
                </div>
              </>
            )}

            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="secondary"
                disabled={probeMutation.isPending}
                onClick={() => probeMutation.mutate()}
              >
                测试连接
              </Button>
              {probeMutation.isPending ? <span className="text-muted">检测中…</span> : null}
              {probe ? <span>{probeText(probe)}</span> : null}
            </div>

            <fieldset className="space-y-1">
              <legend className="text-muted">功能开关（逐项可关，默认全部关闭）</legend>
              {FEATURES.map((f) => (
                <label key={f.key} className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={ai.features[f.key]}
                    onChange={(e) =>
                      saveMutation.mutate({
                        features: { ...ai.features, [f.key]: e.target.checked },
                      })
                    }
                  />
                  <span>{f.label}</span>
                </label>
              ))}
            </fieldset>

            <p className="text-muted">
              数据边界：敏感记忆与排除应用的剪贴板内容永远不会被发送；远程模式仅发送你确认功能所需的最小上下文，密钥只保存在本机。
            </p>
          </div>
        ) : (
          <p className="text-muted">当前未配置任何模型服务，所有功能按原样运行。</p>
        )}

        <div className="border-t border-border pt-2">
          <div className="flex items-center justify-between">
            <span className="text-muted">建议历史（{history.length}）</span>
            {history.length > 0 ? (
              <ConfirmButton
                size="sm"
                confirmLabel="确认清空"
                confirmTitle="清空建议历史不影响任何业务数据"
                onConfirm={() => clearHistoryMutation.mutate()}
              >
                清空建议历史
              </ConfirmButton>
            ) : null}
          </div>
          {history.length === 0 ? (
            <p className="mt-1 text-muted">暂无建议记录。</p>
          ) : (
            <ul className="mt-1 max-h-40 space-y-1 overflow-auto">
              {history.slice(0, 20).map((r) => (
                <li key={r.id} className="flex items-center gap-2 text-[11px] text-muted">
                  <span className="rounded border border-border px-1">{r.featureType}</span>
                  <span>{STATUS_COPY[r.status as AISuggestionStatus]}</span>
                  <span className="truncate">
                    {r.payload.items[0]?.title ?? r.payload.summary ?? "（无效输出已丢弃）"}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>

        {message ? <p className="text-muted">{message}</p> : null}
      </div>
    </section>
  );
}

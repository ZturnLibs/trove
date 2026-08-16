import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { PageScaffold } from "@/components/PageScaffold";
import { Button } from "@/design-system/primitives/Button";
import { ipc, type HealthDashboardSnapshot, type ReminderOutcomeStats } from "@/ipc/client";
import { cn } from "@/lib/cn";

function formatBytes(n: number) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function pct(part: number, total: number) {
  if (total <= 0) return 0;
  return Math.round((part / total) * 100);
}

function StatCard({
  title,
  hint,
  children,
}: {
  title: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-[var(--radius-panel)] border border-border bg-surface p-3">
      <h2 className="text-[13px] font-medium">{title}</h2>
      {hint ? <p className="mt-0.5 text-[11px] text-muted">{hint}</p> : null}
      <div className="mt-2">{children}</div>
    </section>
  );
}

function ReminderStatsBlock({ label, stats }: { label: string; stats: ReminderOutcomeStats }) {
  const total = stats.onTime + stats.snoozed + stats.missed + stats.pendingOverdue;
  const rows = [
    { key: "onTime", label: "按时完成", value: stats.onTime, color: "bg-emerald-500/70" },
    { key: "snoozed", label: "贪睡", value: stats.snoozed, color: "bg-amber-500/70" },
    { key: "missed", label: "错过", value: stats.missed, color: "bg-rose-500/70" },
    {
      key: "pending",
      label: "仍逾期",
      value: stats.pendingOverdue,
      color: "bg-muted/50",
    },
  ];

  return (
    <div>
      <p className="mb-2 text-[11px] font-medium text-muted">{label}</p>
      {total === 0 ? (
        <p className="text-[12px] text-muted">窗口内暂无到期提醒记录</p>
      ) : (
        <div className="space-y-2">
          <div className="flex h-2 overflow-hidden rounded-full bg-surface-raised">
            {rows.map((row) =>
              row.value > 0 ? (
                <div
                  key={row.key}
                  className={cn(row.color, "h-full")}
                  style={{ width: `${pct(row.value, total)}%` }}
                  title={`${row.label} ${row.value}`}
                />
              ) : null,
            )}
          </div>
          <dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-[12px]">
            {rows.map((row) => (
              <div key={row.key} className="flex justify-between gap-2">
                <dt className="text-muted">{row.label}</dt>
                <dd>
                  {row.value}
                  <span className="ml-1 text-muted">({pct(row.value, total)}%)</span>
                </dd>
              </div>
            ))}
          </dl>
        </div>
      )}
    </div>
  );
}

function CompletionTrend({ snap }: { snap: HealthDashboardSnapshot }) {
  const max = Math.max(1, ...snap.tasks.completionTrend.map((d) => d.count));
  return (
    <div className="flex items-end gap-1.5 h-16">
      {snap.tasks.completionTrend.map((day) => (
        <div key={day.date} className="flex min-w-0 flex-1 flex-col items-center gap-1">
          <div
            className="w-full rounded-sm bg-accent/60"
            style={{ height: `${Math.max(4, (day.count / max) * 100)}%` }}
            title={`${day.date}: ${day.count}`}
          />
          <span className="text-[9px] text-muted">{day.date.slice(5)}</span>
        </div>
      ))}
    </div>
  );
}

function StorageBar({
  snap,
  onRunGc,
  gcPending,
}: {
  snap: HealthDashboardSnapshot;
  onRunGc: () => void;
  gcPending: boolean;
}) {
  const parts = [
    { label: "数据库", bytes: snap.storage.databaseBytes + snap.storage.walBytes },
    { label: "资源", bytes: Math.max(0, snap.storage.assetsBytes) },
    { label: "缩略图", bytes: snap.storage.thumbBytes },
    { label: "备份", bytes: snap.backupTotalBytes },
  ];
  const total = parts.reduce((sum, p) => sum + p.bytes, 0) || 1;
  const colors = ["bg-blue-500/70", "bg-violet-500/70", "bg-cyan-500/70", "bg-orange-500/70"];

  return (
    <div className="space-y-2">
      <div className="flex h-3 overflow-hidden rounded-full bg-surface-raised">
        {parts.map((part, i) =>
          part.bytes > 0 ? (
            <div
              key={part.label}
              className={cn(colors[i], "h-full")}
              style={{ width: `${pct(part.bytes, total)}%` }}
              title={`${part.label} ${formatBytes(part.bytes)}`}
            />
          ) : null,
        )}
      </div>
      <dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-[12px]">
        {parts.map((part) => (
          <div key={part.label} className="flex justify-between gap-2">
            <dt className="text-muted">{part.label}</dt>
            <dd>{formatBytes(part.bytes)}</dd>
          </div>
        ))}
      </dl>
      <p className="text-[11px] text-muted">{snap.storage.note}</p>
      <div className="rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 py-2 text-[12px]">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-muted">
            可回收孤儿资源 {snap.storageGc.candidateCount} 项 ·{" "}
            {formatBytes(snap.storageGc.candidateBytes)}（保留{" "}
            {snap.storageGc.retentionDays} 天）
          </span>
          <Button
            size="sm"
            variant="secondary"
            disabled={snap.storageGc.candidateCount === 0 || gcPending}
            onClick={onRunGc}
          >
            {gcPending ? "清理中…" : "清理可回收资源"}
          </Button>
        </div>
        <p className="mt-1 text-[10px] text-muted">{snap.storageGc.note}</p>
      </div>
    </div>
  );
}

export function HealthDashboardPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const snapQuery = useQuery({
    queryKey: ["health-dashboard", "snapshot"],
    queryFn: () => ipc.healthDashboardSnapshot(),
  });

  const snap = snapQuery.data;

  const gcMutation = useMutation({
    mutationFn: () => ipc.storageRunAssetsGc(),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["health-dashboard"] });
    },
  });

  return (
    <PageScaffold
      title="健康仪表盘"
      description="本地数据、备份与提醒遵守情况 — 纯统计，不含效率评分"
      actions={
        <Button
          size="sm"
          variant="secondary"
          onClick={() => void snapQuery.refetch()}
          disabled={snapQuery.isFetching}
        >
          {snapQuery.isFetching ? "刷新中…" : "刷新"}
        </Button>
      }
    >
      <div className="mx-auto max-w-3xl space-y-4 p-4">
        {snapQuery.isLoading ? (
          <p className="text-[12px] text-muted">加载中…</p>
        ) : snapQuery.isError ? (
          <p className="text-[12px] text-danger">无法加载健康数据</p>
        ) : snap ? (
          <>
            {snap.backup.lastError ? (
              <div className="rounded-[var(--radius-panel)] border border-danger/40 bg-danger/5 px-3 py-2 text-[12px] text-danger">
                备份异常：{snap.backup.lastError}
              </div>
            ) : null}

            <div className="grid gap-3 sm:grid-cols-2">
              <StatCard title="备份" hint="与设置页手动备份结果一致">
                <dl className="space-y-1 text-[12px]">
                  <div className="flex justify-between gap-2">
                    <dt className="text-muted">轮转数量</dt>
                    <dd>{snap.backup.count}</dd>
                  </div>
                  <div className="flex justify-between gap-2">
                    <dt className="text-muted">最近成功</dt>
                    <dd>{snap.backup.latestCreatedAt ?? "无"}</dd>
                  </div>
                  <div className="flex justify-between gap-2">
                    <dt className="text-muted">目录占用</dt>
                    <dd>{formatBytes(snap.backupTotalBytes)}</dd>
                  </div>
                </dl>
                <Button
                  size="sm"
                  variant="ghost"
                  className="mt-2"
                  onClick={() => navigate("/settings")}
                >
                  前往设置管理备份
                </Button>
              </StatCard>

              <StatCard title="存储分布">
                <StorageBar
                  snap={snap}
                  onRunGc={() => gcMutation.mutate()}
                  gcPending={gcMutation.isPending}
                />
              </StatCard>

              <StatCard title="提醒遵守率" hint="按本地时区统计到期窗口">
                <div className="space-y-4">
                  <ReminderStatsBlock label="近 7 天" stats={snap.reminders7d} />
                  <ReminderStatsBlock label="近 30 天" stats={snap.reminders30d} />
                </div>
              </StatCard>

              <StatCard title="任务与收件箱">
                <dl className="space-y-1 text-[12px]">
                  <div className="flex justify-between gap-2">
                    <dt className="text-muted">收件箱待处理</dt>
                    <dd>{snap.tasks.inboxCount}</dd>
                  </div>
                  <div className="flex justify-between gap-2">
                    <dt className="text-muted">最久积压</dt>
                    <dd>
                      {snap.tasks.inboxOldestDays != null
                        ? `${snap.tasks.inboxOldestDays} 天`
                        : "—"}
                    </dd>
                  </div>
                  <div className="flex justify-between gap-2">
                    <dt className="text-muted">14 天未更新</dt>
                    <dd>{snap.tasks.staleActiveCount}</dd>
                  </div>
                </dl>
                <p className="mt-3 mb-1 text-[11px] text-muted">近 7 天完成趋势</p>
                <CompletionTrend snap={snap} />
              </StatCard>

              <StatCard title="剪贴板" hint="保留余量与收藏占比">
                <dl className="space-y-1 text-[12px]">
                  <div className="flex justify-between gap-2">
                    <dt className="text-muted">当前条数</dt>
                    <dd>
                      {snap.clipboard.totalCount} / {snap.clipboard.maxItems}
                    </dd>
                  </div>
                  <div className="flex justify-between gap-2">
                    <dt className="text-muted">剩余额度</dt>
                    <dd>{snap.clipboard.remainingSlots}</dd>
                  </div>
                  <div className="flex justify-between gap-2">
                    <dt className="text-muted">收藏占比</dt>
                    <dd>
                      {snap.clipboard.totalCount > 0
                        ? `${pct(snap.clipboard.favoriteCount, snap.clipboard.totalCount)}%`
                        : "—"}
                    </dd>
                  </div>
                  <div className="flex justify-between gap-2">
                    <dt className="text-muted">保留天数</dt>
                    <dd>{snap.clipboard.retentionDays} 天</dd>
                  </div>
                </dl>
                <div className="mt-2 h-2 overflow-hidden rounded-full bg-surface-raised">
                  <div
                    className="h-full bg-accent/60"
                    style={{
                      width: `${pct(snap.clipboard.totalCount, snap.clipboard.maxItems)}%`,
                    }}
                  />
                </div>
              </StatCard>
            </div>

            <p className="text-[11px] text-muted">
              快照时间 {snap.generatedAt} · 数据均来自本地 SQL 聚合
            </p>
          </>
        ) : null}
      </div>
    </PageScaffold>
  );
}

import { EmptyState, PageScaffold } from "@/components/PageScaffold";

export function TodayPage() {
  return (
    <PageScaffold title="今日" description="逾期事项优先，其次是今天的任务与提醒">
      <EmptyState
        title="今日还没有事项"
        body="阶段 1 将接入任务列表。你也可以用全局快捷键快速记录。"
      />
    </PageScaffold>
  );
}

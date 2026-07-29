import { EmptyState, PageScaffold } from "@/components/PageScaffold";

export function InboxPage() {
  return (
    <PageScaffold title="收件箱" description="尚未整理的新任务">
      <EmptyState title="收件箱为空" body="新任务默认进入这里，整理后再分到清单。" />
    </PageScaffold>
  );
}

import { EmptyState, PageScaffold } from "@/components/PageScaffold";

export function MemoryPage() {
  return (
    <PageScaffold title="记忆" description="短小、可检索的信息片段">
      <EmptyState title="还没有记忆" body="阶段 3 将支持 Markdown 片段、标签与一键转任务。" />
    </PageScaffold>
  );
}

import { EmptyState, PageScaffold } from "@/components/PageScaffold";

export function ClipboardPage() {
  return (
    <PageScaffold title="剪切板" description="文本历史，本地保存">
      <EmptyState
        title="剪切板历史尚未启用"
        body="阶段 4 将接入安全采集、暂停记录与转任务/记忆。"
      />
    </PageScaffold>
  );
}

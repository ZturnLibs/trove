import { EmptyState, PageScaffold } from "@/components/PageScaffold";

export function TasksPage() {
  return (
    <PageScaffold title="任务" description="按清单浏览与管理">
      <EmptyState title="还没有任务" body="阶段 1 将实现创建、筛选、排序与详情编辑。" />
    </PageScaffold>
  );
}

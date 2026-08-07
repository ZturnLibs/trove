import { useCallback, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import type { PagedResult } from "@/ipc/client";
import { Button } from "@/design-system/primitives/Button";

export const DEFAULT_PAGE_SIZE = 200;

export function usePagedQuery<T>(
  queryKey: unknown[],
  fetchPage: (offset: number, limit: number) => Promise<PagedResult<T>>,
  pageSize = DEFAULT_PAGE_SIZE,
) {
  const [extraItems, setExtraItems] = useState<T[]>([]);
  const [meta, setMeta] = useState({ total: 0, hasMore: false });
  const [loadingMore, setLoadingMore] = useState(false);

  const firstPageQuery = useQuery({
    queryKey,
    queryFn: async () => {
      const page = await fetchPage(0, pageSize);
      setExtraItems([]);
      setMeta({ total: page.total, hasMore: page.hasMore });
      return page.items;
    },
  });

  const items = useMemo(
    () => [...(firstPageQuery.data ?? []), ...extraItems],
    [firstPageQuery.data, extraItems],
  );

  const loadMore = useCallback(async () => {
    if (!meta.hasMore || loadingMore) return;
    setLoadingMore(true);
    try {
      const offset = (firstPageQuery.data?.length ?? 0) + extraItems.length;
      const page = await fetchPage(offset, pageSize);
      setExtraItems((prev) => [...prev, ...page.items]);
      setMeta({ total: page.total, hasMore: page.hasMore });
    } finally {
      setLoadingMore(false);
    }
  }, [
    extraItems.length,
    fetchPage,
    firstPageQuery.data?.length,
    loadingMore,
    meta.hasMore,
    pageSize,
  ]);

  return {
    items,
    total: meta.total,
    hasMore: meta.hasMore,
    loading: firstPageQuery.isLoading,
    loadingMore,
    loadMore,
    error: firstPageQuery.error,
  };
}

export function PagedListFooter({
  shown,
  total,
  hasMore,
  loadingMore,
  onLoadMore,
}: {
  shown: number;
  total: number;
  hasMore: boolean;
  loadingMore: boolean;
  onLoadMore: () => void;
}) {
  if (total === 0) return null;
  return (
    <div className="flex flex-col items-center gap-2 border-t border-border py-3">
      <span className="text-[11px] text-muted">
        已显示 {shown} / 共 {total} 条
      </span>
      {hasMore ? (
        <Button
          size="sm"
          variant="secondary"
          disabled={loadingMore}
          onClick={() => void onLoadMore()}
        >
          {loadingMore ? "加载中…" : "加载更多"}
        </Button>
      ) : null}
    </div>
  );
}

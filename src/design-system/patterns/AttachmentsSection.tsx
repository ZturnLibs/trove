import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, ImagePlus, Paperclip, X } from "lucide-react";
import { Button } from "@/design-system/primitives/Button";
import { ipc, type ClipboardItem } from "@/ipc/client";
import { cn } from "@/lib/cn";

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function AttachmentsSection({
  entityType,
  entityId,
}: {
  entityType: "task" | "memory";
  entityId: string;
}) {
  const queryClient = useQueryClient();
  const [pickerOpen, setPickerOpen] = useState(false);
  const [removeArmedId, setRemoveArmedId] = useState<string | null>(null);
  const [preview, setPreview] = useState<{ src: string; loading: boolean } | null>(
    null,
  );
  const removeTimer = useRef<number | null>(null);

  // 图片预览：点击缩略图拉取原图 data URL；Esc / 点击遮罩 / ✕ 关闭。
  useEffect(() => {
    if (!preview) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setPreview(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [preview]);

  const openPreview = (assetId: string) => {
    setPreview({ src: "", loading: true });
    void ipc.assetReadData(assetId).then((dataUrl) => {
      setPreview(
        dataUrl ? { src: dataUrl, loading: false } : { src: "", loading: false },
      );
    });
  };


  const onRemoveClick = (linkId: string) => {
    if (removeArmedId === linkId) {
      if (removeTimer.current) window.clearTimeout(removeTimer.current);
      removeTimer.current = null;
      setRemoveArmedId(null);
      removeMutation.mutate(linkId);
    } else {
      setRemoveArmedId(linkId);
      if (removeTimer.current) window.clearTimeout(removeTimer.current);
      removeTimer.current = window.setTimeout(() => setRemoveArmedId(null), 3000);
    }
  };

  const linksQuery = useQuery({
    queryKey: ["links", entityType, entityId],
    queryFn: () => ipc.entityLinkAssets(entityType, entityId),
  });

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["links", entityType, entityId] });
    void queryClient.invalidateQueries({ queryKey: ["clipboard"] });
  };

  const removeMutation = useMutation({
    mutationFn: (linkId: string) => ipc.entityLinkRemove(linkId),
    onSuccess: invalidate,
  });

  const links = linksQuery.data ?? [];

  const attachMutation = useMutation({
    // 目标必须是资产 id（剪贴板条目 id 不是资产 id——传条目 id 会创建
    // 幽灵链接，查询时被 assets.get 跳过，附件永不显示）。
    mutationFn: async (assetIds: string[]) => {
      const linked = new Set(links.map((a) => a.assetId));
      const targets = assetIds.filter((id) => !linked.has(id));
      for (const targetId of targets) {
        await ipc.entityLinkCreate({
          sourceType: entityType,
          sourceId: entityId,
          targetType: "asset",
          targetId,
          linkKind: "attachment",
        });
      }
      return targets.length;
    },
    onSuccess: () => {
      setPickerOpen(false);
      invalidate();
    },
  });

  return (
    <section className="space-y-2 border-t border-border pt-3">
      <div className="flex items-center justify-between">
        <h3 className="flex items-center gap-1 text-[12px] font-medium">
          <Paperclip className="size-3.5" />
          附件
          {links.length > 0 ? <span className="text-muted">({links.length})</span> : null}
        </h3>
        <Button size="sm" variant="ghost" onClick={() => setPickerOpen(true)}>
          <ImagePlus className="size-3.5" />
          附加图片
        </Button>
      </div>

      {links.length === 0 ? (
        <p className="text-[11px] text-muted">
          暂无附件。可从剪切板图片历史附加图片。
        </p>
      ) : (
        <ul className="grid grid-cols-3 gap-2">
          {links.map((asset) => {
            const src = asset.thumbBase64
              ? `data:image/png;base64,${asset.thumbBase64}`
              : null;
            return (
              <li
                key={asset.linkId}
                className="group relative overflow-hidden rounded-[var(--radius-control)] border border-border bg-surface"
              >
                {src ? (
                  <img
                    src={src}
                    alt="附件缩略图（点击预览原图）"
                    title="点击预览原图"
                    className="aspect-square w-full cursor-zoom-in object-cover"
                    onClick={(event) => {
                      event.stopPropagation();
                      openPreview(asset.assetId);
                    }}
                  />
                ) : (
                  <div className="flex aspect-square w-full items-center justify-center text-[11px] text-muted">
                    无预览
                  </div>
                )}
                <div className="flex items-center justify-between gap-1 px-1 py-0.5 text-[10px] text-muted">
                  <span className="truncate">
                    {asset.width && asset.height
                      ? `${asset.width}×${asset.height}`
                      : formatBytes(asset.byteSize)}
                  </span>
                  <button
                    type="button"
                    title={
                      removeArmedId === asset.linkId
                        ? "再次点击确认移除附件"
                        : "移除附件"
                    }
                    className={cn(
                      "rounded p-0.5 text-muted hover:bg-row-hover",
                      removeArmedId === asset.linkId
                        ? "bg-danger/10 text-danger"
                        : "hover:text-danger",
                      "opacity-0 transition-opacity group-hover:opacity-100",
                    )}
                    onClick={() => onRemoveClick(asset.linkId)}
                  >
                    {removeArmedId === asset.linkId ? (
                      <span className="px-0.5 text-[10px]">确认移除</span>
                    ) : (
                      <X className="size-3.5" />
                    )}
                  </button>
                </div>
              </li>
            );
          })}
        </ul>
      )}

      {pickerOpen ? (
        <ImagePicker
          linkedAssetIds={links.map((a) => a.assetId)}
          busy={attachMutation.isPending}
          onAttach={(assetIds) => attachMutation.mutate(assetIds)}
          onClose={() => setPickerOpen(false)}
        />
      ) : null}

      {preview ? (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-8"
          onClick={() => setPreview(null)}
        >
          <button
            type="button"
            aria-label="关闭预览"
            className="absolute right-4 top-4 rounded p-1.5 text-white/80 hover:bg-white/10 hover:text-white"
            onClick={() => setPreview(null)}
          >
            <X className="size-5" />
          </button>
          {preview.loading ? (
            <p className="text-[13px] text-white/80">加载原图…</p>
          ) : preview.src ? (
            <img
              src={preview.src}
              alt="附件预览"
              className="max-h-[88vh] max-w-[92vw] rounded-[var(--radius-control)] object-contain shadow-2xl"
              onClick={(event) => event.stopPropagation()}
            />
          ) : (
            <p className="text-[13px] text-white/80">无法加载原图</p>
          )}
        </div>
      ) : null}
    </section>
  );
}

function ImagePicker({
  linkedAssetIds,
  busy,
  onAttach,
  onClose,
}: {
  linkedAssetIds: string[];
  busy: boolean;
  onAttach: (assetIds: string[]) => void;
  onClose: () => void;
}) {
  const [selected, setSelected] = useState<string[]>([]);

  const historyQuery = useQuery({
    queryKey: ["clipboard", "images"],
    queryFn: () => ipc.clipboardQuery({ kind: "image", limit: 60 }),
  });
  // 仅展示带资产 id 的图片（无资产则无法建立附件链接）。
  const items = (historyQuery.data?.items ?? []).filter(
    (item): item is ClipboardItem & { assetId: string } =>
      item.assetId !== null && !linkedAssetIds.includes(item.assetId),
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
      <button
        type="button"
        aria-label="关闭选择器"
        className="absolute inset-0 cursor-default bg-black/40"
        onClick={onClose}
      />
      <div
        className="relative max-h-[70vh] w-full max-w-lg overflow-auto rounded-[var(--radius-panel)] border border-border bg-surface p-4 shadow-xl"
        role="dialog"
        aria-modal="true"
        aria-label="从剪切板图片历史附加"
      >
        <div className="mb-3 flex items-center justify-between">
          <h4 className="text-[13px] font-medium">从剪切板图片历史附加</h4>
          <Button size="sm" variant="ghost" onClick={onClose}>
            关闭
          </Button>
        </div>
        {historyQuery.isLoading ? (
          <p className="p-4 text-center text-[12px] text-muted">加载中…</p>
        ) : items.length === 0 ? (
          <p className="p-4 text-center text-[12px] text-muted">
            暂无可附加的新图片。先复制一张图片试试。
          </p>
        ) : (
          <>
            <div className="grid grid-cols-4 gap-2">
              {items.map((item) => {
                const src = item.thumbBase64
                  ? `data:image/png;base64,${item.thumbBase64}`
                  : null;
                const picked = selected.includes(item.assetId);
                return (
                  <button
                    key={item.id}
                    type="button"
                    className={cn(
                      "relative aspect-square overflow-hidden rounded-[var(--radius-control)] border transition-colors",
                      picked
                        ? "border-accent ring-2 ring-accent/40"
                        : "border-border hover:border-accent",
                    )}
                    title={`${item.width ?? ""}×${item.height ?? ""}`}
                    onClick={() =>
                      setSelected((prev) =>
                        prev.includes(item.assetId)
                          ? prev.filter((id) => id !== item.assetId)
                          : [...prev, item.assetId],
                      )
                    }
                    aria-pressed={picked}
                  >
                    {src ? (
                      <img
                        src={src}
                        alt=""
                        className="h-full w-full object-cover"
                      />
                    ) : (
                      <span className="flex h-full w-full items-center justify-center text-[10px] text-muted">
                        图片
                      </span>
                    )}
                    {picked ? (
                      <span className="absolute right-1 top-1 flex size-4 items-center justify-center rounded-full bg-accent text-accent-fg">
                        <Check className="size-3" />
                      </span>
                    ) : null}
                  </button>
                );
              })}
            </div>
            <div className="mt-3 flex items-center justify-between gap-2">
              <span className="text-[11px] text-muted">
                已选 {selected.length} 张，可多选
              </span>
              <div className="flex gap-2">
                <Button size="sm" variant="ghost" onClick={onClose}>
                  取消
                </Button>
                <Button
                  size="sm"
                  disabled={selected.length === 0 || busy}
                  onClick={() => onAttach(selected)}
                >
                  {busy ? "附加中…" : `附加${selected.length > 0 ? `（${selected.length}）` : ""}`}
                </Button>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

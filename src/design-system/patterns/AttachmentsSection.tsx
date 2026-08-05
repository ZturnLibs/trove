import { useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ImagePlus, Paperclip, X } from "lucide-react";
import { Button } from "@/design-system/primitives/Button";
import { ipc, type LinkedAsset } from "@/ipc/client";
import { cn } from "@/lib/cn";

function thumbSrc(asset: LinkedAsset) {
  return asset.thumbBase64
    ? `data:image/png;base64,${asset.thumbBase64}`
    : null;
}

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
  const removeTimer = useRef<number | null>(null);

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

  const addMutation = useMutation({
    mutationFn: (assetId: string) =>
      ipc.entityLinkCreate({
        sourceType: entityType,
        sourceId: entityId,
        targetType: "asset",
        targetId: assetId,
        linkKind: "attachment",
      }),
    onSuccess: () => {
      setPickerOpen(false);
      invalidate();
    },
  });

  const assets = linksQuery.data ?? [];

  return (
    <section className="space-y-2 border-t border-border pt-3">
      <div className="flex items-center justify-between">
        <h3 className="flex items-center gap-1 text-[12px] font-medium">
          <Paperclip className="size-3.5" />
          附件
          {assets.length > 0 ? <span className="text-muted">({assets.length})</span> : null}
        </h3>
        <Button size="sm" variant="ghost" onClick={() => setPickerOpen(true)}>
          <ImagePlus className="size-3.5" />
          附加图片
        </Button>
      </div>

      {assets.length === 0 ? (
        <p className="text-[11px] text-muted">
          暂无附件。可从剪切板图片历史附加图片。
        </p>
      ) : (
        <ul className="grid grid-cols-3 gap-2">
          {assets.map((asset) => {
            const src = thumbSrc(asset);
            return (
              <li
                key={asset.linkId}
                className="group relative overflow-hidden rounded-[var(--radius-control)] border border-border bg-surface"
              >
                {src ? (
                  <img
                    src={src}
                    alt="附件缩略图"
                    className="aspect-square w-full object-cover"
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
          onPick={(assetId) => addMutation.mutate(assetId)}
          onClose={() => setPickerOpen(false)}
        />
      ) : null}
    </section>
  );
}

function ImagePicker({
  onPick,
  onClose,
}: {
  onPick: (assetId: string) => void;
  onClose: () => void;
}) {
  const historyQuery = useQuery({
    queryKey: ["clipboard", "images"],
    queryFn: () => ipc.clipboardQuery({ kind: "image", limit: 60 }),
  });
  const items = historyQuery.data ?? [];

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
            暂无图片历史。先复制一张图片试试。
          </p>
        ) : (
          <div className="grid grid-cols-4 gap-2">
            {items.map((item) => {
              const src = item.thumbBase64
                ? `data:image/png;base64,${item.thumbBase64}`
                : null;
              return (
                <button
                  key={item.id}
                  type="button"
                  className="aspect-square overflow-hidden rounded-[var(--radius-control)] border border-border hover:border-accent"
                  title={`${item.width ?? ""}×${item.height ?? ""}`}
                  onClick={() => onPick(item.id)}
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
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FileText, FolderOpen, Link2 } from "lucide-react";
import { Button } from "@/design-system/primitives/Button";
import { ipc } from "@/ipc/client";
import { cn } from "@/lib/cn";

function formatBytes(bytes: number | null) {
  if (bytes == null) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function FileRefsSection({
  entityType,
  entityId,
}: {
  entityType: "task" | "memory";
  entityId: string;
}) {
  const queryClient = useQueryClient();
  const listQuery = useQuery({
    queryKey: ["file-refs", entityType, entityId],
    queryFn: () => ipc.fileRefListForEntity(entityType, entityId),
  });

  const invalidate = () => {
    void queryClient.invalidateQueries({
      queryKey: ["file-refs", entityType, entityId],
    });
  };

  const attachMutation = useMutation({
    mutationFn: () => ipc.fileRefPickAndAttach(entityType, entityId),
    onSuccess: invalidate,
  });

  const relinkMutation = useMutation({
    mutationFn: (id: string) => ipc.fileRefRelink(id),
    onSuccess: invalidate,
  });

  const files = listQuery.data ?? [];

  return (
    <section className="space-y-2 border-t border-border pt-3">
      <div className="flex items-center justify-between">
        <h3 className="flex items-center gap-1 text-[12px] font-medium">
          <FileText className="size-3.5" />
          文件引用
          {files.length > 0 ? (
            <span className="text-muted">({files.length})</span>
          ) : null}
        </h3>
        <Button
          size="sm"
          variant="ghost"
          disabled={attachMutation.isPending}
          onClick={() => attachMutation.mutate()}
        >
          添加文件
        </Button>
      </div>
      {files.length === 0 ? (
        <p className="text-[11px] text-muted">
          保存本地文件引用（不复制本体）；删除引用不会删除原文件。
        </p>
      ) : (
        <ul className="space-y-1">
          {files.map(({ linkId, file }) => (
            <li
              key={linkId}
              className="flex items-center gap-2 rounded-[var(--radius-control)] border border-border px-2 py-1.5 text-[12px]"
            >
              <div className="min-w-0 flex-1">
                <div className="truncate font-medium">{file.displayName}</div>
                <div
                  className={cn(
                    "truncate text-[10px]",
                    file.accessible ? "text-muted" : "text-danger",
                  )}
                >
                  {file.accessible ? file.pathHint : "文件不可访问"}
                  {file.byteSize != null ? ` · ${formatBytes(file.byteSize)}` : ""}
                </div>
              </div>
              <Button
                size="sm"
                variant="ghost"
                title="打开"
                disabled={!file.accessible}
                onClick={() => void ipc.fileRefOpen(file.id)}
              >
                打开
              </Button>
              <Button
                size="sm"
                variant="ghost"
                title="在 Finder 中显示"
                disabled={!file.accessible}
                onClick={() => void ipc.fileRefReveal(file.id)}
              >
                <FolderOpen className="size-3.5" />
              </Button>
              {!file.accessible ? (
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={relinkMutation.isPending}
                  onClick={() => relinkMutation.mutate(file.id)}
                >
                  <Link2 className="size-3.5" />
                  重新定位
                </Button>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { Button } from "@/design-system/primitives/Button";
import { ipc, type CsvPreview } from "@/ipc/client";

type Props = {
  onMessage: (msg: string) => void;
};

export function CsvTasksSection({ onMessage }: Props) {
  const queryClient = useQueryClient();
  const fileRef = useRef<HTMLInputElement>(null);
  const [preview, setPreview] = useState<CsvPreview | null>(null);
  const [csvText, setCsvText] = useState<string | null>(null);
  const [fileName, setFileName] = useState<string | null>(null);
  const [skipDuplicates, setSkipDuplicates] = useState(true);

  const batchesQuery = useQuery({
    queryKey: ["csv", "batches"],
    queryFn: () => ipc.csvImportBatches(),
  });

  const exportCsv = useMutation({
    mutationFn: () => ipc.csvExportTasks(),
    onSuccess: (text) => {
      const blob = new Blob([text], { type: "text/csv;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `trove-tasks-${new Date().toISOString().slice(0, 10)}.csv`;
      a.click();
      URL.revokeObjectURL(url);
      onMessage("已导出任务 CSV");
    },
    onError: (err) =>
      onMessage(err instanceof Error ? err.message : "导出 CSV 失败"),
  });

  const runPreview = useMutation({
    mutationFn: (text: string) => ipc.csvPreviewTasks(text),
    onSuccess: (data) => setPreview(data),
    onError: (err) =>
      onMessage(err instanceof Error ? err.message : "预览失败"),
  });

  const importCsv = useMutation({
    mutationFn: () => {
      if (!csvText) throw new Error("没有待导入文件");
      return ipc.csvImportTasks({
        csv: csvText,
        skipDuplicates,
        mapping: preview?.mapping,
      });
    },
    onSuccess: (result) => {
      void queryClient.invalidateQueries({ queryKey: ["csv"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      setPreview(null);
      setCsvText(null);
      setFileName(null);
      onMessage(`已导入 ${result.created} 条任务（跳过 ${result.skipped}）`);
    },
    onError: (err) =>
      onMessage(err instanceof Error ? err.message : "导入失败"),
  });

  const undoBatch = useMutation({
    mutationFn: (id: string) => ipc.csvUndoImport(id),
    onSuccess: (result) => {
      void queryClient.invalidateQueries({ queryKey: ["csv"] });
      void queryClient.invalidateQueries({ queryKey: ["tasks"] });
      onMessage(`已撤销导入：删除 ${result.deleted} 条，保留 ${result.kept} 条（已修改）`);
    },
    onError: (err) =>
      onMessage(err instanceof Error ? err.message : "撤销失败"),
  });

  const canImport =
    Boolean(csvText) &&
    preview != null &&
    preview.errorCount === 0 &&
    (preview.duplicateCount === 0 || skipDuplicates) &&
    !importCsv.isPending;

  return (
    <div className="mt-4 space-y-3 border-t border-border pt-3 text-[12px]">
      <h3 className="font-medium">任务 CSV</h3>
      <p className="text-muted">
        用于从其他待办工具迁移任务。不含周期规则与提醒；完整备份请用上方 JSON。
      </p>
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          variant="secondary"
          onClick={() => exportCsv.mutate()}
          disabled={exportCsv.isPending}
        >
          导出任务 CSV…
        </Button>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => fileRef.current?.click()}
          disabled={runPreview.isPending}
        >
          预览导入 CSV…
        </Button>
        <input
          ref={fileRef}
          type="file"
          accept="text/csv,.csv"
          className="hidden"
          onChange={(e) => {
            const file = e.target.files?.[0];
            e.target.value = "";
            if (!file) return;
            void file.text().then((text) => {
              setFileName(file.name);
              setCsvText(text);
              runPreview.mutate(text);
            });
          }}
        />
      </div>

      {preview && csvText ? (
        <div className="space-y-2 rounded border border-border p-3">
          <p>
            {fileName}：{preview.rowCount} 行 · 可导入 {preview.validCount} ·
            重复 {preview.duplicateCount} · 错误 {preview.errorCount}
          </p>
          <p className="text-muted">
            映射：标题={preview.mapping.title ?? "未识别"}
            {preview.mapping.dueDate ? ` · 截止日期=${preview.mapping.dueDate}` : ""}
            {preview.mapping.list ? ` · 清单=${preview.mapping.list}` : ""}
          </p>
          {preview.unmappedLists.length > 0 ? (
            <p className="text-muted">
              未匹配清单将进入收件箱：{preview.unmappedLists.join("、")}
            </p>
          ) : null}
          {preview.errors.length > 0 ? (
            <ul className="text-danger">
              {preview.errors.map((item) => (
                <li key={`${item.row}-${item.message}`}>
                  第 {item.row} 行：{item.message}
                </li>
              ))}
            </ul>
          ) : null}
          {preview.duplicates.length > 0 ? (
            <ul className="text-muted">
              {preview.duplicates.map((item) => (
                <li key={`${item.row}-dup`}>
                  第 {item.row} 行重复：{item.title}
                </li>
              ))}
            </ul>
          ) : null}
          {preview.sample.length > 0 ? (
            <ul className="text-muted">
              {preview.sample.map((row, i) => (
                <li key={`${row.title}-${i}`}>
                  {row.title}
                  {row.dueDate ? ` · ${row.dueDate}` : ""}
                  {row.duplicate ? " · 重复" : ""}
                </li>
              ))}
            </ul>
          ) : null}
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={skipDuplicates}
              onChange={(e) => setSkipDuplicates(e.target.checked)}
            />
            跳过重复任务
          </label>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" onClick={() => importCsv.mutate()} disabled={!canImport}>
              确认导入
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => {
                setPreview(null);
                setCsvText(null);
                setFileName(null);
              }}
            >
              取消
            </Button>
          </div>
        </div>
      ) : null}

      {(batchesQuery.data ?? []).filter((b) => b.status === "applied").length > 0 ? (
        <ul className="space-y-1">
          {(batchesQuery.data ?? [])
            .filter((b) => b.status === "applied")
            .map((batch) => (
              <li key={batch.id} className="flex items-center justify-between gap-2">
                <span className="text-muted">
                  {batch.createdAt} · 导入 {batch.created} 条
                </span>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => undoBatch.mutate(batch.id)}
                  disabled={undoBatch.isPending}
                >
                  撤销
                </Button>
              </li>
            ))}
        </ul>
      ) : null}
    </div>
  );
}

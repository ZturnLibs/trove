import { useQuery } from "@tanstack/react-query";
import { Button } from "@/design-system/primitives/Button";
import { BrandLogo } from "@/components/BrandLogo";
import { ipc } from "@/ipc/client";

export function AboutDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const healthQuery = useQuery({
    queryKey: ["app", "health"],
    queryFn: () => ipc.appHealth(),
  });

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
      <button
        type="button"
        aria-label="关闭关于"
        className="absolute inset-0 cursor-default bg-black/40"
        onClick={onClose}
      />
      <div
        className="relative w-72 rounded-[var(--radius-panel)] border border-border bg-surface p-6 text-center shadow-lg"
        role="dialog"
        aria-modal="true"
        aria-label="关于 Trove"
      >
        <BrandLogo variant="brand" className="mx-auto h-16 w-16" />
        <h2 className="mt-3 text-[16px] font-semibold">Trove</h2>
        <p className="mt-1 text-[12px] text-muted">
          版本 {healthQuery.data?.appVersion ?? "…"}
        </p>
        <p className="mt-2 text-[12px] text-foreground">本地优先的个人工作台</p>
        <p className="mt-1 text-[11px] text-muted">© 2026 Trove</p>
        <Button size="sm" className="mt-4" onClick={onClose}>
          关闭
        </Button>
      </div>
    </div>
  );
}

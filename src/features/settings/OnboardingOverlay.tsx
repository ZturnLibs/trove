import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/design-system/primitives/Button";
import { ipc } from "@/ipc/client";

export function OnboardingOverlay() {
  const queryClient = useQueryClient();
  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: () => ipc.settingsGet(),
  });

  const complete = useMutation({
    mutationFn: async () => {
      const settings = await ipc.settingsGet();
      return ipc.settingsSave({ ...settings, onboardingCompleted: true });
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });

  if (!settingsQuery.data || settingsQuery.data.onboardingCompleted) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-6">
      <div className="max-w-lg rounded-[var(--radius-panel)] border border-border bg-surface p-5 shadow-lg">
        <h2 className="text-[16px] font-semibold">欢迎使用 Trove</h2>
        <p className="mt-2 text-[13px] text-muted">
          这是一个本地优先的个人工作台。任务、提醒、记忆和剪切板都保存在本机，可完全离线使用。
        </p>
        <ul className="mt-4 space-y-2 text-[12px] text-foreground">
          <li>· 关闭主窗口会隐藏到托盘，不会退出；提醒与剪切板采集继续运行。</li>
          <li>· 通知权限用于到期提醒；拒绝后提醒仍会出现在「今日」，但不弹系统通知。</li>
          <li>· 剪切板仅记录文本，可随时暂停；直接粘贴需要辅助功能权限，否则请用「再次复制」。</li>
          <li>· 请定期导出或依赖启动自动备份，保护你的个人数据。</li>
        </ul>
        <div className="mt-5 flex justify-end gap-2">
          <Button
            size="sm"
            variant="secondary"
            onClick={() => {
              void ipc.windowShowQuick("capture");
            }}
          >
            先试试快速记录
          </Button>
          <Button
            size="sm"
            onClick={() => complete.mutate()}
            disabled={complete.isPending}
          >
            开始使用
          </Button>
        </div>
      </div>
    </div>
  );
}

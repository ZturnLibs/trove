import type { ClipboardKindHint } from "@/ipc/client";

export const KIND_HINT_LABEL: Record<ClipboardKindHint, string> = {
  plain: "文本",
  url: "链接",
  email: "邮箱",
  phone: "电话",
  date: "含日期",
  code: "代码",
  error: "报错",
};

export type SmartAction = "memory" | "task" | "copy" | "link";

export function actionsForKindHint(kindHint: ClipboardKindHint): SmartAction[] {
  switch (kindHint) {
    case "phone":
      return ["task", "copy"];
    case "code":
      return ["memory", "copy"];
    case "error":
      return ["memory", "task", "copy"];
    case "date":
      return ["task", "memory", "copy"];
    default:
      return ["memory", "task", "copy"];
  }
}

export const ACTION_LABEL: Record<SmartAction, string> = {
  memory: "存记忆",
  task: "新建任务",
  copy: "复制",
  link: "关联",
};

/**
 * 周回顾出牌决策（纯函数）：出牌前校验主实例最新任务状态。
 * 任务可能已被其他窗口完结/归档——非 todo 一律自动跳过并提示，
 * 不执行动作（ProcessingSession.handleAction 的可测内核）。
 */

export type CardActionDecision =
  | { proceed: true }
  | { proceed: false; notice: string };

export function resolveCardAction(latest: {
  status: string;
  title: string;
}): CardActionDecision {
  if (latest.status !== "todo") {
    return {
      proceed: false,
      notice: `「${latest.title}」已不在待办状态，已自动跳过`,
    };
  }
  return { proceed: true };
}

import type { ReactNode } from "react";
import { useDroppable } from "@dnd-kit/core";
import { cn } from "@/lib/cn";

export function FocusDropZone({
  id,
  children,
  className,
}: {
  id: string;
  children: ReactNode;
  className?: string;
}) {
  const { setNodeRef, isOver } = useDroppable({ id });
  return (
    <div
      ref={setNodeRef}
      className={cn(
        "min-h-[2.25rem] transition-colors",
        isOver && "bg-row-hover/60",
        className,
      )}
    >
      {children}
    </div>
  );
}

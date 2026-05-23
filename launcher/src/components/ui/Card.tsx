import { type HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export function Card({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "rounded-2xl bg-[var(--color-bg-surface)]/80 backdrop-blur-xl",
        "ring-1 ring-inset ring-[var(--color-border-subtle)]",
        "shadow-2xl shadow-black/40",
        className,
      )}
      {...props}
    />
  );
}

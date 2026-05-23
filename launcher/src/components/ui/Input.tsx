import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  invalid?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { invalid, className, ...props },
  ref,
) {
  return (
    <input
      ref={ref}
      className={cn(
        "h-11 w-full rounded-lg px-4 text-sm",
        "bg-[var(--color-bg-elevated)] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)]",
        "ring-1 ring-inset",
        invalid
          ? "ring-[var(--color-danger-500)] focus:ring-[var(--color-danger-500)]"
          : "ring-[var(--color-border-subtle)] focus:ring-[var(--color-accent-500)]",
        "focus:outline-none focus:ring-2",
        "transition-shadow duration-150",
        className,
      )}
      {...props}
    />
  );
});

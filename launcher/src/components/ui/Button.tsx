import { forwardRef, type ButtonHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const VARIANT_CLASSES: Record<Variant, string> = {
  primary:
    "bg-[var(--color-accent-600)] hover:bg-[var(--color-accent-500)] active:bg-[var(--color-accent-700)] text-white shadow-lg shadow-indigo-900/30 ring-1 ring-inset ring-white/10",
  secondary:
    "bg-[var(--color-bg-elevated)] hover:bg-[var(--color-bg-overlay)] text-[var(--color-text-primary)] ring-1 ring-inset ring-[var(--color-border-subtle)]",
  ghost:
    "bg-transparent hover:bg-white/5 text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]",
  danger:
    "bg-[var(--color-danger-500)] hover:bg-red-500 text-white ring-1 ring-inset ring-white/10",
};

const SIZE_CLASSES: Record<Size, string> = {
  sm: "h-8 px-3 text-xs",
  md: "h-10 px-4 text-sm",
  lg: "h-12 px-6 text-base",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "primary", size = "md", className, ...props },
  ref,
) {
  return (
    <button
      ref={ref}
      className={cn(
        "inline-flex items-center justify-center gap-2 rounded-lg font-medium tracking-tight",
        "transition-[background-color,transform,box-shadow] duration-150",
        "active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50 disabled:active:scale-100",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent-500)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--color-bg-base)]",
        VARIANT_CLASSES[variant],
        SIZE_CLASSES[size],
        className,
      )}
      {...props}
    />
  );
});

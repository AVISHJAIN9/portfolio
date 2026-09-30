export const tokens = {
  colors: {
    bg: "#090d16",
    surface: "#0f172a",
    surfaceHover: "#1e293b",
    border: "#334155",
    borderFocus: "#38bdf8",
    accent: "#38bdf8",
    accentGlow: "rgba(56, 189, 248, 0.15)",
    success: "#10b981",
    warning: "#f59e0b",
    error: "#ef4444",
    textPrimary: "#f8fafc",
    textSecondary: "#94a3b8",
    textMuted: "#64748b",
  },
  fonts: {
    mono: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
    sans: "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
  },
  radii: {
    sm: "0.25rem",
    md: "0.5rem",
    lg: "0.75rem",
    full: "9999px",
  },
  glass: {
    backdrop: "blur(12px)",
    bg: "rgba(15, 23, 42, 0.75)",
    border: "1px solid rgba(255, 255, 255, 0.1)",
  }
};

export function cn(...classes: (string | undefined | null | false)[]): string {
  return classes.filter(Boolean).join(" ");
}

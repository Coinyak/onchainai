import type { Config } from "tailwindcss";

const config: Config = {
  theme: {
    extend: {
      colors: {
        primary: "#1A1A1A",
        secondary: "#6B6B6B",
        tertiary: "#E76F00",
        "neutral-bg": "#FFFFFF",
        "neutral-surface": "#F5F5F0",
        "neutral-hover": "#FAFAFA",
        border: "#E5E5E5",
        "border-strong": "#D1D1D1",
        "text-muted": "#999999",
        error: "#C0392B",
        success: "#2D7D46",
        "on-tertiary": "#FFFFFF",
      },
      borderRadius: {
        sm: "6px",
        md: "8px",
        lg: "12px",
        full: "9999px",
      },
      spacing: {
        xs: "4px",
        sm: "8px",
        md: "16px",
        lg: "24px",
        xl: "32px",
        "2xl": "48px",
        gutter: "16px",
        margin: "24px",
      },
      fontSize: {
        h1: ["28px", { lineHeight: "1.2", fontWeight: "700", letterSpacing: "-0.02em" }],
        h2: ["20px", { lineHeight: "1.3", fontWeight: "600", letterSpacing: "-0.01em" }],
        h3: ["16px", { lineHeight: "1.4", fontWeight: "600" }],
        "body-md": ["14px", { lineHeight: "1.6", fontWeight: "400" }],
        "body-sm": ["13px", { lineHeight: "1.5", fontWeight: "400" }],
        "label-caps": ["11px", { lineHeight: "1", fontWeight: "600", letterSpacing: "0.06em" }],
        code: ["13px", { lineHeight: "1.5", fontWeight: "400" }],
        "mobile-body": ["16px", { lineHeight: "1.65", fontWeight: "400" }],
      },
      fontFamily: {
        sans: ["var(--font-inter)", "Inter", "system-ui", "sans-serif"],
        mono: ["var(--font-jetbrains-mono)", "JetBrains Mono", "monospace"],
      },
      minHeight: {
        touch: "44px",
      },
      minWidth: {
        touch: "44px",
      },
    },
  },
};

export default config;
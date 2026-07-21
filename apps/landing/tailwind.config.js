/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    // Deliberately narrow scale — 4/8/12/16/24/32/48/64/96/128 plus the
    // handful of in-between values the layout actually needs. Nothing else.
    spacing: {
      0: "0px",
      1: "4px",
      2: "8px",
      3: "12px",
      4: "16px",
      5: "20px",
      6: "24px",
      8: "32px",
      10: "40px",
      12: "48px",
      16: "64px",
      20: "80px",
      24: "96px",
      32: "128px",
    },
    borderRadius: {
      none: "0px",
      sm: "6px",
      DEFAULT: "8px",
      md: "10px",
      lg: "12px",
      full: "9999px", // circles/avatars only — never buttons or cards
    },
    boxShadow: {
      // One elevation step. Nothing dramatic, nothing colored.
      sm: "0 1px 2px rgba(0,0,0,0.24)",
      DEFAULT: "0 2px 8px rgba(0,0,0,0.28)",
      none: "none",
    },
    fontFamily: {
      sans: ["Geist", "system-ui", "-apple-system", "Segoe UI", "Roboto", "sans-serif"],
      mono: ["Geist Mono", "ui-monospace", "SFMono-Regular", "SF Mono", "Menlo", "monospace"],
    },
    fontSize: {
      xs: ["12px", { lineHeight: "18px" }],
      sm: ["14px", { lineHeight: "21px" }],
      base: ["16px", { lineHeight: "26px" }],
      lg: ["18px", { lineHeight: "28px" }],
      xl: ["21px", { lineHeight: "30px" }],
      "2xl": ["26px", { lineHeight: "33px", letterSpacing: "-0.01em" }],
      "3xl": ["34px", { lineHeight: "40px", letterSpacing: "-0.02em" }],
      "4xl": ["48px", { lineHeight: "52px", letterSpacing: "-0.02em" }],
    },
    extend: {
      maxWidth: {
        content: "1160px",
      },
      colors: {
        // The palette already in production use across the dashboard, docs,
        // and CLI — reused here rather than inventing a landing-only look.
        bg: "#0a0a0d",
        surface: "#121218",
        "surface-2": "#17171f",
        border: "#232330",
        text: "#eeeef2",
        "text-dim": "#8f8fa3",
        brand: "#7c7bfb",
        "brand-dim": "#6260e0",
        good: "#3fb950",
        warn: "#d29922",
        bad: "#f85149",
      },
    },
  },
  plugins: [],
};

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
      xl: "16px",
      full: "9999px", // circles/avatars only — never buttons or cards
    },
    boxShadow: {
      // Soft, neutral light-mode elevation. No colored/glow shadows anywhere.
      sm: "0 1px 2px rgba(0,0,0,0.07)",
      DEFAULT: "0 4px 16px rgba(0,0,0,0.09)",
      lg: "0 12px 32px rgba(0,0,0,0.12)",
      none: "none",
    },
    fontFamily: {
      // Body copy, UI chrome.
      sans: ["Geist", "system-ui", "-apple-system", "Segoe UI", "Roboto", "sans-serif"],
      // Display/heading font — a tall, warm serif used for h1/h2-scale text only.
      serif: ["Instrument Serif", "Georgia", "serif"],
      mono: ["Geist Mono", "ui-monospace", "SFMono-Regular", "SF Mono", "Menlo", "monospace"],
    },
    fontSize: {
      xs: ["12px", { lineHeight: "18px" }],
      sm: ["14px", { lineHeight: "21px" }],
      base: ["16px", { lineHeight: "26px" }],
      lg: ["18px", { lineHeight: "28px" }],
      xl: ["22px", { lineHeight: "32px" }],
      "2xl": ["30px", { lineHeight: "38px", letterSpacing: "-0.01em" }],
      "3xl": ["42px", { lineHeight: "48px", letterSpacing: "-0.01em" }],
      "4xl": ["58px", { lineHeight: "62px", letterSpacing: "-0.01em" }],
      "5xl": ["76px", { lineHeight: "80px", letterSpacing: "-0.01em" }],
    },
    extend: {
      maxWidth: {
        content: "1160px",
      },
      colors: {
        // Black, white, and one signature blue — pushed toward true black
        // rather than washed-out grey for text, borders, and surfaces.
        // Red/green/yellow exist only where they carry real meaning (run
        // status) — used sparingly, never decoratively. Light mode default.
        bg: "#ffffff",
        surface: "#f4f4f5",
        "surface-2": "#e8e8ea",
        border: "#d8d8db",
        "border-strong": "#b8b8bd",
        text: "#000000",
        "text-dim": "#2e2e33",
        "text-dimmer": "#5c5c63",
        brand: "#2f6fed",
        "brand-dim": "#1f57c9",
        "brand-soft": "#eaf1ff",
        good: "#16a34a",
        "good-soft": "#e9f9ef",
        warn: "#d97706",
        "warn-soft": "#fef3e2",
        bad: "#dc2626",
        "bad-soft": "#fdecec",
      },
    },
  },
  plugins: [],
};

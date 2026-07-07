import type { Config } from "tailwindcss";
import nestPreset from "./nest-tailwind-preset.json";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  presets: [nestPreset as Config],
  theme: {
    extend: {},
  },
  plugins: [],
} satisfies Config;

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// GitHub project-pages deploy under /<repo>/ — CI gets the prefixed base,
// local dev stays at "/"
export default defineConfig({
  plugins: [react()],
  base: process.env.CI ? "/LLM-Lang/" : "/",
});

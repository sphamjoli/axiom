import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  // Emit every asset as a file, never a data: URI, so the Content-Security-Policy
  // in index.html can load fonts from 'self' alone.
  build: { assetsInlineLimit: 0 },
  // The simulation worker is an ES module, so it can import the engine's
  // WebAssembly glue and load the module from 'self'.
  worker: { format: "es" },
});

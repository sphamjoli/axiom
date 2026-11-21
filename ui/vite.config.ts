import { defineConfig, type Plugin } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

const CONTENT_SECURITY_POLICY =
  "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; style-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'";

// The dev server injects styles inline for hot reload, which 'style-src self'
// blocks, so the policy is added to the built page only.
function contentSecurityPolicy(): Plugin {
  return {
    name: "axiom-content-security-policy",
    apply: "build",
    transformIndexHtml: () => [
      {
        tag: "meta",
        attrs: { "http-equiv": "Content-Security-Policy", content: CONTENT_SECURITY_POLICY },
        injectTo: "head-prepend",
      },
    ],
  };
}

export default defineConfig({
  // GitHub Pages serves the site under /<repository>/; the deploy workflow sets this.
  base: process.env.AXIOM_BASE ?? "/",
  plugins: [vue(), tailwindcss(), contentSecurityPolicy()],
  // Emit every asset as a file, never a data: URI, so the Content-Security-Policy
  // can load fonts from 'self' alone.
  build: { assetsInlineLimit: 0 },
  // The simulation worker is an ES module, so it can import the engine's
  // WebAssembly glue and load the module from 'self'.
  worker: { format: "es" },
});

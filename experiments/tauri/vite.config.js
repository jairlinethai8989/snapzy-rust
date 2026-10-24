import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

export default defineConfig({
  root: "ui",
  clearScreen: false,
  server: { host: "127.0.0.1", port: 51819, strictPort: true },
  build: {
    outDir: "../web-dist",
    emptyOutDir: true,
    target: "es2022",
    rollupOptions: {
      input: {
        launcher: fileURLToPath(new URL("./ui/index.html", import.meta.url)),
        editor: fileURLToPath(new URL("./ui/editor.html", import.meta.url)),
      },
    },
  },
});

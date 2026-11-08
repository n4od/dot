import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Tauri expects a fixed port and does not want Vite to clear the terminal,
// otherwise the Rust compiler output disappears.
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1421,
    strictPort: true,
    host: '127.0.0.1',
    // `bloub/` is a vendored reference copy, not part of the app: watching it
    // only makes the watcher crawl 80-odd unrelated files.
    watch: { ignored: ['**/src-tauri/**', '**/bloub/**'] }
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    // WebView2 on Windows 10/11 is evergreen Chromium.
    target: 'chrome105',
    sourcemap: true
  }
})
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Split the heavy, independent subsystems into their own chunks so the parser,
// renderer, and exporters aren't all forced into the main entry chunk.
export default defineConfig({
  plugins: [vue()],
  build: {
    chunkSizeWarningLimit: 700,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('/src/services/openscadParser')) return 'parser'
          if (id.includes('/src/services/webgpuRenderer')) return 'renderer'
          if (
            id.includes('/src/services/stlExport') ||
            id.includes('/src/services/objExport') ||
            id.includes('/src/services/threemfExport') ||
            id.includes('/src/services/zipExport') ||
            id.includes('/src/services/stlImport')
          ) {
            return 'exporters'
          }
          if (id.includes('node_modules/vue') || id.includes('node_modules/@vue')) {
            return 'vue'
          }
        },
      },
    },
  },
})

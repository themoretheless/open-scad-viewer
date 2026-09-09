import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Split the heavy, independent subsystems into their own chunks so the parser,
// renderer, and exporters aren't all forced into the main entry chunk.
export default defineConfig({
  plugins: [vue()],
  worker: { format: 'es', rollupOptions: { output: { manualChunks(id) {
    if (id.includes('/src/generated/geometry-kernels/bytes')) return 'geometry-kernel-bytes'
  } } } },
  build: {
    chunkSizeWarningLimit: 700,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('/src/generated/geometry-kernels/bytes')) return 'geometry-kernel-bytes'
          // Shared by the entry graph (photogrammetry loader) and lazy language
          // chunks; without its own chunk it drags the geometry kernel into
          // the entry preload list.
          if (id.endsWith('/src/services/wasmPacking.ts') || id.endsWith('/src/services/valueBinaryCodec.ts')) return 'binary-codec'
          // The entry graph needs only this regex; keep it out of the lazy
          // compiler chunk so the geometry kernel is not preloaded.
          if (id.endsWith('/src/services/modelGraphTextDetect.ts')) return 'modelgraph-text-detect'
          if (id.endsWith('/src/services/modelGraphText.ts')) return 'modelgraph-text'
          if (id.includes('/src/services/meshSurfaceGroups')) return 'surface-selection'
          if (id.includes('/src/services/openscadParser') || id.includes('/src/parser/')) return 'parser'
          if (id.includes('/src/services/webgpuRenderer') || id.includes('/src/renderer/')) return 'renderer'
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

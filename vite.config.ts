import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Split the heavy, independent subsystems into their own chunks so the parser,
// renderer, and exporters aren't all forced into the main entry chunk.
// Native rolldown codeSplitting groups: matched modules are captured without
// their dependencies, so lazy groups never drag the geometry kernel into the
// entry preload list.
export default defineConfig({
  plugins: [vue()],
  worker: { format: 'es', rollupOptions: { output: { manualChunks(id) {
    if (id.includes('/src/generated/geometry-kernels/bytes')) return 'geometry-kernel-bytes'
    if (id.includes('/src/generated/harfbuzz/bytes')) return 'harfbuzz-bytes'
    if (id.includes('/src/generated/photogrammetry/bytes')) return 'photogrammetry-bytes'
    if (id.includes('/src/generated/wasm-brotli/bytes')) return 'wasm-brotli-bytes'
  } } } },
  build: {
    chunkSizeWarningLimit: 700,
    rollupOptions: {
      output: {
        codeSplitting: {
          groups: [
            { name: 'geometry-kernel-bytes', test: /src[\\/]generated[\\/]geometry-kernels[\\/]bytes/ },
            { name: 'harfbuzz-bytes', test: /src[\\/]generated[\\/]harfbuzz[\\/]bytes/ },
            { name: 'photogrammetry-bytes', test: /src[\\/]generated[\\/]photogrammetry[\\/]bytes/ },
            { name: 'wasm-brotli-bytes', test: /src[\\/]generated[\\/]wasm-brotli[\\/]bytes/ },
            // Shared by the entry graph (photogrammetry loader) and lazy language
            // chunks; without its own chunk it drags the geometry kernel into
            // the entry preload list.
            { name: 'binary-codec', test: /(src[\\/]core[\\/]sha256\.ts|src[\\/]services[\\/](wasmPacking|valueBinaryCodec)\.ts)$/ },
            // The entry graph needs only this regex; keep it out of the lazy
            // compiler chunk so the geometry kernel is not preloaded.
            { name: 'detect', test: /src[\\/]services[\\/]modelGraphTextDetect\.ts$/ },
            { name: 'modelgraph-text', test: /src[\\/]services[\\/]modelGraphText\.ts$/ },
            { name: 'directBodies', test: /src[\\/]services[\\/]directBodiesScad\.ts$/ },
            {
              // Direct-modeling helpers are shared by lazy CAD panels; they must
              // not capture the geometry kernel they reference.
              name: 'direct-modeling',
              test: /src[\\/]services[\\/]direct(Modeling|SolidTools|ProfileTools|SketchGeometry|ModelingTools)\.ts$/,
              includeDependenciesRecursively: false,
            },
            { name: 'surface-selection', test: /src[\\/]services[\\/]meshSurfaceGroups/ },
            { name: 'parser', test: /src[\\/]services[\\/]openscadParser|src[\\/]parser[\\/]/ },
            { name: 'renderer', test: /src[\\/]services[\\/]webgpuRenderer|src[\\/]renderer[\\/]/ },
            { name: 'exporters', test: /src[\\/]services[\\/](stlExport|objExport|threemfExport|zipExport|stlImport)/ },
            { name: 'vue', test: /node_modules[\\/](@vue|vue)/ },
          ],
        },
      },
    },
  },
})

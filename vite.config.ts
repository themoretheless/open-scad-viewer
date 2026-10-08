import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import {compactWgslPlugin} from './scripts/compact-wgsl.mjs'

// Split the heavy, independent subsystems into their own chunks so the parser,
// renderer, and exporters aren't all forced into the main entry chunk.
// Native rolldown codeSplitting groups: matched modules are captured without
// their dependencies, so lazy groups never drag the geometry kernel into the
// entry preload list.
export default defineConfig({
  define: { __G1_SHARED_GEOMETRY_MODULE__: 'false' },
  plugins: [compactWgslPlugin(),vue()],
  worker: { format: 'es', rollupOptions: { output: { manualChunks(id) {
    // Share decoder code on disk; each worker still owns its runtime state.
    if (/src[\\/]services[\\/](wasmPacking|wasmBase91|wasmBrotliPacking)\.ts$/.test(id)) return 'worker-wasm-packing'
    if (id.includes('/src/generated/geometry-kernels/bytes')) return 'geometry-kernel-bytes'
    if (id.includes('/src/generated/language-kernel/bytes')) return 'language-kernel-bytes'
    if (id.includes('/src/generated/harfbuzz/bytes')) return 'harfbuzz-bytes'
    if (id.includes('/src/generated/photogrammetry/bytes')) return 'photogrammetry-bytes'
    if (id.includes('/src/generated/wasm-brotli/bytes')) return 'wasm-brotli-bytes'
  } } } },
  build: {
    // The distribution verifier below owns explicit budgets for every large
    // chunk, including the packed geometry kernel.
    chunkSizeWarningLimit: 3_020,
    rollupOptions: {
      onwarn(warning, defaultWarn) {
        // Emscripten's browser bundle contains a dead Node.js fs branch. The
        // browser runtime supplies wasmBinary and never evaluates that path.
        if (warning.message.includes('Module "fs" has been externalized')
          && warning.message.includes('harfbuzzjs/hb.js')) return
        defaultWarn(warning)
      },
      output: {
        codeSplitting: {
          groups: [
            { name: 'geometry-kernel-bytes', test: /src[\\/]generated[\\/]geometry-kernels[\\/]bytes/ },
            { name: 'language-kernel-bytes', test: /src[\\/]generated[\\/]language-kernel[\\/]bytes/ },
            { name: 'harfbuzz-bytes', test: /src[\\/]generated[\\/]harfbuzz[\\/]bytes/ },
            { name: 'photogrammetry-bytes', test: /src[\\/]generated[\\/]photogrammetry[\\/]bytes/ },
            { name: 'wasm-brotli-bytes', test: /src[\\/]generated[\\/]wasm-brotli[\\/]bytes/ },
            { name: 'vr-core-bytes', test: /src[\\/]generated[\\/]vr-core[\\/]bytes/ },
            // Shared by the entry graph (photogrammetry loader) and lazy language
            // chunks; without its own chunk it drags the geometry kernel into
            // the entry preload list.
            { name: 'binary-codec', test: /(src[\\/]core[\\/]sha256\.ts|src[\\/]services[\\/](wasmPacking|wasmBase91|valueBinaryCodec)\.ts)$/ },
            // The entry graph needs only this regex; keep it out of the lazy
            // compiler chunk so the geometry kernel is not preloaded.
            { name: 'detect', test: /src[\\/]services[\\/]rushFrontendDetect\.ts$/ },
            // WASM host plumbing is shared by every kernel. Grouped with the
            // compiler it would pull the language kernel into the entry preload.
            { name: 'wasm-host', test: /src[\\/]services[\\/](wasmHost|wasmBrotliPacking)\.ts$/ },
            { name: 'rush-frontend', test: /src[\\/]services[\\/]rushFrontend\.ts$/ },
            { name: 'solid-draft-storage', test: /src[\\/]services[\\/]solidDraft(Head)?Store\.ts$/ },
            { name: 'directBodies', test: /src[\\/]services[\\/]directBodiesScad\.ts$/ },
            {
              // Direct-modeling helpers are shared by lazy CAD panels; they must
              // not capture the geometry kernel they reference.
              name: 'direct-modeling',
              test: /src[\\/]services[\\/]direct(Modeling|SolidTools|ProfileTools|SketchGeometry|ModelingTools)\.ts$/,
              includeDependenciesRecursively: false,
            },
            { name: 'solid-gpu-view', test: /src[\\/]services[\\/]solidGpuView/ },
            { name: 'surface-selection', test: /src[\\/]services[\\/]meshSurfaceGroups/ },
            { name: 'parser', test: /src[\\/]services[\\/]openscadParser|src[\\/]parser[\\/]/ },
            { name: 'renderer', test: /src[\\/]services[\\/]webgpuRenderer|src[\\/]renderer[\\/]/ },
            { name: 'vue', test: /node_modules[\\/](@vue|vue)/ },
          ],
        },
      },
    },
  },
})

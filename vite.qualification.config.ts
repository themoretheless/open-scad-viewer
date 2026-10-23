import { qualificationKernelModuleDeliveryPlugin } from './scripts/qualificationKernelModuleDelivery.mjs'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'

const qualificationEntry = fileURLToPath(
  new URL('./tests/fixtures/browser-qualification.html', import.meta.url),
)
const memoryEntry = fileURLToPath(
  new URL('./tests/fixtures/browser-memory-qualification.html', import.meta.url),
)

/**
 * Isolated G1 evidence bundle. It is intentionally separate from vite.config.ts
 * so neither the qualification lane nor its fault fixtures enter product dist.
 */
export default defineConfig({
  base: './',
  define: { __G1_SHARED_GEOMETRY_MODULE__: 'true' },
  plugins: [qualificationKernelModuleDeliveryPlugin()],
  worker: { plugins: () => [qualificationKernelModuleDeliveryPlugin()] },
  // This isolated entry uses embedded kernels, not product streaming assets.
  publicDir: false,
  build: {
    outDir: 'tmp/browser-qualification-dist',
    emptyOutDir: true,
    rollupOptions: { input: [qualificationEntry, memoryEntry] },
  },
})

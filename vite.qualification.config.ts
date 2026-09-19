import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'

const qualificationEntry = fileURLToPath(
  new URL('./tests/fixtures/browser-qualification.html', import.meta.url),
)

/**
 * Isolated G1 evidence bundle. It is intentionally separate from vite.config.ts
 * so neither the qualification lane nor its fault fixtures enter product dist.
 */
export default defineConfig({
  base: './',
  // This isolated entry uses embedded kernels, not product streaming assets.
  publicDir: false,
  build: {
    outDir: 'tmp/browser-qualification-dist',
    emptyOutDir: true,
    rollupOptions: { input: qualificationEntry },
  },
})

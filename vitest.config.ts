import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  test: {
    // Geometry-heavy suites exceed the 5s default when the full run saturates all cores.
    testTimeout: 30000,
    hookTimeout: 30000,
    // Limit worker count so WASM/worker-join timeouts do not trigger under full-load contention.
    maxWorkers: 4,
    projects: [
      { extends: true, test: { name: 'node', environment: 'node', include: ['tests/**/*.test.ts'], exclude: ['tests/directModelerUi.test.ts', 'tests/mainModelingUi.test.ts', 'tests/svgPanel.test.ts', 'tests/gcodePanel.test.ts'] } },
      { extends: true, test: { name: 'direct-ui', environment: './tests/directUiEnvironment.ts', include: ['tests/directModelerUi.test.ts', 'tests/mainModelingUi.test.ts', 'tests/svgPanel.test.ts', 'tests/gcodePanel.test.ts'] } },
    ],
  },
})

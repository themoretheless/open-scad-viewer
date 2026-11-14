import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vitest/config'
import { svetPlugin } from './scripts/svet-vite-plugin.mjs'

const SVET_RUNTIME_PATH = fileURLToPath(new URL('./src/forma/svet-runtime.ts', import.meta.url))

export default defineConfig({
  plugins: [svetPlugin()],
  resolve: {
    alias: [
      { find: /^vue$/, replacement: SVET_RUNTIME_PATH },
      { find: /^@vue\/server-renderer$/, replacement: SVET_RUNTIME_PATH },
      { find: /^svet$/, replacement: SVET_RUNTIME_PATH },
    ],
  },
  environments: {
    client: {
      dev: {
        inline: true,
      },
    },
  },
  ssr: {
    noExternal: true,
  },
  optimizeDeps: {
    exclude: ['vue', '@vue/server-renderer'],
  },
  test: {
    // Geometry-heavy suites exceed the 5s default when the full run saturates all cores.
    testTimeout: 30000,
    hookTimeout: 30000,
    // Limit worker count so WASM/worker-join timeouts do not trigger under full-load contention.
    maxWorkers: 4,
    projects: [
      { extends: true, test: { name: 'node', environment: 'node', include: ['tests/**/*.test.ts'], exclude: ['tests/offsetDeviationPanel.test.ts', 'tests/formaUi.test.ts', 'tests/meshModelerUi.test.ts', 'tests/mechanicsEditorsUi.test.ts', 'tests/directModelerUi.test.ts', 'tests/expressionInspectionFocus.test.ts', 'tests/profileBridgePanel.test.ts', 'tests/sourceGraphG2Panel.test.ts', 'tests/frozenGraphBodyPanel.test.ts', 'tests/solidVolumeDistanceUi.test.ts', 'tests/materialPathCheckUi.test.ts', 'tests/mainModelingUi.test.ts', 'tests/svgPanel.test.ts', 'tests/gcodePanel.test.ts', 'tests/vrControls.test.ts', 'tests/modalDialogUi.test.ts'] } },
      { extends: true, test: { name: 'direct-ui', environment: './tests/directUiEnvironment.ts', include: ['tests/offsetDeviationPanel.test.ts', 'tests/formaUi.test.ts', 'tests/mechanicsEditorsUi.test.ts', 'tests/directModelerUi.test.ts', 'tests/expressionInspectionFocus.test.ts', 'tests/profileBridgePanel.test.ts', 'tests/sourceGraphG2Panel.test.ts', 'tests/frozenGraphBodyPanel.test.ts', 'tests/solidVolumeDistanceUi.test.ts', 'tests/materialPathCheckUi.test.ts', 'tests/mainModelingUi.test.ts', 'tests/svgPanel.test.ts', 'tests/gcodePanel.test.ts', 'tests/vrControls.test.ts', 'tests/meshModelerUi.test.ts', 'tests/modalDialogUi.test.ts'] } },
    ],
  },
})

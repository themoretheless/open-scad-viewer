import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  test: {
    projects: [
      { extends: true, test: { name: 'node', environment: 'node', include: ['tests/**/*.test.ts'], exclude: ['tests/directModelerUi.test.ts', 'tests/mainModelingUi.test.ts'] } },
      { extends: true, test: { name: 'direct-ui', environment: './tests/directUiEnvironment.ts', include: ['tests/directModelerUi.test.ts', 'tests/mainModelingUi.test.ts'] } },
    ],
  },
})

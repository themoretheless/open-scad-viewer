// TypeScript/JavaScript lint configuration. Existing violations are pinned per directory in
// `architecture-baseline.json` under `eslint`; `npm run lint` fails only when a count grows.
import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'

export const LINT_IGNORES = [
  'node_modules/**',
  'dist/**',
  'output/**',
  'tmp/**',
  'public/**',
  'docs/**',
  'crates/**',
  'integrations/**',
  'forma-studio/**',
  'ui/**',
  'tests/fixtures/**',
  'src/generated/**',
  'src/render/shaders/generated/**',
  '**/*.d.ts',
  '.ci-*/**',
  '.kimi-worktrees/**',
]

export default tseslint.config({ ignores: LINT_IGNORES }, js.configs.recommended, ...tseslint.configs.recommended, {
  languageOptions: {
    ecmaVersion: 2022,
    sourceType: 'module',
    globals: { ...globals.browser, ...globals.node, ...globals.worker },
  },
  linterOptions: { reportUnusedDisableDirectives: 'warn' },
  rules: {
    // Unused bindings prefixed with `_` are intentional placeholders.
    '@typescript-eslint/no-unused-vars': [
      'error',
      { argsIgnorePattern: '^_', varsIgnorePattern: '^_', caughtErrors: 'none' },
    ],
  },
})

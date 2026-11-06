/**
 * # Svet SFC & UI Vite Compiler Plugin (`svet-vite-plugin.mjs`)
 *
 * Compiles `.svt` (and legacy `.vue`) SFCs directly to Svet runtime primitives (`src/forma/svet-runtime.ts`),
 * eliminating all runtime dependencies on `vue` and `@vitejs/plugin-vue`.
 */
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parse, compileScript, compileStyle } from '@vue/compiler-sfc'
import { transformWithEsbuild } from 'vite'

const SVET_RUNTIME_PATH = fileURLToPath(new URL('../src/forma/svet-runtime.ts', import.meta.url))
const PROJECT_ROOT = fileURLToPath(new URL('..', import.meta.url))

function normalizeFsPath(filename) {
  if (filename.startsWith(PROJECT_ROOT)) return filename
  if (filename.startsWith('/src/') || filename.startsWith('/tests/')) {
    return resolve(PROJECT_ROOT, filename.slice(1))
  }
  return filename
}

function isSvetSfc(filename) {
  return filename.endsWith('.svt')
}

function isSvetStyleVirtualId(id) {
  return id.includes('.svt?svet&type=style')
}

function getScopeHash(filename) {
  return createHash('sha256').update(filename).digest('hex').slice(0, 8)
}

function rewriteVueImportsToSvet(code) {
  return code.replace(
    /(\bfrom\s+['"])(?:vue|@vue\/server-renderer)(['"])/g,
    `$1${SVET_RUNTIME_PATH}$2`,
  )
}

export function svetPlugin() {
  const descriptorCache = new Map()

  return {
    name: 'vite:svet-sfc',
    enforce: 'pre',

    config() {
      return {
        resolve: {
          alias: [
            { find: /^vue$/, replacement: SVET_RUNTIME_PATH },
            { find: /^@vue\/server-renderer$/, replacement: SVET_RUNTIME_PATH },
            { find: /^svet$/, replacement: SVET_RUNTIME_PATH },
          ],
        },
      }
    },

    resolveId(id) {
      if (id === 'vue' || id === '@vue/server-renderer' || id === 'svet') {
        return SVET_RUNTIME_PATH
      }
      if (isSvetStyleVirtualId(id)) {
        const [rawFile, rawQuery] = id.split('?', 2)
        return `${normalizeFsPath(rawFile)}?${rawQuery}`
      }
      return null
    },

    load(id) {
      if (!isSvetStyleVirtualId(id)) return null
      const [rawFilename, rawQuery] = id.split('?', 2)
      const filename = normalizeFsPath(rawFilename)
      const params = new URLSearchParams(rawQuery)
      const index = Number(params.get('index') ?? 0)
      let descriptor = descriptorCache.get(filename)
      if (!descriptor) {
        const source = readFileSync(filename, 'utf8')
        descriptor = parse(source, { filename }).descriptor
        descriptorCache.set(filename, descriptor)
      }
      const styleBlock = descriptor.styles[index]
      if (!styleBlock) return ''
      const rawStyleSource = styleBlock.src
        ? readFileSync(resolve(dirname(filename), styleBlock.src), 'utf8')
        : styleBlock.content
      const styleFilename = styleBlock.src
        ? resolve(dirname(filename), styleBlock.src)
        : filename
      const scopeId = `data-v-${getScopeHash(filename)}`
      const compiled = compileStyle({
        source: rawStyleSource,
        filename: styleFilename,
        id: scopeId,
        scoped: Boolean(styleBlock.scoped),
      })
      return {
        code: compiled.code,
        map: null,
      }
    },

    async transform(code, id) {
      if (id.includes('?raw')) return null
      const [filename, rawQuery] = id.split('?', 2)
      if (!isSvetSfc(filename)) {
        if ((filename.endsWith('.ts') || filename.endsWith('.mts') || filename.endsWith('.js')) &&
            (code.includes("'vue'") || code.includes('"vue"') || code.includes('@vue/server-renderer'))) {
          return {
            code: rewriteVueImportsToSvet(code),
            map: null,
          }
        }
        return null
      }
      if (rawQuery) return null

      const { descriptor, errors } = parse(code, { filename })
      if (errors.length > 0) {
        throw errors[0]
      }
      descriptorCache.set(filename, descriptor)

      const shortId = getScopeHash(filename)
      const hasScoped = descriptor.styles.some(s => s.scoped)
      const scopeId = `data-v-${shortId}`

      const script = compileScript(descriptor, {
        id: shortId,
        inlineTemplate: true,
        isProd: true,
        templateOptions: {
          scoped: hasScoped,
          compilerOptions: {
            scopeId: hasScoped ? scopeId : undefined,
          },
        },
      })

      let outCode = rewriteVueImportsToSvet(script.content)

      if (descriptor.styles.length > 0) {
        const styleImports = descriptor.styles
          .map((s, i) => {
            if (s.src && !s.scoped) {
              return `import ${JSON.stringify(s.src)};`
            }
            const lang = s.lang || 'css'
            return `import ${JSON.stringify(`${filename}?svet&type=style&index=${i}&lang.${lang}`)};`
          })
          .join('\n')
        outCode = `${styleImports}\n${outCode}`
      }

      const extraProps = [
        `__file: ${JSON.stringify(filename)}`,
        ...(hasScoped ? [`__scopeId: ${JSON.stringify(scopeId)}`] : []),
      ].join(',\n  ')
      outCode = outCode.replace(
        /export\s+default\s+(?:\/\*[\s@]*__PURE__[\s*]*\*\/\s*)?_defineComponent\s*\(\{/,
        `export default /*@__PURE__*/_defineComponent({\n  ${extraProps},`,
      )
      outCode = outCode.replace(
        /return\s+\(\{([\s\S]*?)\}\s*as\s+unknown\s+as\s+[^\n)]+\)/,
        `return ({\n  ${extraProps},$1})`,
      )
      outCode = outCode.replace(
        /export\s+default\s+/,
        `import { registerCompiledSfc as __svet_reg } from ${JSON.stringify(SVET_RUNTIME_PATH)};\nconst __svet_sfc = `,
      )
      if (hasScoped) {
        outCode += `\n__svet_sfc.__scopeId = ${JSON.stringify(scopeId)};`
      }
      outCode += `\n__svet_reg(${JSON.stringify(filename)}, __svet_sfc);\nexport default __svet_sfc;\n`

      const transformed = await transformWithEsbuild(outCode, `${filename}.ts`, {
        loader: 'ts',
        target: 'es2022',
        sourcemap: true,
      })

      return {
        code: transformed.code,
        map: transformed.map,
      }
    },
  }
}

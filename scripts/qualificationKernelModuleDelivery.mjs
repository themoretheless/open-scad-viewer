import { fileURLToPath } from 'node:url'
const kernel = fileURLToPath(new URL('../src/services/geometry/kernel.ts', import.meta.url))
const required = fileURLToPath(new URL('../src/services/geometry/kernelCompilationRequired.ts', import.meta.url))

/** The qualification Worker receives a verified immutable module. Its runtime
 * must not import or parse the embedded byte payload, including in Vite dev.
 * The parent compiler's direct import remains the real verified compiler.
 */
export function qualificationKernelModuleDeliveryPlugin() {
  return {
    name: 'qualification-native-module-delivery',
    enforce: 'pre',
    resolveId(source, importer) {
      if (source === './kernelCompilation' && importer?.split('?')[0] === kernel) return required
    },
  }
}

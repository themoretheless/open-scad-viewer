import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
export function defaultCacheRoot(): string {
  return resolve(dirname(fileURLToPath(import.meta.url)), '../..', '.open-scad-runtime')
}

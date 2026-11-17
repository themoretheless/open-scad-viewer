#!/usr/bin/env node
// Ratchet for `noUncheckedIndexedAccess`: the flag is enabled per directory.
// Paths listed in CLEAN_PATHS must stay free of errors under the flag;
// the rest of the tree is migrated directory by directory.
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

/** Path prefixes (relative to the repository root) that compile cleanly under the flag. */
export const CLEAN_PATHS = Object.freeze([
  'src/composables/',
  'src/core/',
  'src/data/',
  'src/dev/',
  'src/exchange/',
  'src/fabrication/',
  'src/features/',
  'src/forma/',
  'src/main.ts',
  'src/parser/',
  'src/render/',
])

/** Path prefixes checked through `tsconfig.mcp.json`, which the root project does not include. */
export const MCP_CLEAN_PATHS = Object.freeze(['src/mcp/'])

const ERROR_LINE = /^(.+?)\(\d+,\d+\): error TS\d+:/

/** Returns tsc error lines whose file falls under one of the clean path prefixes. */
export function regressions(tscOutput, cleanPaths = CLEAN_PATHS) {
  return tscOutput.split(/\r?\n/).filter(line => {
    const file = ERROR_LINE.exec(line)?.[1]?.replaceAll('\\', '/')
    return (
      file !== undefined &&
      cleanPaths.some(prefix => (prefix.endsWith('/') ? file.startsWith(prefix) : file === prefix))
    )
  })
}

function tscOutput(extraArgs) {
  const tsc = spawnSync('npx', ['tsc', '--noEmit', '--pretty', 'false', '--noUncheckedIndexedAccess', ...extraArgs], {
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  })
  if (tsc.error) throw tsc.error
  return `${tsc.stdout}\n${tsc.stderr}`
}

function main() {
  const found = [...regressions(tscOutput([])), ...regressions(tscOutput(['-p', 'tsconfig.mcp.json']), MCP_CLEAN_PATHS)]
  if (found.length) {
    console.error(`noUncheckedIndexedAccess regressions in clean paths (${found.length}):`)
    for (const line of found) console.error(line)
    process.exit(1)
  }
  console.log(`unchecked index ratchet ok (${[...CLEAN_PATHS, ...MCP_CLEAN_PATHS].join(', ')})`)
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main()

'use strict'

const Module = globalThis.__OPENSCAD_MODULE__
if (!Module || Module.useNodeRawFS !== false) throw new Error('fixture requires MEMFS configuration')

const files = new Map()
const directories = new Set(['/'])
const asBuffer = (value) => Buffer.isBuffer(value)
  ? Buffer.from(value)
  : value instanceof Uint8Array
    ? Buffer.from(value)
    : Buffer.from(String(value), 'utf8')

Module.FS = {
  mkdir(path) {
    if (directories.has(path)) throw { errno: 20 }
    directories.add(path)
  },
  writeFile(path, value) {
    files.set(path, asBuffer(value))
  },
  readFile(path) {
    const value = files.get(path)
    if (!value) throw new Error(`fixture file missing: ${path}`)
    return Buffer.from(value)
  },
  stat(path) {
    const value = files.get(path)
    if (!value) throw new Error(`fixture file missing: ${path}`)
    return { size: value.byteLength }
  },
  chdir(path) {
    if (!directories.has(path)) throw new Error(`fixture directory missing: ${path}`)
  },
}

Module.callMain = args => {
  const source = files.get('/project/main.scad')?.toString('utf8') ?? ''
  if (source.includes('HANG_FOREVER')) {
    // Deliberately non-cooperative: only the parent process deadline/cancel can stop it.
    while (true) { /* spin */ }
  }
  if (source.includes('TRY_HOST_READ')) {
    require('node:fs').readFileSync('/etc/passwd')
  }
  if (source.includes('UNKNOWN_MODULE')) {
    Module.printErr("WARNING: Ignoring unknown module 'fixture_unknown'")
    throw { status: 1 }
  }
  if (source.includes('NOISY_RUNTIME')) {
    for (let index = 0; index < 512; index += 1) Module.printErr(`fixture-log-${index}-${'x'.repeat(3_000)}`)
  }
  if (source.includes('include <lib/helpers.scad>') && !files.has('/project/lib/helpers.scad')) {
    Module.printErr('WARNING: missing fixture include')
    throw { status: 1 }
  }
  const outputIndex = args.indexOf('-o')
  if (outputIndex < 0 || typeof args[outputIndex + 1] !== 'string') throw new Error('fixture output argument missing')
  const output = {
    args,
    source,
    projectFiles: [...files.keys()].sort(),
    rawFs: Module.useNodeRawFS,
    environment: Module.environment,
  }
  files.set(args[outputIndex + 1], Buffer.from(JSON.stringify(output), 'utf8'))
  return 0
}

Module.calledRun = true
Module.onRuntimeInitialized?.()
module.exports = Module

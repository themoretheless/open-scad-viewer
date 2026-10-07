#!/usr/bin/env node
// Observational tracing only: the frozen supervisor and its memory producer
// still own the jobs, sampling points, budgets, termination and cleanup.
import { spawn } from 'node:child_process'
import { createWriteStream } from 'node:fs'
import { mkdir, appendFile, writeFile } from 'node:fs/promises'
import { basename, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { readLinuxProcessTable, processTreePids } from './run-manifold-g1-memory-qualification.mjs'

const root = fileURLToPath(new URL('../', import.meta.url))
if (process.platform !== 'linux') throw new Error('WebKit process RSS diagnosis requires Linux')
if (process.argv.length !== 4 || process.argv[2] !== '--output') throw new Error('Use --output DIRECTORY')
const output = resolve(process.argv[3])
await mkdir(output, { recursive: true })
const tracePath = resolve(output, 'browser-process-rss.jsonl')
await writeFile(tracePath, '', { flag: 'wx' })
let reads = 0
let pending = Promise.resolve()
let reading = false
const failures = []
const trace = async () => {
  if (reading || reads >= 240) return
  reading = true
  try {
    const table = await readLinuxProcessTable()
    const descendants = processTreePids(table, process.pid)
    const processes = [...descendants].map(pid => table.get(pid))
      .filter(p => /WebKit|MiniBrowser|pw_run\.sh/.test(p.command))
      .map(p => ({ pid: p.pid, parentPid: p.parentPid, rssBytes: p.rssBytes,
        executable: basename(p.command.split(/\s+/)[0]).slice(0, 64) }))
    await appendFile(tracePath, `${JSON.stringify({ at: new Date().toISOString(), processes })}\n`)
    reads++
  } catch (error) {
    if (failures.length < 8) failures.push(String(error).slice(0, 512))
  } finally { reading = false }
}
const stdout = createWriteStream(resolve(output, 'supervisor-stdout.json'), { flags: 'wx' })
const stderr = createWriteStream(resolve(output, 'supervisor-stderr.log'), { flags: 'wx' })
const child = spawn(process.execPath, ['scripts/run-browser-qualification-supervisor.mjs',
  '--mode', 'memory', '--browser', 'webkit', '--run-index', '1'], {
  cwd: root, env: process.env, stdio: ['ignore', 'pipe', 'pipe'],
})
child.stdout.pipe(stdout)
child.stderr.pipe(stderr)
const timer = setInterval(() => { if (!reading) pending = trace() }, 10_000)
let result
try {
  result = await new Promise((accept, reject) => {
    child.once('error', reject)
    child.once('close', (exitCode, signal) => accept({ exitCode, signal }))
  })
} finally {
  clearInterval(timer)
  await pending
  await Promise.all([stdout, stderr].map(stream => stream.closed ? undefined
    : new Promise((accept, reject) => { stream.once('close', accept); stream.once('error', reject) })))
}
const summary = { schema: 'g1-webkit-process-memory-diagnostic-v1', diagnosticOnly: true,
  qualificationClaim: 'none', qualificationUnits: 0, sourceSha: process.env.GITHUB_SHA ?? null,
  processTraceReads: reads, processTraceFailures: failures, ...result }
await writeFile(resolve(output, 'diagnostic.json'), `${JSON.stringify(summary, null, 2)}\n`)
console.log(JSON.stringify(summary))
process.exitCode = failures.length ? 2 : result.exitCode ?? 2

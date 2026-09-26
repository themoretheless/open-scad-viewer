#!/usr/bin/env node
/**
 * Provision an Ubuntu 24.04 container and run one G1 matrix harness command.
 * Results are discovery-only unless the caller separately proves frozen OS/Node
 * archive digests and the full cleanRunDefinition.
 *
 * Usage:
 *   node scripts/run-g1-ubuntu-docker-fragment.mjs \
 *     --row oracle-differential \
 *     --env ubuntu-node22 \
 *     --run-index 1
 */

import { spawn } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync, existsSync, appendFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repositoryRoot = fileURLToPath(new URL('../', import.meta.url))
const PLAN = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v39.json')
const RESULT = resolve(repositoryRoot, 'output/qualification/semantic-manifold-g1-candidate-run-v39/result.json')
const FRAGMENTS = resolve(repositoryRoot, 'output/qualification/semantic-manifold-g1-candidate-run-v39/fragments.jsonl')
const DOCKER_HOST = process.env.DOCKER_HOST ?? 'unix:///Users/themoretheless/.colima/default/docker.sock'

const NODE_BY_ENV = {
  'ubuntu-node20': { version: '20.19.0', artifact: 'node-v20.19.0-linux-x64.tar.xz', sha256: 'b4e336584d62abefad31baecff7af167268be9bb7dd11f1297112e6eed3ca0d5' },
  'ubuntu-node22': { version: '22.23.2', artifact: 'node-v22.23.2-linux-x64.tar.xz', sha256: 'd60acfe00a2932254bb0ad20e01b0d74397a0875595de719654b214f4b03f307' },
}

function die(msg, code = 2) {
  console.error(msg)
  process.exit(code)
}

function parseArgs(argv) {
  const values = new Map()
  for (let i = 0; i < argv.length; i += 1) {
    const name = argv[i]
    const value = argv[++i]
    if (!name?.startsWith('--') || value === undefined) die('Usage: --row --env --run-index')
    values.set(name, value)
  }
  return {
    row: values.get('--row'),
    env: values.get('--env'),
    runIndex: Number(values.get('--run-index')),
  }
}

function run(cmd, args, opts = {}) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(cmd, args, { stdio: ['ignore', 'pipe', 'pipe'], env: { ...process.env, DOCKER_HOST }, ...opts })
    const out = []
    const err = []
    child.stdout.on('data', (b) => out.push(b))
    child.stderr.on('data', (b) => err.push(b))
    child.on('close', (code) => {
      resolvePromise({
        code,
        stdout: Buffer.concat(out).toString('utf8'),
        stderr: Buffer.concat(err).toString('utf8'),
      })
    })
    child.on('error', reject)
  })
}

function digest(text) {
  const bytes = Buffer.from(text, 'utf8')
  return { algorithm: 'sha256', value: createHash('sha256').update(bytes).digest('hex'), byteLength: bytes.byteLength }
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const plan = JSON.parse(readFileSync(PLAN, 'utf8'))
  const row = plan.matrix.find((r) => r.id === args.row)
  if (!row) die(`Unknown row ${args.row}`)
  if (!row.executionEnvironmentIds.includes(args.env)) die(`Row does not admit ${args.env}`)
  const node = NODE_BY_ENV[args.env]
  if (!node) die(`Docker helper only supports ubuntu-node20/22, got ${args.env}`)
  if (!Number.isInteger(args.runIndex) || args.runIndex < 1 || args.runIndex > row.work.cleanRunsRequired) {
    die(`Invalid run-index`)
  }
  const seed = row.work.seeds[args.runIndex - 1]

  // amd64 on arm Colima via qemu — listed arch, but OS image digest is not plan-frozen → discovery-only
  // Prefer official Node bookworm images (preinstalled toolchain). Still
  // discovery-only: image digest is not the plan's frozen Ubuntu 24.04 binding.
  const platform = 'linux/amd64'
  const image = `node:${node.version}-bookworm`
  const harnessRaw = row.harness.command
    .replaceAll('<1|2|3>', String(args.runIndex))
    .replaceAll('<chromium|webkit>', 'chromium')
  // Skip package pretest (geometry WASM build needs Rust). Use Vitest binary directly.
  const harness = harnessRaw.startsWith('npm test -- ')
    ? `./node_modules/.bin/vitest run ${harnessRaw.slice('npm test -- '.length).replace(/^--run\s+/, '')}`
    : harnessRaw

  const script = `
set -euo pipefail
node -v
npm -v
cd /work
rm -rf /tmp/g1-work
mkdir -p /tmp/g1-work
tar -C /work -cf - \
  package.json package-lock.json .npmrc vitest.config.ts tsconfig.json \
  src tests scripts docs crates tools \
  | tar -C /tmp/g1-work -xf -
# Host-built generated kernels (gitignored) — required when pretest/Rust is skipped.
if [ -d /work/src/generated ]; then
  mkdir -p /tmp/g1-work/src
  cp -a /work/src/generated /tmp/g1-work/src/
fi
cd /tmp/g1-work
npm ci --ignore-scripts --include=dev --registry=https://registry.npmjs.org/ --audit=false --fund=false
export QUALIFICATION_SEED=${seed}
export QUALIFICATION_PLAN_ID=semantic-manifold-g1-plan-v39
export CI=1
${harness}
`

  const startedAt = new Date().toISOString()
  const startedMs = Date.now()
  const outcome = await run('docker', [
    'run', '--rm', '--platform', platform,
    '-v', `${repositoryRoot}:/work`,
    '-w', '/work',
    image,
    'bash', '-lc', script,
  ])
  const endedAt = new Date().toISOString()
  const passed = outcome.code === 0

  mkdirSync(dirname(RESULT), { recursive: true })
  if (!existsSync(RESULT)) {
    writeFileSync(RESULT, `${JSON.stringify({
      schema: 'open-scad-viewer/qualification-candidate-result',
      schemaVersion: 1,
      candidateRunId: plan.executionProtocol.candidateRunId,
      planId: plan.planId,
      createdAt: startedAt,
      priorResultsImported: false,
      plannedWorkUnits: plan.executionProtocol.plannedWorkUnits,
      completedWorkUnits: 0,
      status: 'in-progress',
      qualificationClaim: 'none',
      fragments: [],
    }, null, 2)}\n`)
  }

  const fragment = {
    fragmentId: `${row.id}/${args.env}/run-${args.runIndex}`,
    matrixRowId: row.id,
    environmentId: args.env,
    runIndex: args.runIndex,
    seed,
    classification: 'discovery-only',
    status: passed ? 'passed' : 'failed',
    unitsPlanned: row.work.unitsPerCleanRun,
    unitsCompleted: passed ? row.work.unitsPerCleanRun : 0,
    startedAt,
    endedAt,
    durationMs: Date.now() - startedMs,
    command: ['docker', 'run', '--platform', platform, image, harness],
    exitCode: outcome.code,
    stdoutDigest: digest(outcome.stdout.slice(0, 256 * 1024)),
    stderrDigest: digest(outcome.stderr.slice(0, 256 * 1024)),
    host: { platform: 'linux', arch: 'x86_64', via: 'colima-qemu', image, imageNote: 'node:*-bookworm amd64; not frozen Ubuntu 24.04 → discovery-only' },
    protocolNotes: [
      'Docker/Colima path uses linux/amd64 Node bookworm images.',
      'OS image digest and full cleanRunDefinition wipe/sanitized-env are not claimed.',
      'Cannot satisfy u07 until clean-post-freeze classification is earned.',
    ],
  }

  appendFileSync(FRAGMENTS, `${JSON.stringify(fragment)}\n`)
  const result = JSON.parse(readFileSync(RESULT, 'utf8'))
  result.fragments.push(fragment)
  result.updatedAt = endedAt
  result.completedWorkUnits = result.fragments
    .filter((f) => f.classification === 'clean-post-freeze' && f.status === 'passed')
    .reduce((s, f) => s + Number(f.unitsCompleted ?? 0), 0)
  writeFileSync(RESULT, `${JSON.stringify(result, null, 2)}\n`)

  if (!passed) {
    console.error(outcome.stderr.slice(-4000))
    console.error(outcome.stdout.slice(-4000))
  }
  console.log(JSON.stringify({
    fragmentId: fragment.fragmentId,
    status: fragment.status,
    classification: fragment.classification,
    exitCode: outcome.code,
  }, null, 2))
  process.exit(passed ? 0 : 1)
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})

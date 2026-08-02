import { spawn } from 'node:child_process'

const mode = process.argv[2]
const browser = process.argv[3] ?? 'chromium'
const runIndex = Number(process.argv[4] ?? '1')

if (mode === 'actual-pass') {
  process.stdout.write(JSON.stringify({
    qualificationClaim: 'none',
    status: 'passed',
    browser,
    cleanRunFragment: 1,
  }) + '\n')
} else if (mode === 'memory-pass') {
  process.stdout.write(JSON.stringify({
    qualificationClaim: 'none',
    status: 'single-clean-run-passed',
    browser,
    orchestration: {
      runIndex,
      cleanRunsRepresentedByThisInvocation: 1,
    },
  }) + '\n')
} else if (mode === 'overflow') {
  process.stdout.write('x'.repeat(300 * 1024))
} else if (mode === 'hang') {
  spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], {
    stdio: 'ignore',
  }).unref()
  setInterval(() => {}, 1000)
} else {
  process.stderr.write('unknown fixture mode\n')
  process.exitCode = 2
}

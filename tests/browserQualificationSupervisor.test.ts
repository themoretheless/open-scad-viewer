import { resolve } from 'node:path'
import { describe, expect, it, vi } from 'vitest'

import {
  BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT,
  parseBrowserSupervisorArguments,
  superviseBrowserQualification,
} from '../scripts/run-browser-qualification-supervisor.mjs'

const fixture = resolve(
  import.meta.dirname,
  'fixtures/browser-qualification-supervisor-child.mjs',
)

function invocation(mode: string, browser = 'chromium', runIndex = 1, timeoutMs = 2_000) {
  return {
    script: fixture,
    arguments: [mode, browser, String(runIndex)],
    timeoutMs,
  }
}

describe('browser qualification process supervisor', () => {
  it('accepts only one frozen mode, engine, and clean-run index', () => {
    expect(parseBrowserSupervisorArguments([
      '--mode', 'actual', '--browser', 'webkit', '--run-index', '3',
    ])).toEqual({ mode: 'actual', browser: 'webkit', runIndex: 3 })
    for (const argv of [
      ['--mode', 'actual', '--browser', 'chromium'],
      ['--mode', 'other', '--browser', 'chromium', '--run-index', '1'],
      ['--mode', 'memory', '--browser', 'chrome', '--run-index', '1'],
      ['--mode', 'memory', '--browser', 'firefox', '--run-index', '4'],
    ]) expect(() => parseBrowserSupervisorArguments(argv)).toThrowError(expect.objectContaining({
      code: 'E_SUPERVISOR_ARGUMENT',
    }))
  })

  it('fails closed before publication when spawn exposes no valid process group id', async () => {
    const kill = vi.fn()
    await expect(superviseBrowserQualification({
      mode: 'actual', browser: 'chromium', runIndex: 1,
    }, {
      platform: 'linux',
      invocation: invocation('actual-pass'),
      spawn: () => ({ pid: undefined, kill }),
    })).rejects.toMatchObject({ code: 'E_SUPERVISOR_SPAWN' })
    expect(kill).toHaveBeenCalledWith('SIGKILL')
  })

  it.skipIf(process.platform === 'win32')('publishes only after a passing child joins with an empty process group', async () => {
    const actual = await superviseBrowserQualification({
      mode: 'actual', browser: 'chromium', runIndex: 2,
    }, {
      platform: 'linux',
      invocation: invocation('actual-pass'),
    })
    expect(actual).toMatchObject({
      schema: BROWSER_QUALIFICATION_SUPERVISOR_CONTRACT.schema,
      status: 'single-clean-run-passed',
      qualificationClaim: 'none',
      mode: 'actual',
      browser: 'chromium',
      runIndex: 2,
      containment: {
        hardKillSignal: 'SIGKILL',
        joinedBeforePublication: true,
        processGroupEmptyBeforePublication: true,
      },
      childEvidence: { status: 'passed' },
    })

    const memory = await superviseBrowserQualification({
      mode: 'memory', browser: 'firefox', runIndex: 3,
    }, {
      platform: 'linux',
      invocation: invocation('memory-pass', 'firefox', 3),
    })
    expect(memory).toMatchObject({
      status: 'single-clean-run-passed',
      mode: 'memory',
      browser: 'firefox',
      runIndex: 3,
      childEvidence: { orchestration: { runIndex: 3 } },
    })
  })

  it.skipIf(process.platform === 'win32')('hard-kills and joins a timed-out child process group', async () => {
    await expect(superviseBrowserQualification({
      mode: 'actual', browser: 'chromium', runIndex: 1,
    }, {
      platform: 'linux',
      invocation: invocation('hang', 'chromium', 1, 50),
    })).rejects.toMatchObject({
      code: 'E_SUPERVISOR_TIMEOUT',
      details: { timeoutMs: 50, hardKilledAndJoined: true },
    })
  })

  it.skipIf(process.platform === 'win32')('detects and kills a process group left after the direct child exits', async () => {
    let groupProbe = 0
    const kill = (_pid: number, signal: NodeJS.Signals | 0) => {
      if (signal === 'SIGKILL') return true
      groupProbe++
      if (groupProbe === 1) return true
      const error = Object.assign(new Error('group absent'), { code: 'ESRCH' })
      throw error
    }
    await expect(superviseBrowserQualification({
      mode: 'actual', browser: 'chromium', runIndex: 1,
    }, {
      platform: 'linux',
      invocation: invocation('actual-pass'),
      kill,
    })).rejects.toMatchObject({
      code: 'E_SUPERVISOR_ORPHAN_GROUP',
      details: { hardKilledAndJoined: true },
    })
  })

  it.skipIf(process.platform === 'win32')('drains but rejects unbounded child output', async () => {
    await expect(superviseBrowserQualification({
      mode: 'actual', browser: 'chromium', runIndex: 1,
    }, {
      platform: 'linux',
      invocation: invocation('overflow'),
    })).rejects.toMatchObject({ code: 'E_SUPERVISOR_OUTPUT_LIMIT' })
  })
})

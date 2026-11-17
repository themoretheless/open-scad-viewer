import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import {
  expressionShellExpectation,
  validExpressionShell,
  type ExpressionShellOptions,
} from '../src/kernel/expressionShell'
type SupportCheck = { boundaries: Record<string, unknown>[]; cells: number; spans: number; [key: string]: unknown }
type Fixture = {
  request: ExpressionShellOptions & { supportTangencyCheck: SupportCheck }
  shell: { supportTangencyCheck: SupportCheck }
}
const fixtures: { admitted: Fixture; unproven: Fixture } = JSON.parse(
  readFileSync(
    new URL(
      '../docs/qualification/mapped-retained-carrier-2026-10-08/expression-support-tangency-dispatch-2026-10-09.json',
      import.meta.url,
    ),
    'utf8',
  ),
)
it('accepts actual native support joins and bound-free exhausted work', () => {
  for (const value of Object.values(fixtures) as Fixture[]) {
    expect(validExpressionShell(expressionShellExpectation(value.request), value)).toBe(true)
  }
})
it('rejects missing boundaries, stale limits and fabricated body admission', () => {
  const original = fixtures.admitted,
    expected = expressionShellExpectation(original.request)
  for (const change of [
    (v: Fixture) => {
      v.shell.supportTangencyCheck.boundaries.pop()
    },
    (v: Fixture) => {
      v.shell.supportTangencyCheck.neighborBodyQualified = true
    },
    (v: Fixture) => {
      v.shell.supportTangencyCheck.solidQualified = true
    },
    (v: Fixture) => {
      v.request.supportTangencyCheck.spans--
    },
    (v: Fixture) => {
      v.shell.supportTangencyCheck.cells++
    },
    (v: Fixture) => {
      v.shell.supportTangencyCheck.boundaries[0]!.pending = [
        [
          [0, 1],
          [0, 1],
        ],
      ]
    },
  ]) {
    const modified = structuredClone(original)
    change(modified)
    expect(validExpressionShell(expected, modified)).toBe(false)
  }
})
it('rejects a forged certificate when native support work is disabled', () => {
  const modified = structuredClone(fixtures.unproven)
  modified.shell.supportTangencyCheck = structuredClone(fixtures.admitted.shell.supportTangencyCheck)
  expect(validExpressionShell(expressionShellExpectation(modified.request), modified)).toBe(false)
})

it('rejects contradictory proof status and reasons at report and seam level', () => {
  const original = fixtures.admitted,
    expected = expressionShellExpectation(original.request)
  for (const change of [
    (v: Fixture) => {
      v.shell.supportTangencyCheck.reason = 'profile-transition-support-tangency-work-limit'
    },
    (v: Fixture) => {
      v.shell.supportTangencyCheck.boundaries[0]!.reason = 'full-section-join-regularity-unproven'
    },
    (v: Fixture) => {
      v.shell.supportTangencyCheck.boundaries[2]!.reason = 'full-section-join-qualified'
    },
  ]) {
    const modified = structuredClone(original)
    change(modified)
    expect(validExpressionShell(expected, modified)).toBe(false)
  }
  const modified = structuredClone(fixtures.unproven)
  modified.shell.supportTangencyCheck.reason = 'profile-transition-support-tangency-qualified'
  expect(validExpressionShell(expressionShellExpectation(modified.request), modified)).toBe(false)
})

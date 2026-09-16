import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

describe('analytic STEP product seam', () => {
  it('keeps analytic peer separate from faceted cadStep', () => {
    const analytic = readFileSync('src/services/cadAnalyticStep.ts', 'utf8')
    const faceted = readFileSync('src/services/cadStep.ts', 'utf8')
    expect(analytic).toContain('brep_nurbs_export_step')
    expect(analytic).toContain('brep_nurbs_import_step')
    expect(analytic).toContain('FACETED_BREP')
    expect(analytic).toContain('cadStep.ts')
    expect(faceted).toContain('FACETED_BREP')
    expect(faceted).not.toContain('brep_nurbs_export_step')
  })
})

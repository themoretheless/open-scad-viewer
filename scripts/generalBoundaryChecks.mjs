// Protocol assertions only: all geometry and admission execute in Rust.
export function runGeneralBoundaryChecks(fixture, dispatch) {
  const records = []
  const expect = (value, message) => { if (!value) throw Error(message) }
  const run = (name, request, check) => {
    const result = dispatch(request)
    check(result)
    records.push({name, passed: true, certified: result.certified,
      continuousBound: result.continuousBound, reason: result.reason ?? null})
    return result
  }
  const body = run('original-moving-frame-filled-caps', fixture.sweep, r => {
    expect(r.certified && r.continuousBound && r.withinBudget && r.solidGeometryCertified, 'Original boundary unproved')
    expect(r.model && r.regularity?.proven, 'Missing native certified geometry')
    expect(r.boundaryErrorUpper < fixture.sweep.maxDeviation && r.filledCapErrorUpper?.length === 2, 'Missing filled-cap error')
    expect(r.idealBodyEmbeddingCertified === false && r.smoothnessCertified === false && r.seamContinuity === 'C0', 'Unsupported guarantee promoted')
  })
  run('authored-reference-whole-boundary', fixture.common, r => {
    expect(r.certified && r.referenceVolume?.proven && r.retainedVolume?.proven, 'Common boundary unproved')
    expect(r.sourceMotionLawCertified === false && r.smoothnessCertified === false, 'Authored reference cannot infer a law')
  })
  for (const key of ['sourceCells', 'sourceProducts', 'sourceExactWork', 'regularityCells']) {
    const request = structuredClone(fixture.sweep)
    request.limits[key] = 0
    request.certified = true
    request.continuousBound = true
    request.model = body.model
    run(`refuse-${key}-despite-client-flags`, request, r => expect(r.certified === false && r.model === null, `Forged ${key} admission`))
  }
  const short = structuredClone(fixture.sweep)
  short.limits.volume.maxPairs = 1
  run('refuse-unfinished-solid-pairs', short, r => expect(r.certified === false && r.model === null && r.solidGeometryCertified === false, 'Unfinished pairs admitted'))
  for (const stage of ['referenceVolume', 'retainedVolume']) {
    const request = structuredClone(fixture.common)
    request.limits[stage].maxPairs = 1
    run(`refuse-${stage}`, request, r => expect(r.certified === false && r.continuousBound === true && r[stage]?.proven === false, `Borrowed ${stage} proof`))
  }
  const bad = structuredClone(fixture.sweep)
  bad.limits.sourceCells = 0
  bad.sourceCaps[0].surface.weights[0][0] = 0
  let rejected = false
  try { dispatch(bad) } catch { rejected = true }
  expect(rejected, 'Invalid source weight bypassed validation')
  records.push({name: 'reject-invalid-source-before-zero-budget', passed: true, certified: false})
  const exported = dispatch({op: 'brep_nurbs_export_step_v9', model: body.model})
  const restored = dispatch({op: 'brep_nurbs_import_step_v9', text: exported.text})
  expect(exported.certificate.complete && restored.certificate.complete, 'STEP certificate incomplete')
  expect(restored.model.faces.length === body.model.faces.length && restored.model.edges.length === body.model.edges.length, 'STEP topology changed')
  records.push({name: 'direct-step-topology-round-trip', passed: true})
  return {scope: 'finite native protocol, filled-cap, budget-refusal and STEP checks; no UI or universal acceptance claim',
    passed: records.length, records}
}

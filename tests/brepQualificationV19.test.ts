import {readFileSync} from 'node:fs'
import {describe,expect,it} from 'vitest'
import {BREP_CAPABILITY_MATRIX} from '../src/services/geometry/brepCapability'

const read=(path:string)=>JSON.parse(readFileSync(path,'utf8'))

describe('G8 append-only freeform analysis + ledger catch-up v19',()=>{
 it('ships freeform tess/mass and previously-Qualified NURBS cells without rewriting v18',()=>{
  const index=read('docs/qualification/plans/g8-full-matrix-index-v19.json')
  const matrix=read('docs/qualification/brep-full-closed-matrix-v19.json')
  const registry=read('docs/qualification/brep-capability-registry-release-full-v19.json')
  expect(index.successorOf).toBe('docs/qualification/plans/g8-full-matrix-index-v18.json')
  expect(matrix.successorOf).toBe('docs/qualification/brep-full-closed-matrix-v18.json')
  expect(registry.successorOf).toBe('docs/qualification/brep-capability-registry-release-full-v18.json')

  for(const id of [
   'certified-generic-rational-freeform-tessellation/1',
   'certified-generic-rational-freeform-mass-quadrature/1',
   'exact-valence3-corner-blend/1',
   'exact-variable-radius-fillet/1',
   'nurbs-ss/1',
   'nurbs-boolean/1',
   'step-interchange/6',
   'nurbs-step-bicubic-face/1',
   'nurbs-step-trimmed-bicubic/1',
   'nurbs-step-solid/1',
  ]){
   expect(index.capabilities.find((entry:{id:string})=>entry.id===id))
    .toMatchObject({maturity:'Qualified',releaseState:'shipped'})
   expect(matrix.admittedOps).toContain(id)
   expect(registry.capabilities).toContain(id)
   expect(BREP_CAPABILITY_MATRIX.find(row=>row.id===id)?.maturity).toBe('Qualified')
  }

  expect(matrix.explicitRefuse).not.toContain('certified-generic-rational-freeform-tessellation')
  expect(matrix.explicitRefuse).not.toContain('certified-generic-rational-freeform-mass-quadrature')
  expect(matrix.explicitRefuse).toContain('freeform-x-freeform-boolean')
  expect(matrix.explicitRefuse).toContain('freeform-bump-mass-quadrature')
  expect(matrix.explicitRefuse).toContain('freeform-varying-weight-analysis')
  expect(matrix.pendingQualification).toEqual([])
  expect(matrix.unresolvedCandidateRows).toEqual([])
 })

 it('binds freeform tessellation and mass plans to product evidence',()=>{
  const tess=read('docs/qualification/plans/certified-generic-rational-freeform-tessellation-1.json')
  const tessEv=read('docs/qualification/certified-generic-rational-freeform-tessellation-1-evidence-v1.json')
  expect(tess).toMatchObject({
   capability:'certified-generic-rational-freeform-tessellation/1',
   lifecycle:{status:'qualified'},
   unresolvedRows:[],
  })
  expect(tessEv.capability).toBe('certified-generic-rational-freeform-tessellation/1')

  const mass=read('docs/qualification/plans/certified-generic-rational-freeform-mass-quadrature-1.json')
  const massEv=read('docs/qualification/certified-generic-rational-freeform-mass-quadrature-1-evidence-v1.json')
  expect(mass).toMatchObject({
   capability:'certified-generic-rational-freeform-mass-quadrature/1',
   lifecycle:{status:'qualified'},
   unresolvedRows:[],
  })
  expect(massEv.capability).toBe('certified-generic-rational-freeform-mass-quadrature/1')
 })
})

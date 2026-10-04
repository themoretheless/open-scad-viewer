import {expect, it} from 'vitest'
import {inspectSweepWalls, type SweepWallAuditOptions} from '../src/services/nurbsSweepAudit'
import type {NurbsSurface} from '../src/services/nurbsSurface'

const plane = (x: number, y = 0): NurbsSurface => ({
  degreeU: 1, degreeV: 1, knotsU: [0,0,1,1], knotsV: [0,0,1,1],
  controlPoints: [x,x+1].map(u => [y,y+1].map(v => [u,v,0])),
  weights: [[1,1],[1,1]],
})
const options: SweepWallAuditOptions = {
  sharedBoundaries: [], clearance: 0, distanceTolerance: .001,
  maxInjectivityCells: 100, maxPairs: 100, maxPairCells: 100,
}
it('preserves whole-chart and pair evidence without claiming shell embedding', () => {
  const walls = [plane(0), plane(3)], before = structuredClone(walls)
  const report = inspectSweepWalls(walls, options)
  expect(report).toMatchObject({chartsAndPairsCertified: true, globalEmbeddingCertified: false,
    unresolvedCharts: [], pairs: {allPairsSeparated: true, unresolved: []}})
  expect(report.charts).toHaveLength(2)
  expect(report.charts.every(chart => chart.certified && chart.projection !== null)).toBe(true)
  expect(walls).toEqual(before)
})
it('keeps exhausted chart and pair hierarchy budgets independent', () => {
  const walls = [plane(0), plane(3)]
  const chart = inspectSweepWalls(walls, {...options, maxInjectivityCells: 0})
  expect(chart).toMatchObject({chartsAndPairsCertified: false, unresolvedCharts: [0,1],
    pairs: {allPairsSeparated: true}})
  expect(chart.charts.every(item => item.reason === 'cell-budget-exhausted' && item.projection === null)).toBe(true)
  const pair = inspectSweepWalls(walls, {...options, maxPairs: 0})
  expect(pair.charts.every(item => item.certified)).toBe(true)
  expect(pair.chartsAndPairsCertified).toBe(false)
  expect(pair.pairs.unresolved.map(item => item.patches)).toEqual([[0,1]])
})
it('distinguishes exact shared boundaries from gaps and interior overlap', () => {
  const sharedBoundaries: [number, number][] = [[0,1]]
  expect(inspectSweepWalls([plane(0),plane(0,1)], {...options, sharedBoundaries}))
    .toMatchObject({chartsAndPairsCertified: true, declaredBoundariesC0: true,
      c0Boundaries: [[0,1]], pairs: {boundaryOnlyPairs: [[0,1]]}})
  expect(inspectSweepWalls([plane(0),plane(0,1.25)], {...options, sharedBoundaries}))
    .toMatchObject({chartsAndPairsCertified: false, declaredBoundariesC0: false,
      unresolvedBoundaries: [[0,1]]})
  const overlap = inspectSweepWalls([plane(0),plane(.5)], {...options, maxPairCells: 0})
  expect(overlap.charts.every(item => item.certified)).toBe(true)
  expect(overlap.chartsAndPairsCertified).toBe(false)
  expect(overlap.pairs.unresolved).toHaveLength(1)
  expect(() => inspectSweepWalls([plane(0)], {...options, sharedBoundaries})).toThrow()
  expect(() => inspectSweepWalls([plane(0)], {...options, maxPairs: -1})).toThrow()
})
it('does not promote separated pairs when a retained chart folds', () => {
  const folded = plane(0)
  folded.knotsU = [0,0,.5,1,1]
  folded.controlPoints.push(structuredClone(folded.controlPoints[0]!))
  folded.weights.push([1,1])
  const report = inspectSweepWalls([folded,plane(3)], options)
  expect(report.pairs.allPairsSeparated).toBe(true)
  expect(report.chartsAndPairsCertified).toBe(false)
  expect(report.unresolvedCharts).toContain(0)
  expect(report.charts[0]!.certified).toBe(false)
})
it('certifies rational tensor boundaries on either parameter axis',()=>{
 const patch=(xs:number[]):NurbsSurface=>({
  degreeU:2,degreeV:2,knotsU:[0,0,0,1,1,1],knotsV:[0,0,0,1,1,1],
  controlPoints:xs.map(x=>[0,.5,1].map(y=>[x,y,0])),weights:[[1,2,1],[1,2,1],[1,2,1]],
 })
 const a=patch([-1,-.5,0]),b=patch([0,.5,1])
 const opts={...options,sharedBoundaries:[[0,1]] as [number,number][],maxPairCells:0}
 expect(inspectSweepWalls([a,b],opts)).toMatchObject({declaredBoundariesC0:true,pairs:{allPairsCompatible:true,boundaryOnlyPairs:[[0,1]]}})
 const transpose={...b,controlPoints:b.controlPoints.map((_,u)=>b.controlPoints.map(row=>row[u]!)),weights:b.weights.map((_,u)=>b.weights.map(row=>row[u]!))}
 expect(inspectSweepWalls([a,transpose],opts).pairs.allPairsCompatible).toBe(true)
 expect(inspectSweepWalls([a,patch([0,-.5,-1])],opts).pairs.allPairsCompatible).toBe(false)
 const changed=structuredClone(b);changed.weights[0]![1]=3
 expect(inspectSweepWalls([a,changed],opts).declaredBoundariesC0).toBe(false)
})

it('allows perpendicular profile walls only with a checked separating plane',()=>{
 const patch=(xs:number[],other:boolean):NurbsSurface=>({
  degreeU:2,degreeV:2,knotsU:[0,0,0,1,1,1],knotsV:[0,0,0,1,1,1],
  controlPoints:xs.map(x=>[0,.5,1].map(z=>other?[0,x,z]:[x,0,z])),
  weights:[[1,2,1],[1,2,1],[1,2,1]],
 })
 const a=patch([-1,-.5,0],false),b=patch([0,.5,1],true)
 const opts={...options,sharedBoundaries:[[0,1]] as [number,number][],maxPairCells:0}
 expect(inspectSweepWalls([a,b],opts).pairs).toMatchObject({allPairsCompatible:true,boundaryOnlyPairs:[[0,1]]})
 b.controlPoints[2]![1]=structuredClone(a.controlPoints[0]![1]!)
 expect(inspectSweepWalls([a,b],opts).pairs.allPairsCompatible).toBe(false)
})

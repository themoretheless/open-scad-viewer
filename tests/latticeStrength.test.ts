import {describe,it,expect} from 'vitest'
import {analyzeLatticePrintability,optimizeLatticeForPrint,formatLatticePrintReport,latticeSlicerHintsJson} from '../src/services/latticePrintReport'
import {analyzeLatticeStrength,formatLatticeStrengthReport,latticeMaterials,latticeStructureHint,weakSpotMarkerMeshes} from '../src/services/latticeStrength'
import type {LighteningOptions} from '../src/services/solidLightening'

const channel:LighteningOptions={pattern:'grid',axis:'x',cell:20,rib:1.35,rim:2,bottom:.6,top:0,seed:42,jitter:0,lineWidth:.45,perimeters:3}
const spatial:LighteningOptions={pattern:'octet',axis:'z',cell:15,rib:3.2,rim:0,bottom:0,top:0,seed:42,jitter:0,lineWidth:.45,perimeters:3,skin:0,openTop:true}
const settings={nozzle:.4,layer:.2,lines:3,skinLayers:3,maxBridge:10,openTop:true}

function cube(s:number){
  const p=[[0,0,0],[s,0,0],[s,s,0],[0,s,0],[0,0,s],[s,0,s],[s,s,s],[0,s,s]]
  const vertices=new Float32Array(p.flatMap(v=>[...v,0,0,1]))
  const indices=new Uint32Array([0,2,1,0,3,2,4,5,6,4,6,7,0,1,5,0,5,4,1,2,6,1,6,5,2,3,7,2,7,6,3,0,4,3,4,7])
  const transform=new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1])
  return {vertices,indices,transform}
}

describe('native lattice printability report',()=>{
  it('ranks Z for channels, flags bridges and emits slicer hints',()=>{
    const r=analyzeLatticePrintability(channel,settings)
    expect(r.preferredAxis).toBe('z')
    expect(r.orientations.map(o=>o.axis)[0]).toBe('z')
    expect(r.issues.map(i=>i.kind)).toEqual(expect.arrayContaining(['bridge','orientation']))
    expect(JSON.parse(latticeSlicerHintsJson(r)).hints.support_enable).toBe(true)
    expect(formatLatticePrintReport(r,'en')).toContain('FDM print')
  })
  it('optimizes channels to Z and bounds the bridge without mutating input',()=>{
    const before=structuredClone(channel)
    const r=optimizeLatticeForPrint(channel,settings)
    expect(r.options.axis).toBe('z')
    expect(r.options.cell-r.options.rib).toBeLessThanOrEqual(settings.maxBridge+1e-9)
    expect(r.changed).toContain('fit_lines_layers')
    expect(channel).toEqual(before)
  })
})

describe('native lattice strength estimate',()=>{
  it('reports Ashby scaling, FDM orthotropy and ranked weak spots',()=>{
    const r=analyzeLatticeStrength(cube(30),spatial,'pla',{case:'compression',forceN:100,safety:2})
    expect(r.loadClass).toBe('stretch')
    expect(r.ashbyExponent).toBe(1)
    expect(r.materialEffective.layerAlignment).toBe(1)
    expect(r.weakSpots.length).toBeGreaterThan(0)
    expect(r.weakSpots.every((s,i,a)=>i===0||a[i-1].score>=s.score)).toBe(true)
    expect(formatLatticeStrengthReport(r,'ru')).toContain('Сопромат')
    const markers=weakSpotMarkerMeshes(r.weakSpots,1)
    expect(markers).toHaveLength(r.weakSpots.length)
    expect(markers[0].entityId).toMatch(/^entity:weak-spot\//)
  })
  it('exposes nominal presets and structure classes; rejects bad loads',()=>{
    expect(latticeMaterials().map(m=>m.id)).toContain('nylon')
    expect(latticeStructureHint('grid').load).toBe('bending')
    expect(()=>analyzeLatticeStrength(cube(30),spatial,'pla',{case:'compression',forceN:-1,safety:2})).toThrow()
    expect(()=>analyzeLatticeStrength(cube(30),spatial,'unobtainium',{case:'compression',forceN:1,safety:2})).toThrow()
  })
})

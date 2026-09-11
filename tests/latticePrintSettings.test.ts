import {describe,it,expect} from 'vitest'
import {
 fitLatticeToPrint,
 printBridgeWarning,
 bridgeSpanMm,
 overhangRisk,
 suggestPrintOrientation,
 analyzeLatticePrintability,
 optimizeLatticeForPrint,
 formatLatticePrintReport,
 latticeSlicerHintsJson,
} from '../src/services/latticePrintSettings'
import type {LighteningOptions} from '../src/services/solidLightening'

const o:LighteningOptions={pattern:'web',axis:'x',cell:8,rib:1.35,rim:2,bottom:.1,top:.7,seed:42,jitter:.7,lineWidth:.45,perimeters:3}
const p={nozzle:.6,layer:.25,lines:4,skinLayers:4,maxBridge:5,openTop:true,maxOverhangDeg:50,minFeatureMm:.6,printSpeedMms:45,fanPct:100}

describe('print geometry fitting',()=>{
 it('fits walls and floor, opens top and preserves source options',()=>{const r=fitLatticeToPrint(o,p);expect(r.rib).toBe(2.7);expect(r.bottom).toBe(1);expect(r.top).toBe(0);expect(r.axis).toBe('z');expect(o.axis).toBe('x');expect(fitLatticeToPrint(r,p)).toEqual(r)})
 it('resolves spatial skin and sampling without removing diagonals',()=>{const r=fitLatticeToPrint({...o,pattern:'spatial',skin:1,step:2,diagonals:true},p);expect(r.skin).toBe(2.7);expect(r.step).toBeLessThanOrEqual(r.rib/3);expect(r.diagonals).toBe(true);expect(printBridgeWarning(r,p)).toBe(true)})
 it('fits octet and BCC like other spatial lattices',()=>{for(const pattern of ['octet','bcc'] as const){const r=fitLatticeToPrint({...o,pattern,skin:1,step:2,wallDepth:3.6},p);expect(r.rib).toBe(2.7);expect(r.step).toBeLessThanOrEqual(r.wallDepth!/2);expect(r.skin).toBe(2.7)}})
 it('rejects impossible layer heights and keeps optional shell absent',()=>{expect(()=>fitLatticeToPrint(o,{...p,layer:1})).toThrow();expect(fitLatticeToPrint({...o,pattern:'spatial',skin:0},p).skin).toBe(0)})
})

describe('orientation and printability',()=>{
 it('ranks Z highest for channel lattices even when source axis is X',()=>{
  const ranked=suggestPrintOrientation(o,p)
  expect(ranked[0].axis).toBe('z')
  expect(overhangRisk(o,'z')).toBeLessThan(overhangRisk(o,'x'))
 })
 it('flags bridges, thin walls and builds slicer hints',()=>{
  const report=analyzeLatticePrintability(o,p)
  expect(report.bridgeSpanMm).toBe(bridgeSpanMm(o))
  expect(report.issues.some(i=>i.kind==='bridge'||i.kind==='thin_wall')).toBe(true)
  expect(report.preferredAxis).toBe('z')
  expect(report.slicerHints.wall_loops).toBe(4)
  expect(report.slicerHints.recommended_print_axis).toBe('z')
  const text=formatLatticePrintReport(report,'en')
  expect(text).toMatch(/FDM print/)
  expect(text).toMatch(/Orientations:/)
  const json=JSON.parse(latticeSlicerHintsJson(report))
  expect(json.generator).toMatch(/lattice-print/)
  expect(json.hints.detect_thin_walls).toBe(true)
 })
 it('warns on cooling for fine cells at high speed',()=>{
  const fine={...o,cell:4,rib:1.2,axis:'z' as const}
  const report=analyzeLatticePrintability(fine,{...p,printSpeedMms:100,maxBridge:20,lines:2})
  expect(report.issues.some(i=>i.kind==='cooling')).toBe(true)
 })
 it('marks closed spatial skin as overhang / skin risk',()=>{
  const spatial={...o,pattern:'octet' as const,skin:2.4,openTop:false,axis:'z' as const,cell:12,rib:3.2}
  expect(overhangRisk(spatial,'z')).toBeGreaterThanOrEqual(0.85)
  const report=analyzeLatticePrintability(spatial,{...p,openTop:false,maxBridge:20})
  expect(report.issues.some(i=>i.kind==='skin'||i.kind==='overhang')).toBe(true)
 })
})

describe('optimize for print',()=>{
 it('reorients channels, fits lines and shrinks bridges',()=>{
  const wide={...o,cell:20,rib:1.35,axis:'x' as const}
  const {options,report,changed}=optimizeLatticeForPrint(wide,p,{reorient:true,shrinkBridges:true})
  expect(changed).toContain('reorient_channels_z')
  expect(changed).toContain('fit_lines_layers')
  expect(changed.some(c=>c.startsWith('shrink_cell')||c.startsWith('thicken_rib'))).toBe(true)
  expect(options.axis).toBe('z')
  expect(bridgeSpanMm(options)).toBeLessThanOrEqual(p.maxBridge+1e-6)
  expect(report.preferredAxis).toBe('z')
 })
 it('opens spatial top when print settings request openTop',()=>{
  const spatial={...o,pattern:'bcc' as const,skin:2,openTop:false,cell:12,rib:2}
  const {options,changed}=optimizeLatticeForPrint(spatial,{...p,openTop:true,maxBridge:30},{reorient:true,shrinkBridges:false})
  expect(options.openTop).toBe(true)
  expect(changed).toContain('open_top')
 })
})

import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string,transform=(s:string)=>s) {
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(readFileSync('examples/rush/'+name+'.r','utf8'))).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.at(-1)!.report)
}
function contains(b:number[],x:number){
  expect(b[0]).toBeLessThanOrEqual(x)
  expect(b[1]).toBeGreaterThanOrEqual(x)
}
const a=Math.PI/9
const tip=(pitch:number,outer:number)=>Math.sqrt(outer**2-(pitch*Math.cos(a))**2)
const external=(tip(10,11)+tip(20,21)-30*Math.sin(a))/(Math.PI*Math.cos(a))
const internal=(tip(10,11)-tip(40,39)+30*Math.sin(a))/(Math.PI*Math.cos(a))
it('uses both actual gear definitions for overlap, mounting limits and tooth clearance',async()=>{
  for(const name of ['external-spur-ratio','mounted-spur-ratio','spur-overlap-range']) contains((await run(name)).contactRatio,external)
  contains((await run('internal-spur-ratio')).contactRatio,internal)
  const {clearancesMm}=await run('spur-tip-clearance')
  for(const b of clearancesMm) contains(b,0.25)
  contains((await run('spur-mounting-window')).centresMm,30)
  await expect(run('external-spur-ratio',s=>s.replace('mate = brep_gear(module:1mm','mate = brep_gear(module:2mm'))).rejects.toThrow(/module/i)
},180000)
it('preserves caliper, signed backlash and spinner layout bounds in the edited gear report',async()=>{
  const halfAngle=Math.PI/40-0.03/20
  const chord=await run('spur-tooth-chord')
  contains(chord.chordMm,20*Math.sin(halfAngle))
  contains(chord.depthMm,11-10*Math.cos(halfAngle))
  contains((await run('spur-tooth-thinning')).thinningMm,0.03)
  for(const name of ['external-spur-backlash','internal-spur-backlash','spur-backlash-range'])contains((await run(name)).backlashMm,0.05)
  contains((await run('planet-tip-separation')).gapMm,40)
  expect((await run('spinner-layout-counts')).counts).toEqual([2,3,4,6,8,9,12,18])
},180000)

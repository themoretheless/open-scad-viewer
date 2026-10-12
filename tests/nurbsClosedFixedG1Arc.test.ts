import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepPatchViewportEvidence,readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'

it('owns closed Fixed inverse-arc error at a positive G1 seam with unequal original speeds',async()=>{
 const source=readFileSync('examples/rush/closed-g1-arc-length-fixed-progressive-sweep.r','utf8')
 const document=compileRushFrontend(source).document,before=JSON.stringify(document)
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(readSweepPatchViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toBeNull()
 expect(JSON.stringify(document)).toBe(before)
 await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('max_deviation: 1mm','max_deviation: 0.000000000000000000000000000001mm')).document,{action:'build'}))
  .rejects.toThrow(/continuous retained-patch error/)
 const damaged=source.replace('[4,-4,0],[4,0,0]','[4,4,0],[4,0,0]')
 expect(damaged).not.toBe(source)
 await expect(buildOwnNurbsAsync(compileRushFrontend(damaged).document,{action:'build'})).rejects.toThrow(/continuous retained-patch error is unproved.*arc-length-correspondence-unproved/)
})

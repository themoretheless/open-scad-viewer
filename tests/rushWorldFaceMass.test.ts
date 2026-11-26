import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,before='',after=''){
  const text=readFileSync('examples/rush/'+name+'.r','utf8').replace(before,after)
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
it('retains the common radical and computes the XYZ centroid of a face with a hole',async()=>{
  expect(await build('world-face-mass')).toMatchObject({
    areaScaleSquared:'5/4',signedAreaCoefficient:'3',firstMomentCoefficients:['3','3','15/2'],centroid:['1','1','5/2'],
    centroidalSecondMomentCoefficients:[['5/4','0','5/8'],['0','5/4','0'],['5/8','0','5/16']],
  })
  await expect(build('world-face-mass','[2,2,3]','[2,2,4]')).rejects.toThrow(/projection refused/)
})
it('keeps exact principal values and distinguishes an isotropic square from a tilted face',async()=>{
  const expected={areaScaleSquared:'5/4',midpoint:'45/32',halfGapSquared:'25/1024',isotropic:false}
  expect(await build('principal-face-moments')).toMatchObject(expected)
  const d=await build('principal-face-directions')
  expect(d).toMatchObject({...expected,planeProjector:[['4/5','0','2/5'],['0','1','0'],['2/5','0','1/5']]})
  expect(d.deviator).not.toBeNull()
  const flat=await build('principal-face-directions','[[2,0,3],[2,2,3]]','[[2,0,2],[2,2,2]]')
  expect(flat).toMatchObject({isotropic:true,halfGapSquared:'0',deviator:null})
})

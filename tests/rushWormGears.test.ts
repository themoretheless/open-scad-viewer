import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {readNativeGeometryData} from '../src/kernel/nativeGeometrySnapshot'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

for (const kind of ['worm','wheel']) {
  it(`builds the finite ZA ${kind} from the Rush profile and preserves its original walls`,async()=>{
    const source=readFileSync(`examples/rush/finite-za-${kind}.r`,'utf8')
    const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
    const model=readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
    const report=JSON.parse(scene.measurements![0]!.report)
    expect(model.bodies).toHaveLength(1)
    if (kind === 'wheel') {
      expect(model.edges).toHaveLength(1680)
      expect(model.faces).toHaveLength(722)
    }
    expect(model.shells[0].closed).toBe(true)
    expect(report.wallErrorUpperMm).toBeGreaterThan(0)
    expect(report.wallErrorUpperMm).toBeLessThanOrEqual(1e-4)
    expect(report.solidGeometryCertified).toBe(false)
    if(kind==='worm'){
      const pitch=Math.PI
      const crest=6,root=3.8
      const slope=Math.tan(Math.PI/9)
      const tipWidth=pitch/2-2*slope,baseWidth=pitch/2+2*1.2*slope
      const idealVolume=Math.PI*(tipWidth*crest**2+(pitch-baseWidth)*root**2+(baseWidth-tipWidth)*(crest**2+crest*root+root**2)/3)/pitch
      expect(Math.abs(report.signedVolumeMm3-idealVolume)).toBeLessThan(0.005*idealVolume)
      expect(report.boundaryAgreement).toBe(true)
    }
  },240000)
}

it('releases the native wheel builder when the editor cancels after a completed step', async () => {
  const source = readFileSync('examples/rush/finite-za-wheel.r', 'utf8')
  const document = compileRushFrontend(source).document
  for (let attempt = 0; attempt < 2; attempt++) {
    let checkpoints = 0
    await expect(buildTextNurbsScene(document, 'full', {
      onYield: () => { checkpoints++ },
      shouldAbort: () => checkpoints >= 2,
    })).rejects.toThrow(/cancel/i)
    expect(checkpoints).toBe(2)
  }
}, 180000)

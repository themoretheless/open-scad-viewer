import assert from 'node:assert/strict'

export async function checkSurfaceUiEvidence({evidenceOnly,entry,page,file,source,load,build,assertBuilt}) {
    if(evidenceOnly){
     entry.viewportRenderingQualified=false
     entry.webGpuUnavailable=await page.locator('.no-gpu').isVisible()
    }else{
     await page.locator('.no-gpu').waitFor({state:'hidden',timeout:30000})
     await page.locator('canvas.gpu-canvas').waitFor({state:'visible'})
     entry.assertions.push({case:'viewport-renderer-ready',status:'passed'})
    }
    const patchEvidence=page.getByTestId('sweep-patch-final-evidence')
    await patchEvidence.waitFor({state:'visible'})
    assert.match(await patchEvidence.innerText(),/Область: сохранённые поверхности/)
    assert.match(await patchEvidence.innerText(),/Регулярность поверхностей:\s*(доказана|не доказана)/)
    entry.assertions.push({case:'separate-retained-surface-regularity-presentation',status:'passed'})
    if(file==='decomposed-profile-g1-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/G2 между частями одного профиля:\s*не доказана/)
     assert.match(await patchEvidence.innerText(),/G1 между частями одного профиля:\s*доказана/)
     const corner=source.replace('[1.5mm,0.25mm,0mm]','[1.5mm,0.375mm,0mm]')
     assert.notEqual(corner,source)
     await load(corner);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/G1 между частями одного профиля:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/G1 между частями одного профиля:\s*доказана/)
     entry.assertions.push({case:'decomposition-G1-curvature-jump-corner-refusal-and-restoration',status:'passed'})
    }
    if(file==='decomposed-profile-g2-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/G2 между частями одного профиля:\s*доказана\s*\(31\)/)
     const corner=source.replace('[0.5mm,0mm,0mm]','[0.5mm,0.25mm,0mm]')
     assert.notEqual(corner,source)
     await load(corner);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/G2 между частями одного профиля:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/G2 между частями одного профиля:\s*доказана\s*\(31\)/)
     entry.assertions.push({case:'decomposition-G2-scoped-corner-refusal-and-restoration',status:'passed'})
    }
    if(file==='authored-closed-cartesian-c2-progressive-sweep.r'||file==='authored-closed-nonclamped-c2-progressive-sweep.r'||file==='authored-closed-piecewise-c2-progressive-sweep.r'||file==='authored-closed-c2-progressive-sweep.r'||file==='authored-closed-multispan-c2-progressive-sweep.r'||file==='authored-closed-projective-c2-progressive-sweep.r'||file==='closed-authored-source-bound-progressive-sweep.r'||file==='closed-authored-arc-length-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*доказана/)
     entry.assertions.push({case:'closed-authored-original-frame-C2-separate-presentation',status:'passed'})
    }
    if(file==='closed-conic-direction-frame-c1-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/C1 замкнутого исходного кадра:\s*доказана/)
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*не доказана/)
     const bad=source.replace('[1mm,1mm,0]','[1.0000000000000002mm,1mm,0]')
     assert.notEqual(bad,source)
     await load(bad);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C1 замкнутого исходного кадра:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C1 замкнутого исходного кадра:\s*доказана/)
     assert.match(await patchEvidence.innerText(),/Ошибка sweep-поверхностей относительно исходного переноса профиля:\s*доказана/)
     entry.assertions.push({case:'closed-direction-C1-single-bit-refusal-and-restoration-without-C2-promotion',status:'passed'})
    }
    if(file==='closed-planar-path-frame-c2-progressive-sweep.r'||file==='closed-planar-guided-frame-c2-progressive-sweep.r'||file==='closed-arc-length-guided-frame-c2-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*доказана/)
     const bad=file.includes('guided-frame-c2')?source.replace('[2mm,1mm,1mm]','[2mm,1mm,1.0000000000000002mm]'):source.replace('[3mm,3mm,0]','[3mm,3.0000000000000004mm,0]')
     assert.notEqual(bad,source)
     await load(bad);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*доказана/)
     entry.assertions.push({case:'closed-path-source-C2-single-bit-refusal-and-restoration',status:'passed'})
     if(file==='closed-planar-guided-frame-c2-progressive-sweep.r')assert.match(await patchEvidence.innerText(),/Ошибка sweep-поверхностей относительно исходного переноса профиля:\s*не доказана/)
    }
    if(file==='authored-closed-cartesian-c2-progressive-sweep.r'){
     const bad=source.replace('[-0.28125,0.125,1]','[-0.28124999999999994,0.125,1]')
     assert.notEqual(bad,source)
     await load(bad);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*доказана/)
     entry.assertions.push({case:'cartesian-closed-frame-C2-single-bit-refusal-and-restoration',status:'passed'})
    }
    if(file==='authored-closed-piecewise-c2-progressive-sweep.r'){
     const bad=source.replace('[-0.109375,0.0625,1]','[-0.10937499999999999,0.0625,1]')
     assert.notEqual(bad,source,'The original interior second jet must change by one binary64 step')
     await load(bad);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 замкнутого исходного кадра:\s*доказана/)
     entry.assertions.push({case:'piecewise-original-closed-C2-single-bit-refusal-and-restoration',status:'passed'})
    }
    if(file==='mixed-knots-curved-frenet-progressive-sweep.r'){
     assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*доказана/)
     const bad=source.replace('[0.5mm,0.25mm,0]','[0.5mm,0.25000000000000006mm,0]')
     assert.notEqual(bad,source)
     await load(bad);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*не доказана/)
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*доказана/)
     entry.assertions.push({case:'mixed-original-path-C2-frame-single-bit-refusal-and-restoration',status:'passed'})
    }
    if(['nonaxial-planar-rmf-progressive-sweep.r','arc-length-guided-progressive-sweep.r','arc-length-curved-guided-progressive-sweep.r'].includes(file)){
     assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*доказана/)
     entry.assertions.push({case:'original-frame-C2-separate-presentation',status:'passed'})
    }
    if(file==='corrected-frenet-sweep.r'||file==='corrected-frenet-antiparallel-progressive-sweep.r'){
     const text=await patchEvidence.innerText()
     assert.match(text,/C2 исходного кадра:\s*доказана/)
     assert.match(text,/Ошибка sweep-поверхностей относительно исходного переноса профиля:\s*доказана/)
     const bad=file==='corrected-frenet-antiparallel-progressive-sweep.r'
      ?source.replace('[1mm,-1mm,0]','[1.0000000000000002mm,-1mm,0]')
      :source.replace('[1mm,1mm,0]','[1mm,1mm,0.0000000000000001mm]')
     assert.notEqual(bad,source)
     await load(bad);await build()
     if(file==='corrected-frenet-sweep.r'){
      const refusal=page.locator('.message.error');await refusal.waitFor()
      entry.correctedSpatialRefusal=await refusal.innerText()
      assert.match(entry.correctedSpatialRefusal,/Progressive sweep .* exceeds /)
      assert.equal(await patchEvidence.count(),0,'Spatial refinement refusal must clear the old planar C2 proof')
     }else{
      await assertBuilt()
      assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*не доказана/)
     }
     await load(source);await build();await assertBuilt()
     assert.match(await patchEvidence.innerText(),/C2 исходного кадра:\s*доказана/)
     assert.match(await patchEvidence.innerText(),/Ошибка sweep-поверхностей относительно исходного переноса профиля:\s*доказана/)
     entry.assertions.push({case:file==='corrected-frenet-sweep.r'?'corrected-spatial-perturbation-budget-refusal-and-planar-source-C2-restoration':'corrected-planar-source-C2-and-error-off-plane-refusal-and-restoration',status:'passed'})
    }else if(file.includes('frenet')||file.includes('fixed-normal')){
     assert.match(await patchEvidence.innerText(),/Регулярность исходного кадра:\s*доказана/)
     entry.assertions.push({case:'whole-original-frame-regularity-presentation',status:'passed'})
    }
    if(file!=='closed-planar-guided-frame-c2-progressive-sweep.r'&&(file.startsWith('arc-length-')||file==='oblique-rmf-progressive-sweep.r'||file==='progressive-sweep.r'||file==='affine-progressive-sweep.r'||file.startsWith('closed-')||file==='fixed-progressive-sweep.r'||file==='fixed-normal-progressive-sweep.r'||file==='frenet-progressive-sweep.r'||file==='spatial-frenet-progressive-sweep.r'||file==='spatial-frenet-affine-progressive-sweep.r')){
     const sourceBudget=source.match(/max_deviation:\s*([0-9.]+)mm/)
     assert.ok(sourceBudget,'Surface fixture must declare its error budget')
     const targetBudget=Number(sourceBudget[1])
     assert.ok(Number.isFinite(targetBudget)&&targetBudget>0,'Source error budget must be positive')
     const text=await patchEvidence.innerText()
     assert.match(text,/профиля:\s*доказана/)
     const bound=text.match(/≤([0-9.,eE+-]+)\s*\/\s*([0-9.,eE+-]+)\s*mm/)
     assert.ok(bound,'Surface must show the final retained error and budget')
     assert.equal(Number(bound[2].replace(',','.')),targetBudget,'Refinement must use the source error budget')
     assert.ok(Number(bound[1].replace(',','.'))>=0&&Number(bound[1].replace(',','.'))<=targetBudget,'Final retained error must fit the source budget')
     entry.assertions.push({case:file==='closed-spatial-rmf-affine-certified.r'?'closed-spatial-rmf-configured-0.25mm-evidence':file==='oblique-rmf-progressive-sweep.r'||file==='progressive-sweep.r'||file==='affine-progressive-sweep.r'?'rmf-original-source-error-evidence':file==='fixed-progressive-sweep.r'?'fixed-refined-error-evidence':file==='closed-fixed-normal-full-turn-progressive-sweep.r'?'closed-full-turn-0.01mm-evidence':'closed-refined-two-mm-evidence',status:'passed'})
     // At 257 stations this full-turn fixture has sampled refinement below
     // 0.0006 mm, but its continuous bound is above it. Exercise the actual
     // continuous admission gate rather than an earlier sampled refusal.
     const strictBudget=file==='closed-planar-rmf-full-turn-progressive-sweep.r'?'0.0006':'0.000000000000000000000000000001'
     let strict=source.replace(/max_deviation:\s*[0-9.]+mm/,`max_deviation: ${strictBudget}mm`)
     if(file.startsWith('arc-length-'))strict=strict.replace(/max_sections:\s*(33|17|9)/,'max_sections: 3')
     assert.notEqual(strict,source,'Strict closed fixture must change the error budget')
     await load(strict);await build()
     const refusal=page.locator('.message.error');await refusal.waitFor()
     assert.match(await refusal.innerText(),/continuous retained-patch error/i)
     assert.equal(await page.getByTestId('sweep-patch-final-evidence').count(),0,'Refused source must clear final patch proof')
     await load(source);await build();await assertBuilt()
     await patchEvidence.waitFor({state:'visible'})
     assert.match(await patchEvidence.innerText(),/профиля:\s*доказана/)
     entry.assertions.push({case:file==='oblique-rmf-progressive-sweep.r'||file==='progressive-sweep.r'||file==='affine-progressive-sweep.r'?'rmf-strict-bound-refusal-and-restoration':file==='fixed-progressive-sweep.r'?'fixed-strict-bound-refusal-and-restoration':'closed-strict-bound-refusal-and-restoration',status:'passed'})
    }
    entry.assertions.push({case:'scoped-retained-patch-evidence',status:'passed'})
   
}

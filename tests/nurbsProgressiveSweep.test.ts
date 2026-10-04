import {expect,it} from 'vitest'
import {callNurbsRust} from '../src/services/geometry/nurbs'
import {readFileSync} from 'node:fs'
import {circleNurbsCurve,bezierNurbsCurve,streamProgressiveNurbsProfiles,previewProgressiveNurbsProfiles,progressiveSweepNurbsPatches,progressiveSweepNurbsProfiles,type NurbsScaleLaw} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface,type NurbsSurface} from '../src/services/nurbsSurface'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs,buildOwnNurbsAsync} from '../src/services/modelGraphNurbsKernel'
const law=(a:number,b:number):NurbsScaleLaw=>({degree:1,knots:[2,2,6,6],values:[a,b],weights:[1,1]})
const profile=()=>bezierNurbsCurve([[1,0,0],[2,0,1]],[1,2])
const path=()=>bezierNurbsCurve([[0,0,0],[0,0,10]])
function point(patches:NurbsSurface[],u:number,v:number){
 const patch=patches.find(p=>u>=p.knotsU[p.degreeU]!&&u<=p.knotsU[p.controlPoints.length]!&&v>=p.knotsV[p.degreeV]!&&v<=p.knotsV[p.controlPoints[0]!.length]!)!
 return evaluateNurbsSurface(patch,u,v).point
}

it('simultaneously scales and twists a weighted profile and preserves progressive reports',()=>{
 const result=progressiveSweepNurbsPatches(profile(),path(),law(1,2),law(0,90),{normal:[1,0,0],initialSections:3,maxSections:129,maxDeviation:.001})
 expect(result.report).toMatchObject({accepted:true,continuousBound:false,roundingCertified:false})
 expect(result.levels.length).toBeGreaterThan(1)
 expect(result.levels[0]!.accepted).toBe(false)
 expect(result.patches!.length).toBeGreaterThan(1)
 for(let i=0;i<result.report.sections;i++)for(const u of [0,.13,.5,.87,1]){
  const v=i/(result.report.sections-1),a=2*u/(1+u),angle=Math.PI*v/2
  const p=point(result.patches!,u,v)
  expect(p[0]).toBeCloseTo((1+v)*(1+a)*Math.cos(angle),10)
  expect(p[1]).toBeCloseTo((1+v)*(1+a)*Math.sin(angle),10)
  expect(p[2]).toBeCloseTo(10*v+(1+v)*a,10)
 }
 const refused=progressiveSweepNurbsPatches(profile(),path(),law(1,2),law(0,90),{normal:[1,0,0],initialSections:3,maxSections:5,maxDeviation:1e-6})
 expect(refused.patches).toBeNull()
 expect(refused.report.accepted).toBe(false)
})

it('lowers angular twist laws, defaults and explicit orientation choices with correct dimensions',()=>{
 const source=readFileSync('examples/rush/progressive-sweep.r','utf8')
 const compiled=compileModelGraphText(source),node=compiled.document.nodes.find(n=>n.op==='progressive_sweep')!
 expect(node).toMatchObject({orientation:'rmf',spacing:'parameter',twist:{values:[0,180]},length_tolerance:.001})
 const result=buildOwnNurbs(compiled.document,{action:'build'})
 expect(result.report.error_bound_certified).toBe(false)
 expect(result.report.construction?.[node.id]).toMatchObject({accepted:true,continuousBound:false})
 for(const mode of ['rmf','fixed','fixed_normal'])expect(()=>buildOwnNurbs(compileModelGraphText(source.replace('normal: [1,0,0]','normal: [1,0,0],orientation: "'+mode+'"')).document,{action:'build'})).not.toThrow()
 expect(()=>buildOwnNurbs(compileModelGraphText(source.replace('normal: [1,0,0]','normal: [1,0,0],orientation: "frenet"')).document,{action:'build'})).toThrow(/direction|normal/)
 for(const invalid of [source.replace('180deg','180mm'),source.replace('values: [1,2]','values: [1,2mm]'),source.replace('initial_sections: 5','initial_sections: 5mm'),source.replace('normal: [1,0,0]','normal: [1mm,0,0]'),source.replace('normal: [1,0,0]','normal: [1,0,0],orientation: "invalid"')])expect(()=>compileModelGraphText(invalid)).toThrow()
 const coarse=compileModelGraphText(source.replace('max_sections: 257','max_sections: 5'))
 expect(()=>buildOwnNurbs(coarse.document,{action:'build'})).toThrow(/Progressive sweep sampled refinement/)
})

it('supports closed full-turn twist with matching seam and refuses partial-turn closure',()=>{
 const source=readFileSync('examples/rush/closed-progressive-sweep.r','utf8')
 const graph=compileModelGraphText(source),node=graph.document.nodes.find(n=>n.op==='progressive_sweep')!
 const result=buildOwnNurbs(graph.document,{action:'build'})
 expect(result.report.construction?.[node.id]).toMatchObject({accepted:true,closedPath:true,seamContinuity:'C0'})
 const patches=(result.report.definitions[node.id] as unknown as {patches:NurbsSurface[]}).patches
 for(const u of [0,.13,.5,.87,1])expect(point(patches,u,0)).toEqual(point(patches,u,1))
 expect(()=>buildOwnNurbs(compileModelGraphText(source.replace('360deg','180deg')).document,{action:'build'})).toThrow(/whole turns/)
})

it('uses inverse arc length for both station positions and law progression',()=>{
 const source=readFileSync('examples/rush/arc-length-progressive-sweep.r','utf8')
 const graph=compileModelGraphText(source),node=graph.document.nodes.find(n=>n.op==='progressive_sweep')!
 expect(node).toMatchObject({spacing:'arc_length',initial_sections:3,max_sections:3})
 const result=buildOwnNurbs(graph.document,{action:'build'})
 expect(result.report.construction?.[node.id]).toMatchObject({accepted:true})
 const patches=(result.report.definitions[node.id] as unknown as {patches:NurbsSurface[]}).patches
 for(const v of [0,.13,.5,.87,1]){
  const p=point(patches,0,v)
  expect(Math.abs(p[0]-(1+v))).toBeLessThan(.001)
  expect(Math.abs(p[2]-10*v)).toBeLessThan(.001)
 }
 expect(()=>compileModelGraphText(source.replace('length_tolerance: 0.001mm','length_tolerance: 0.001deg'))).toThrow()
 expect(()=>buildOwnNurbs(compileModelGraphText(source.replace('length_tolerance: 0.001mm','length_tolerance: 0.001mm,length_max_cells: 1')).document,{action:'build'})).toThrow(/inverse arc-length budget/)
})

it('transports multiple profile boundaries on one grid and refuses the entire set on budget failure',()=>{
 const curves=[bezierNurbsCurve([[.01,0,0],[.02,0,0]]),bezierNurbsCurve([[2,0,0],[2,2,0]]),bezierNurbsCurve([[2,2,0],[0,2,0]])]
 const opts={normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:129,maxDeviation:.001}
 const result=progressiveSweepNurbsProfiles(curves,path(),law(1,2),law(0,90),opts)
 expect(result.report.accepted).toBe(true)
 expect(result.report.sections).toBeGreaterThan(progressiveSweepNurbsPatches(curves[0]!,path(),law(1,2),law(0,90),opts).report.sections)
 const [a,b]=result.profilePatchRanges!.slice(1)
 const left=result.patches!.slice(a![0],a![1]),right=result.patches!.slice(b![0],b![1])
 for(let i=0;i<left.length;i++){
  expect(left[i]!.controlPoints.at(-1)).toEqual(right[i]!.controlPoints[0])
  expect(left[i]!.knotsV).toEqual(right[i]!.knotsV)
 }
 const failed=progressiveSweepNurbsProfiles(curves,path(),law(1,2),law(0,90),{...opts,maxSections:3})
 expect(failed.patches).toBeNull()
 expect(failed.profilePatchRanges).toBeNull()
 expect(()=>progressiveSweepNurbsProfiles([],path(),law(1,2),law(0,90),opts)).toThrow(/1..64/)
 const graph=compileModelGraphText(readFileSync('examples/rush/multi-profile-progressive-sweep.r','utf8'))
 expect(graph.document.nodes.find(n=>n.op==='progressive_sweep')!.inputs).toHaveLength(3)
 const built=buildOwnNurbs(graph.document,{action:'build'})
 const node=graph.document.nodes.find(n=>n.op==='progressive_sweep')!
 expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,profilePatchRanges:expect.arrayContaining([expect.any(Array)])})
})

it('combines rational axis scaling, center offset and twist with independent dimensions',()=>{
 const source=readFileSync('examples/rush/affine-progressive-sweep.r','utf8')
 const graph=compileModelGraphText(source),node=graph.document.nodes.find(n=>n.op==='progressive_sweep')!
 const result=buildOwnNurbs(graph.document,{action:'build'})
 expect(result.report.construction?.[node.id]).toMatchObject({accepted:true,continuousBound:false})
 const patches=(result.report.definitions[node.id] as unknown as {patches:NurbsSurface[]}).patches
 const sections=(result.report.construction?.[node.id] as {sections:number}).sections
 for(let i=0;i<sections;i++)for(const u of [0,.13,.5,.87,1]){
  const v=i/(sections-1),a=2*u/(1+u),b=2*v/(1+v),angle=v*Math.PI/2
  const x=(1+v)*(1+b)*(1+a)+v,y=(1+v)*(2-b)*(2+a)-2*v,z=10*v+(1+v)*(3+b)+.5*v
  const p=point(patches,u,v)
  expect(p[0]).toBeCloseTo(Math.cos(angle)*x-Math.sin(angle)*y,10)
  expect(p[1]).toBeCloseTo(Math.sin(angle)*x+Math.cos(angle)*y,10)
  expect(p[2]).toBeCloseTo(z,10)
 }
 expect(()=>compileModelGraphText(source.replace('[[1,2,3]','[[1mm,2,3]'))).toThrow()
 expect(()=>compileModelGraphText(source.replace('1mm,-2mm,0.5mm','1deg,-2mm,0.5mm'))).toThrow()
})

it('transports complete authored frames independent of the guide tangent',()=>{
 const vector=(values:[number,number,number][])=>({degree:1,knots:[4,4,8,8],values,weights:[1,1]})
 const result=progressiveSweepNurbsPatches(bezierNurbsCurve([[1,2,3],[2,2,3]]),path(),law(1,1),law(0,90),{
  normal:[1,0,0],orientation:'authored',frameAxis:vector([[2,0,0],[2,0,0]]),
  frameNormal:vector([[7,1,0],[7,1,0]]),initialSections:3,maxSections:129,maxDeviation:.001,
 })
 expect(result.report.accepted).toBe(true)
 for(let i=0;i<result.report.sections;i++){
  const f=i/(result.report.sections-1),a=f*Math.PI/2
  const actual=point(result.patches!,0,f)
  const expected=[1,2*Math.cos(a)-3*Math.sin(a),10*f+2*Math.sin(a)+3*Math.cos(a)]
  actual.forEach((v,k)=>expect(v).toBeCloseTo(expected[k]!,9))
 }
 expect(result.report.continuousBound).toBe(true)
 const opts={normal:[1,0,0] as [number,number,number],orientation:'authored' as const,
  frameAxis:vector([[0,0,1],[0,0,1]]),frameNormal:vector([[0,0,2],[0,0,2]]),maxDeviation:.001}
 expect(()=>progressiveSweepNurbsProfiles([profile()],path(),law(1,1),law(0,0),opts)).toThrow(/parallel/)
})


it('lowers authored orientation laws through Rush and builds the independent frame',()=>{
 const source=readFileSync('examples/rush/authored-progressive-sweep.r','utf8')
 const compiled=compileModelGraphText(source)
 const node=compiled.document.nodes.find(n=>n.op==='progressive_sweep')!
 expect(node).toMatchObject({orientation:'authored',frame_axis:{values:[[2,0,0],[2,0,0]]}})
 const built=buildOwnNurbs(compiled.document,{action:'build'})
 expect(built.report.construction?.[node.id]).toMatchObject({accepted:true,continuousBound:true,
  authoredFrameRegularity:{regularityCertified:true,continuousBound:false,reason:null}})
 expect(()=>buildOwnNurbs(compileModelGraphText(source.replace(/frame_normal: \{[^\n]+\n/, '')).document,{action:'build'})).toThrow(/frame_axis and frame_normal/)
 expect(()=>compileModelGraphText(source.replace('[[2,0,0],[2,0,0]]','[[2mm,0,0],[2mm,0,0]]'))).toThrow()
})

it('transports spatial orientation rails through public WASM with independent domains',()=>{
 const guide=bezierNurbsCurve([[1,0,0],[0,1,10]],[1,2])
 const result=progressiveSweepNurbsPatches(bezierNurbsCurve([[1,0,0],[2,0,1]]),path(),law(1,2),law(0,0),{
  normal:[1,0,0],orientationGuide:guide,initialSections:3,maxSections:257,maxDeviation:.001,
 })
 expect(result.report.accepted).toBe(true)
 for(let i=0;i<result.report.sections;i++){
  const f=i/(result.report.sections-1),x=(1-f)/(1+f),y=2*f/(1+f),length=Math.hypot(x,y)
  const actual=point(result.patches!,0,f)
  actual.forEach((v,k)=>expect(v).toBeCloseTo([(1+f)*x/length,(1+f)*y/length,10*f][k]!,9))
 }
 expect(()=>progressiveSweepNurbsProfiles([profile()],path(),law(1,1),law(0,0),{
  normal:[1,0,0],orientationGuide:path(),maxDeviation:.001,
 })).toThrow(/nonzero|parallel/)
 const arc=progressiveSweepNurbsPatches(bezierNurbsCurve([[1,0,0],[2,0,0]]),path(),law(1,1),law(0,0),{
  normal:[1,0,0],orientationGuide:bezierNurbsCurve([[1,0,0],[1,0,10]],[1,4]),
  spacing:'arc_length',lengthTolerance:.001,lengthMaxCells:100000,initialSections:3,maxSections:5,maxDeviation:.001,
 })
 expect(arc.report.accepted).toBe(true)
 expect(arc.report.lengthResidualUpper).toBeLessThanOrEqual(.001)
 expect(point(arc.patches!,0,.5)[2]).toBeCloseTo(5,2)
 expect(()=>progressiveSweepNurbsProfiles([profile()],path(),law(1,1),law(0,0),{
  normal:[1,0,0],orientationGuide:guide,spacing:'arc_length',lengthTolerance:1e-8,lengthMaxCells:1,maxDeviation:.001,
 })).toThrow(/budget|WorkLimit/)
})


it('resolves a Rush orientation rail as a graph dependency',()=>{
 const source=readFileSync('examples/rush/guided-progressive-sweep.r','utf8')
 const graph=compileModelGraphText(source).document
 const node=graph.nodes.find(n=>n.op==='progressive_sweep')!
 expect(node).toHaveProperty('orientation_guide')
 expect(buildOwnNurbs(graph,{action:'build'}).report.construction?.[node.id]).toMatchObject({accepted:true})
 expect(()=>compileModelGraphText(source.replace('orientation_guide: rail','orientation_guide: missing'))).toThrow()
})

it('fits a common contact anchor across public multi-profile surfaces',()=>{
 const guide=bezierNurbsCurve([[2,0,0],[0,4,10]])
 const profiles=[bezierNurbsCurve([[0,0,0],[2,0,0]]),bezierNurbsCurve([[0,0,0],[1,0,0]],[1,2])]
 const opts={normal:[1,0,0] as [number,number,number],orientationGuide:guide,contactAnchor:{profileIndex:0,parameter:1},maxDeviation:.001,initialSections:3,maxSections:129}
 const result=progressiveSweepNurbsProfiles(profiles,path(),law(1,3),law(0,0),opts)
 expect(result.report.accepted).toBe(true)
 for(const f of [0,.5,1])for(const [index,ratio] of [[0,1],[1,.5]]){
  const [a,b]=result.profilePatchRanges![index!]!
  const actual=point(result.patches!.slice(a,b),1,f)
  actual.forEach((v,k)=>expect(v).toBeCloseTo([2*(1-f)*ratio!,4*f*ratio!,10*f][k]!,9))
 }
 expect(()=>progressiveSweepNurbsProfiles(profiles,path(),law(1,1),law(0,5),opts)).toThrow(/zero twist/)
 expect(()=>progressiveSweepNurbsProfiles(profiles,path(),law(1,1),law(0,0),{...opts,contactAnchor:{profileIndex:2,parameter:1}})).toThrow(/index/)
})


it('refuses incomplete contact anchors at the JSON boundary',()=>{
 const base={profile:profile(),path:path(),scale:bezierNurbsCurve([[1,0,0],[1,0,0]]),
  twist:bezierNurbsCurve([[0,0,0],[0,0,0]]),normal:[1,0,0],orientation:'rmf',spacing:'parameter',
  initial_sections:3,max_sections:5,max_deviation:.001}
 expect(()=>callNurbsRust('surface_progressive_sweep',{...base,contact_parameter:1})).toThrow(/requires orientation guide/)
 expect(()=>callNurbsRust('surface_progressive_sweep',{...base,contact_profile:0})).toThrow(/requires contact parameter/)
})


it('lowers shared contact anchors from Rush and rejects incomplete/dimensional inputs',()=>{
 const source=readFileSync('examples/rush/contact-progressive-sweep.r','utf8')
 const graph=compileModelGraphText(source).document
 const node=graph.nodes.find(n=>n.op==='progressive_sweep')!
 expect(node).toMatchObject({contact_profile:0,contact_parameter:0})
 expect(buildOwnNurbs(graph,{action:'build'}).report.construction?.[node.id]).toMatchObject({accepted:true})
 expect(()=>compileModelGraphText(source.replace('contact_parameter: 0','contact_parameter: 0mm'))).toThrow()
 expect(()=>buildOwnNurbs(compileModelGraphText(source.replace('orientation_guide: rail,','').replace(/rail =[^\n]+\n/,'')).document,{action:'build'})).toThrow(/Contact anchor requires/)
})


it('requests bounded preview levels through WASM without promoting an unaccepted preview',()=>{
 const options={normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:129,maxDeviation:.001}
 const profiles=[profile(),profile()]
 const full=progressiveSweepNurbsProfiles(profiles,path(),law(1,2),law(0,90),options)
 const preview=previewProgressiveNurbsProfiles(profiles,path(),law(1,2),law(0,90),options,5)
 expect(preview.preview).toBe(true)
 expect(preview.report.accepted).toBe(false)
 expect(preview.patches.length).toBeGreaterThan(0)
 expect(preview.profilePatchRanges).toHaveLength(2)
 expect(preview.report).toEqual(full.levels.find(l=>l.sections===5))
 const accepted=previewProgressiveNurbsProfiles(profiles,path(),law(1,2),law(0,90),options,full.report.sections)
 expect(accepted.report).toEqual(full.report)
 expect(accepted.patches).toEqual(full.patches)
 for(const count of [2,130])expect(()=>previewProgressiveNurbsProfiles(profiles,path(),law(1,2),law(0,90),options,count)).toThrow(/outside configured/)
})


it('preserves authored, affine, guide and contact settings in single-level requests',()=>{
 const vector=(value:[number,number,number])=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
 const base={normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:129,maxDeviation:.001}
 const guide=bezierNurbsCurve([[1,0,0],[2,0,10]])
 const cases=[base,{...base,axisScale:vector([2,3,4]),centerLaw:vector([.5,1,0])},
  {...base,orientation:'authored' as const,frameAxis:vector([0,0,1]),frameNormal:vector([0,1,0])},
  {...base,orientationGuide:guide},
  {...base,orientationGuide:guide,contactAnchor:{parameter:0}}]
 for(const options of cases){
  const full=progressiveSweepNurbsProfiles([profile()],path(),law(1,2),law(0,0),options)
  const preview=previewProgressiveNurbsProfiles([profile()],path(),law(1,2),law(0,0),options,full.report.sections)
  expect(preview.report).toEqual(full.report)
  expect(preview.patches).toEqual(full.patches)
 }
})


it('streams the same refinement history and accepted geometry as synchronous construction',async()=>{
 const options={normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:129,maxDeviation:.001}
 const curves=[profile()],guide=path(),scale=law(1,2),twist=law(0,90)
 const expected=progressiveSweepNurbsProfiles(curves,guide,scale,twist,options)
 const stream=streamProgressiveNurbsProfiles(curves,guide,scale,twist,options)
 const reports=[]
 for(;;){
  const next=await stream.next()
  if(next.done){expect(next.value).toEqual(expected);break}
  expect(next.value.preview).toBe(true)
  reports.push(next.value.report)
 }
 expect(reports).toEqual(expected.levels)
})

it('returns no construction patches when streaming exhausts its section budget',async()=>{
 const stream=streamProgressiveNurbsProfiles([profile()],path(),law(1,2),law(0,90),
  {normal:[1,0,0],initialSections:3,maxSections:4,maxDeviation:1e-10})
 expect((await stream.next()).value).toMatchObject({preview:true,report:{sections:3,accepted:false}})
 expect((await stream.next()).value).toMatchObject({preview:true,report:{sections:4,accepted:false}})
 expect(await stream.next()).toMatchObject({done:true,value:{patches:null,profilePatchRanges:null,report:{accepted:false}}})
})

it('honors cancellation before work, while scheduled, and between yielded levels',async()=>{
 const options={normal:[1,0,0] as [number,number,number],initialSections:3,maxSections:129,maxDeviation:.001}
 const create=(signal:AbortSignal)=>streamProgressiveNurbsProfiles([profile()],path(),law(1,2),law(0,90),options,{signal})
 const before=new AbortController();before.abort(new Error('before'))
 await expect(create(before.signal).next()).rejects.toThrow('before')
 const scheduled=new AbortController(),pending=create(scheduled.signal).next()
 scheduled.abort(new Error('scheduled'))
 await expect(pending).rejects.toThrow('scheduled')
 const between=new AbortController(),stream=create(between.signal)
 expect((await stream.next()).value).toMatchObject({preview:true,report:{accepted:false}})
 between.abort(new Error('between'))
 await expect(stream.next()).rejects.toThrow('between')
})


it('streams reachable Rush sweep levels while preserving synchronous graph outputs',async()=>{
 const source=readFileSync('examples/rush/progressive-sweep.r','utf8')
 const document=compileModelGraphText(source).document
 const expected=buildOwnNurbs(document,{action:'build'}),reports:unknown[]=[]
 const result=await buildOwnNurbsAsync(document,{action:'build'},{onSweepPreview:(id,preview)=>{
  expect(document.nodes.find(n=>n.id===id)?.op).toBe('progressive_sweep')
  expect(preview.preview).toBe(true)
  reports.push(preview.report)
 }})
 expect(result).toEqual(expected)
 const sweepId=document.nodes.find(n=>n.op==='progressive_sweep')!.id
 expect(reports).toEqual((expected.report.construction![sweepId] as {levels:unknown[]}).levels)
})

it('cancels the Rush parser between sweep levels and leaves its serialized queue usable',async()=>{
 const {parseOpenSCAD,AbortedError}=await import('../src/services/openscadParser')
 const source=readFileSync('examples/rush/progressive-sweep.r','utf8')
 let cancelled=false,levels=0
 await expect(parseOpenSCAD(source,{shouldAbort:()=>cancelled,onSweepPreview:(_id,preview)=>{
  levels++
  expect(preview.report.accepted).toBe(false)
  cancelled=true
 }})).rejects.toBeInstanceOf(AbortedError)
 expect(levels).toBe(1)
 const recovered=await parseOpenSCAD(source)
 expect(recovered.meshes.length).toBeGreaterThan(0)
})

it('awaits preview consumers and propagates their failures without publishing a graph result',async()=>{
 const document=compileModelGraphText(readFileSync('examples/rush/progressive-sweep.r','utf8')).document
 let levels=0
 await expect(buildOwnNurbsAsync(document,{action:'build'},{onSweepPreview:async()=>{
  levels++
  await Promise.resolve()
  throw new Error('consumer failed')
 }})).rejects.toThrow('consumer failed')
 expect(levels).toBe(1)
})


it('preserves closed, multi-profile, authored, affine, guided and contact graph semantics asynchronously',async()=>{
 for(const name of ['closed-progressive-sweep','arc-length-progressive-sweep','multi-profile-progressive-sweep','authored-progressive-sweep','affine-progressive-sweep','guided-progressive-sweep','contact-progressive-sweep']){
  const document=compileModelGraphText(readFileSync(`examples/rush/${name}.r`,'utf8')).document
  expect(await buildOwnNurbsAsync(document,{action:'build'}),name).toEqual(buildOwnNurbs(document,{action:'build'}))
 }
})

it('streams refused graph previews without promoting them into a scene',async()=>{
 const source=readFileSync('examples/rush/progressive-sweep.r','utf8').replace('max_sections: 257','max_sections: 5')
 const document=compileModelGraphText(source).document
 let levels=0
 await expect(buildOwnNurbsAsync(document,{action:'build'},{onSweepPreview:(_id,preview)=>{
  expect(preview.report.accepted).toBe(false)
  expect(preview.patches.length).toBeGreaterThan(0)
  levels++
 }})).rejects.toThrow(/Progressive sweep sampled refinement/)
 expect(levels).toBe(1)
})


it('places scene previews through nested affine transforms in graph order',async()=>{
 const {progressiveScenePreviewMapper}=await import('../src/services/modelGraphTextScene')
 const {transformNurbsSurfacePatches}=await import('../src/services/nurbsSurface')
 const document=compileModelGraphText(readFileSync('examples/rush/progressive-sweep.r','utf8')).document
 const sweepId=document.nodes.find(n=>n.op==='progressive_sweep')!.id
 const display=document.nodes.find(n=>n.op==='nurbs_patches_tessellate')!
 const translated=[[1,0,0,20],[0,1,0,0],[0,0,1,0],[0,0,0,1]]
 const rotated=[[0,-1,0,0],[1,0,0,0],[0,0,1,0],[0,0,0,1]]
 const modified={...document,nodes:[...document.nodes.filter(n=>n.id!==display.id),
  {id:'placed',op:'transform',input:sweepId,matrix:translated},
  {id:'rotated',op:'transform',input:'placed',matrix:rotated},
  {...display,input:'rotated'}]}
 const mapper=progressiveScenePreviewMapper(modified)
 let levels=0
 await buildOwnNurbsAsync(modified,{action:'build'},{onSweepPreview:(id,preview)=>{
  const placed=mapper(id,preview)!
  expect(placed.patches).toEqual(transformNurbsSurfacePatches(transformNurbsSurfacePatches(preview.patches,translated),rotated))
  expect(placed.report).toBe(preview.report)
  levels++
 }})
 expect(levels).toBeGreaterThan(1)
 const unsupported={...document,root:'thick',nodes:[...document.nodes,{id:'thick',op:'thicken',input:document.root,vector:[0,0,1]}]}
 const reject=progressiveScenePreviewMapper(unsupported)
 await buildOwnNurbsAsync(document,{action:'build'},{onSweepPreview:(id,preview)=>{
  expect(reject(id,preview)).toBeNull()
 }})
})


it('continues corrected Frenet through an inflection without rotating the binormal profile offset',async()=>{
 const source=readFileSync('examples/rush/corrected-frenet-sweep.r','utf8')
 const document=compileModelGraphText(source).document
 const built=await buildOwnNurbsAsync(document,{action:'build'})
 const node=document.nodes.find(n=>n.op==='progressive_sweep')!
 expect(built.report.construction![node.id]).toMatchObject({accepted:true,continuousBound:false})
 const patches=(built.report.definitions[node.id] as unknown as {patches:NurbsSurface[]}).patches
 for(const v of [0,.25,.5,.75,1]){
  expect(point(patches,0,v)[2]).toBeCloseTo(1,10)
  expect(point(patches,1,v)[2]).toBeCloseTo(2,10)
 }
 expect(()=>buildOwnNurbs(compileModelGraphText(source.replace('corrected_frenet','frenet')).document,{action:'build'})).toThrow(/normal/)
})


it('retains corrected Frenet closure and continues a C1 straight-to-curved knot through WASM',()=>{
 const guide=circleNurbsCurve([0,0,0],[0,0,1],5)
 const profile=bezierNurbsCurve([[5,0,-1],[5,0,1]])
 const opts={normal:[0,0,1] as [number,number,number],orientation:'corrected_frenet' as const,
  initialSections:5,maxSections:257,maxDeviation:.01}
 const result=progressiveSweepNurbsProfiles([profile],guide,law(1,1),law(0,360),opts)
 expect(result.report).toMatchObject({accepted:true,closedPath:true,seamContinuity:'C0',continuousBound:false})
 for(const u of [0,.13,.5,.87,1]){
  const start=point(result.patches!,u,0),end=point(result.patches!,u,1)
  expect(Math.hypot(...start.map((x,i)=>x-end[i]!))).toBeLessThan(1e-12)
 }
 expect(()=>progressiveSweepNurbsProfiles([profile],guide,law(1,1),law(0,90),opts)).toThrow(/whole turns/)
 const joined={degree:3,knots:[0,0,0,0,.5,.5,1,1,1,1],
  controlPoints:[[0,0,0],[1,0,0],[2,0,0],[3,0,0],[4,1,0],[5,2,0]],weights:[1,1,1,1,1,1],periodic:false}
 const ribbon=bezierNurbsCurve([[0,0,1],[0,0,2]])
 const preview=previewProgressiveNurbsProfiles([ribbon],joined,law(1,1),law(0,0),{...opts,initialSections:3,maxSections:9},9)
 for(const v of [0,.125,.25,.375,.5,.625,.75,.875,1]){
  expect(point(preview.patches,0,v)[2]).toBeCloseTo(1,10)
  expect(point(preview.patches,1,v)[2]).toBeCloseTo(2,10)
 }
})

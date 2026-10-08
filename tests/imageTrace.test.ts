import {executeSvgJob} from '../src/services/svgWorkerRuntime'
import {readFileSync} from 'node:fs'
import {describe,it,expect} from 'vitest'
import {imageTraceJob,imageTraceSketches,imageTraceColorSketches,imageTraceColorJob} from '../src/services/imageTrace'
import {svgContours} from '../src/services/svgGeometry'
import {xyPlane} from '../src/services/directSketchGeometry'
import {DirectHistory,emptyDirectDocument} from '../src/services/directModeling'
const data='data:image/png;base64,'+readFileSync(new URL('./fixtures/image-trace-donut.png',import.meta.url)).toString('base64')
const settings={widthMm:4,pixelWidth:4,pixelHeight:4,threshold:.5,resolution:128,tolerance:.01,mode:'dark' as const}
describe('native image trace',()=>{
 it('distinguishes dark ink from white and preserves hole boundaries as editable paths',async()=>{
  const job=imageTraceJob(data,settings);if(job.kind!=='contours')throw Error('Expected contours')
  const rings=await svgContours(job.svg,job.options)
  expect(rings).toHaveLength(2)
  const area=(ring:[number,number][])=>ring.reduce((a,p,i)=>{const q=ring[(i+1)%ring.length];return a+(p[0]*q[1]-p[1]*q[0])/2},0)
  expect(rings.map(area).sort((a,b)=>a-b)).toEqual([-4,16])
  const sketches=imageTraceSketches(rings,.01,xyPlane(),'logo')
  expect(sketches.every(s=>s.editablePath?.closed)).toBe(true)
  const history=new DirectHistory();history.commit({...emptyDirectDocument(),sketches})
  expect(history.document.sketches).toHaveLength(2);expect(history.undo().sketches).toHaveLength(0);expect(history.redo().sketches).toHaveLength(2)
 })
 it('supports alpha silhouette without treating opaque white as a hole',async()=>{
  const job=imageTraceJob(data,{...settings,mode:'alpha'});if(job.kind!=='contours')throw Error('Expected contours')
  expect(await svgContours(job.svg,job.options)).toHaveLength(1)
 })
 it('rejects external image URLs, invalid dimensions and excessive contour count',()=>{
  expect(()=>imageTraceJob('https://example.com/image.png',settings)).toThrow()
  expect(()=>imageTraceJob(data,{...settings,pixelHeight:0})).toThrow()
  expect(()=>imageTraceSketches(Array.from({length:201},()=>[[0,0],[1,0],[0,1]]),.1,xyPlane(),'logo')).toThrow(/200/)
 })
})

it('preserves color layers in editable contours and document history',()=>{
 const layers=[{color:[255,0,0] as [number,number,number],contours:[[[0,0],[2,0],[2,2],[0,2]]] as [number,number][][]},{color:[0,0,255] as [number,number,number],contours:[[[3,0],[4,0],[4,1],[3,1]]] as [number,number][][]}]
 const sketches=imageTraceColorSketches(layers,.01,xyPlane(),'Color')
 expect(sketches.map(s=>s.traceColor)).toEqual(['#ff0000','#0000ff']);expect(sketches.every(s=>s.editablePath?.closed)).toBe(true)
 const history=new DirectHistory();history.commit({...emptyDirectDocument(),sketches});expect(history.document.sketches[1].traceColor).toBe('#0000ff');expect(history.undo().sketches).toHaveLength(0)
})

it('runs raster color tracing through the worker and preserves nested color boundaries',async()=>{
 const data='data:image/png;base64,'+readFileSync(new URL('./fixtures/image-trace-color.png',import.meta.url)).toString('base64')
 const options={...settings,pixelWidth:32,pixelHeight:32}
 const result=await executeSvgJob(imageTraceColorJob(data,options,2,true,.01))
 expect(result.layers).toHaveLength(2)
 const red=result.layers!.find(l=>l.color[0]===255)!,blue=result.layers!.find(l=>l.color[2]===255)!
 expect(red.contours).toHaveLength(2);expect(blue.contours).toHaveLength(1)
 expect(imageTraceColorSketches(result.layers!,.01,xyPlane(),'Native colors')).toHaveLength(3)
 const white=await executeSvgJob(imageTraceColorJob(data,options,3,false,.01));expect(white.layers).toHaveLength(3)
})

import {expect,it} from 'vitest'
import {lineNurbsCurve,polylineNurbsCurve,circleNurbsCurve,circleNurbsArc} from '../src/services/nurbsConstructors'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {readFileSync} from 'node:fs'

it('preserves affine paths and explicit clamped closure through WASM',()=>{
  const start:[number,number,number]=[2,-1,3],end:[number,number,number]=[8,5,-2]
  const c=lineNurbsCurve(start,end)
  for(const t of [0,0.31,0.8,1]){
    const p=evaluateNurbsCurve(c,t).point
    for(let a=0;a<3;a++)expect(p[a]).toBeCloseTo((1-t)*start[a]!+t*end[a]!,12)
  }
  const closed=polylineNurbsCurve([start,end,[-4,9,7]],true)
  expect(closed.controlPoints).toHaveLength(4)
  expect(closed.controlPoints[0]).toEqual(closed.controlPoints.at(-1))
  expect(evaluateNurbsCurve(closed,1/3).point).toEqual(end)
  expect(()=>lineNurbsCurve(start,start)).toThrow()
  expect(()=>polylineNurbsCurve([start,end],true)).toThrow()
})

it('preserves radius and oriented circle planes',()=>{
  const center:[number,number,number]=[2,-3,5]
  const normal:[number,number,number]=[1,2,3]
  const full=circleNurbsCurve(center,normal,7)
  expect(full.controlPoints[0]).toEqual(full.controlPoints.at(-1))
  for(const sweep of [-280,75,360]){
    const c=circleNurbsArc(center,normal,7,25,sweep)
    for(const t of [0,0.21,0.7,1]){
      const p=evaluateNurbsCurve(c,t).point.map((x,i)=>x-center[i]!)
      expect(p.reduce((sum,x)=>sum+x*x,0)).toBeCloseTo(49,11)
      expect(p.reduce((sum,x,i)=>sum+x*normal[i]!,0)).toBeCloseTo(0,11)
    }
  }
  expect(()=>circleNurbsCurve(center,[0,0,0],7)).toThrow()
  expect(()=>circleNurbsCurve(center,normal,0)).toThrow()
})

it('keeps line sites as lengths, circle normals scalar and path closure boolean in Rush',()=>{
  const line=readFileSync('examples/rush/line-extrusion.r','utf8')
  expect(compileModelGraphText(line).document.nodes.find(n=>n.op==='line_curve')).toMatchObject({start:[-10,0,0],end:[10,5,3]})
  expect(()=>compileModelGraphText(line.replace('-10mm','-10deg'))).toThrow()
  const polyline=readFileSync('examples/rush/polyline-extrusion.r','utf8')
  expect(compileModelGraphText(polyline).document.nodes.find(n=>n.op==='polyline_curve')).toMatchObject({closed:true})
  expect(compileModelGraphText(polyline.replace('closed: true','closed: false')).document.nodes.find(n=>n.op==='polyline_curve')).toMatchObject({closed:false})
  const circle=readFileSync('examples/rush/circle-extrusion.r','utf8')
  expect(()=>compileModelGraphText(circle.replace('normal: [0,0,1]','normal: [0mm,0mm,1mm]'))).toThrow()
})

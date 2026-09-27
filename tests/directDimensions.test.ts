import {describe,it,expect} from 'vitest'
import { directCornerTool } from '../src/services/directProfileTools'
import {bridgeNurbsCurves,sketchDimensions,editSketchDimension} from '../src/services/directDimensions'
import {evaluateNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {emptyDirectDocument,parseDirectDocument,DirectHistory,type DirectSketch} from '../src/services/directModeling'
const curve=(x:number):NurbsCurve=>({degree:2,knots:[0,0,0,1,1,1],controlPoints:[[x,0,0],[x+1,2,0],[x+3,1,0]],weights:[1,2,1]})
const norm=(v:number[])=>Math.hypot(...v)
const unit=(v:number[])=>v.map(x=>x/norm(v))
const curvature=(d:number[],dd:number[])=>{const l=norm(d),dot=d.reduce((s,x,i)=>s+x*dd[i],0);return dd.map((x,i)=>(x-d[i]*dot/(l*l))/(l*l))}
describe('G2 bridges and sketch dimensions',()=>{
  it.each(['start','end'] as const)('matches rational endpoint tangent and curvature from %s',endA=>{
    for(const endB of ['start','end'] as const){
      const a=curve(0),b=curve(10),bridge=bridgeNurbsCurves(a,b,endA,endB,.7)
      expect(bridge.degree).toBe(5)
      for(const [source,end,t,sign] of [[a,endA,0,endA==='end'?1:-1],[b,endB,1,endB==='start'?1:-1]] as const){
        const expected=evaluateNurbsCurve(source,end==='end'?1:0),actual=evaluateNurbsCurve(bridge,t)
        expected.point.forEach((x,i)=>expect(actual.point[i]).toBeCloseTo(x,9))
        unit(expected.d1!).forEach((x,i)=>expect(unit(actual.d1!)[i]).toBeCloseTo(x*sign,8))
        curvature(expected.d1!,expected.d2!).forEach((x,i)=>expect(curvature(actual.d1!,actual.d2!)[i]).toBeCloseTo(x,8))
      }
    }
  })
  it('rejects degenerate bridges and invalid tension',()=>{
    expect(()=>bridgeNurbsCurves(curve(0),curve(0),'end','end')).toThrow()
    expect(()=>bridgeNurbsCurves(curve(0),curve(10),'end','start',0)).toThrow()
  })
  const sketch:DirectSketch={id:'s',name:'Dimensioned',closed:false,points:[[0,0],[10,0],[10,10]],dimensions:[{kind:'length',a:0,b:1},{kind:'angle',a:0,b:1,c:2}]}
  it('edits length and signed angle, retaining annotations through save and undo',()=>{
    expect(sketchDimensions(sketch).measurements.map(m=>m.value)).toEqual([10,-90])
    const length=editSketchDimension(sketch,0,20)
    expect(length.points[1]).toEqual([20,0])
    const edited=editSketchDimension(sketch,1,-60)
    expect(sketchDimensions(edited).measurements[1].value).toBeCloseTo(-60,8)
    expect(norm(edited.points[2].map((v,i)=>v-edited.points[1][i]))).toBeCloseTo(10,8)
    const doc=emptyDirectDocument();doc.sketches.push(sketch)
    const history=new DirectHistory(doc);const next=history.document;next.sketches[0]=edited;history.commit(next)
    expect(parseDirectDocument(JSON.stringify(history.document)).sketches[0].dimensions).toEqual(sketch.dimensions)
    history.undo();expect(history.document.sketches[0].points).toEqual(sketch.points)
  })
  it('solves multiple linear dimensions together',()=>{
    const constrained:DirectSketch={id:'r',name:'Rectangle',closed:false,points:[[0,0],[10,0],[10,10]],dimensions:[{kind:'length',a:0,b:1},{kind:'length',a:1,b:2}]}
    const edited=editSketchDimension(constrained,0,20)
    expect(Math.hypot(edited.points[1][0]-edited.points[0][0],edited.points[1][1]-edited.points[0][1])).toBeCloseTo(20,6)
    expect(Math.hypot(edited.points[2][0]-edited.points[1][0],edited.points[2][1]-edited.points[1][1])).toBeCloseTo(10,5)
  })
  it('drops references when a corner is replaced by new vertices',()=>{
    const square:DirectSketch={id:'sq',name:'Square',closed:true,points:[[0,0],[20,0],[20,20],[0,20]],dimensions:[{kind:'length',a:0,b:1}]}
    expect(directCornerTool(square,0,2,'fillet').dimensions).toBeUndefined()
    expect(square.dimensions).toHaveLength(1)
  })
  it('rejects invalid references and edits; reports undefined angles without NaN',()=>{
    expect(()=>sketchDimensions({...sketch,dimensions:[{kind:'length',a:0,b:99}]})).toThrow()
    expect(()=>editSketchDimension(sketch,0,-1)).toThrow()
    expect(()=>editSketchDimension(sketch,1,181)).toThrow()
    expect(sketchDimensions({...sketch,points:[[0,0],[0,0],[10,0]]}).measurements[1].value).toBeNull()
  })
  it('measures and edits analytic radius and diameter',()=>{
    const analytic:DirectSketch={id:'c',name:'Circle',closed:true,points:[[5,0],[0,5],[-5,0],[0,-5]],analytic:{kind:'circle',center:[0,0],radius:5,start:0,sweep:360},dimensions:[{kind:'radius'},{kind:'diameter'}]}
    expect(sketchDimensions(analytic).measurements.map(m=>m.value)).toEqual([5,10])
    const edited=editSketchDimension(analytic,1,20)
    expect(edited.analytic?.radius).toBe(10)
    expect(sketchDimensions(edited).measurements.map(m=>m.value)).toEqual([10,20])
  })
})

import type {NurbsBrep} from '../../src/services/geometry/brep'
import type {NurbsCurve} from '../../src/services/nurbsCurve'
export function rationalRoof(base:NurbsBrep):NurbsBrep {
 const m=structuredClone(base),s=m.faces[1].surface
 s.degreeU=2;s.degreeV=2;s.knotsU=[0,0,0,1,1,1];s.knotsV=[0,0,0,1,1,1]
 s.controlPoints=Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>[i,j,i===1&&j===1?3:2]))
 s.weights=[[1,1,1],[1,2,1],[1,1,1]];delete m.topologyIds;return m
}
export const planarCurve=(points:number[][]):NurbsCurve=>({degree:points.length-1,controlPoints:points,weights:points.map(()=>1),knots:[...points.map(()=>0),...points.map(()=>1)],periodic:false})
export const constantHalf=()=>planarCurve([[.5,0],[.5,0]])
export const squareParameter=()=>planarCurve([[0,0],[0,0],[1,0]])
export const retracedParameter=()=>planarCurve([[1,0],[-1,0],[1,0]])

export const constantMultispan=():NurbsCurve=>({degree:1,controlPoints:[[.5,0],[.5,0],[.5,0]],weights:[1,1,1],knots:[0,0,.5,1,1],periodic:false})

/** Authored test input: one polynomial roof retained as two UV triangles. */
export function triangularRoof(base:NurbsBrep):NurbsBrep {
 const m=rationalRoof(base);m.faces[1].surface.weights=[[1,1,1],[1,1,1],[1,1,1]]
 const outer=m.faces[1].outer,old=structuredClone(m.loops[outer].coedges)
 const start=m.edges[old[0].edge].vertices[Number(old[0].reversed)],end=m.edges[old[1].edge].vertices[Number(!old[1].reversed)]
 const edge=m.edges.length;m.edges.push({degenerate:false,vertices:[start,end],curve:{degree:4,knots:[0,0,0,0,0,1,1,1,1,1],weights:[1,1,1,1,1],controlPoints:[[0,0,2],[.5,.5,2],[1,1,8/3],[1.5,1.5,2],[2,2,2]],periodic:false}})
 m.loops[outer].coedges=[old[0],old[1],{edge,reversed:true,pcurve:planarCurve([[1,1],[0,0]])}]
 const second=m.loops.length;m.loops.push({coedges:[{edge,reversed:false,pcurve:planarCurve([[0,0],[1,1]])},old[2],old[3]]})
 const face=m.faces.length;m.faces.push({surface:structuredClone(m.faces[1].surface),outer:second,holes:[]});m.shells[0].faces.push({face,reversed:false});return m
}

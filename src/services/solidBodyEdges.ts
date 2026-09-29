import type {DirectBody} from './directModeling'
import {solidTopology} from './directSolidTools'
import {sampleSolidNurbsCurve} from './solidNurbs'

/** Exact bodies expose their authored topology, never tessellation diagonals. */
export function solidBodyEdges(body:DirectBody) {
 if(body.brep)return body.brep.edges.map((edge,index)=>({
  index,id:body.brep!.topologyIds?.edges[index]??`${body.id}:edge:${index}`,
  a:edge.vertices[0],b:edge.vertices[1],
  points:edge.curve.degree===1?edge.curve.controlPoints.map(point=>[...point]):sampleSolidNurbsCurve(edge.curve,24),
 }))
 return solidTopology(body.mesh).edges.map((edge,index)=>({
  index,id:`${body.id}:mesh-edge:${index}`,a:edge.a,b:edge.b,
  points:[edge.a,edge.b].map(vertex=>Array.from(body.mesh.positions.slice(vertex*3,vertex*3+3))),
 }))
}

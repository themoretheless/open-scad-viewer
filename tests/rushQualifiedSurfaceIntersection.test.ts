import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const example=()=>readFileSync('examples/rush/inspect-closed-surface-intersection.r','utf8')

async function inspect(source:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.find(row=>row.operation==='inspect_surface_intersection')!.report)
}

it('reports the original closed intersection with four enclosures and paired UV trims',async()=>{
  const report=await inspect(example())
  expect(report).toMatchObject({kind:'ClosedEllipse',coversEntireSurfaces:true})
  expect(report.branches).toEqual([{closed:true,edges:[0,1,2,3]}])
  expect(report.vertices).toHaveLength(4)
  expect(report.edges).toHaveLength(4)
  expect(report.trims).toHaveLength(8)
  for(const {worldMm} of report.vertices){
    expect(worldMm[2][0]).toBeLessThanOrEqual(0)
    expect(worldMm[2][1]).toBeGreaterThanOrEqual(0)
  }
},180000)

it('distinguishes isolated tangency and identical carriers',async()=>{
  const source=example()
  const line=(a:number,b:number,k:number)=>{
    const weights=[0.25,-0.25,0.25]
    const points=weights.map((u,i)=>weights.map((v,j)=>[i/2,j/2,a*u+b*v+k]))
    return 'bowl = nurbs_surface(degree_u:2,degree_v:2,knots_u:[0,0,0,1,1,1],knots_v:[0,0,0,1,1,1],control_points:'+JSON.stringify(points)+',weights:[[1,1,1],[1,1,1],[1,1,1]])'
  }
  const tangent=await inspect(source.replace(/^bowl = .*$/m,line(1,1,0)))
  expect(tangent).toMatchObject({kind:'IsolatedTangentPoint',coversEntireSurfaces:true,edges:[],branches:[]})
  expect(tangent.vertices).toHaveLength(1)
  expect(tangent.vertices[0].singular).toBe(true)
  const coincident=await inspect(source.replace(/^bowl = .*$/m,line(0,0,0)))
  expect(coincident).toMatchObject({kind:'CoincidentPatch',coversEntireSurfaces:true})
},180000)

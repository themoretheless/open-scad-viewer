import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const grid=Array.from({length:16},(_,index)=>[index%4,Math.floor(index/4),0])
const normals=grid.map(()=>[0,0,1])
async function inspect(operation:string,args:string){
  const source=[
    '// @rush/1',
    'base = line_curve(start:[0mm,0mm,0mm],end:[3mm,0mm,0mm])',
    'panel = base.surface_extrude([0mm,3mm,0mm])',
    'result = panel.'+operation+'('+args+')',
    'show result.tessellate(4,4)',
  ].join('\n')
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.find(row=>row.operation===operation)!.report)
}

it('finds the sixteen planar inliers without admitting the off-plane sample',async()=>{
  const points=JSON.stringify([...grid,[0,0,5]])
  const report=await inspect('detect_cloud_primitives','points:'+points+',normals:[],primitive_kind:"plane",tolerance_mm:0.001mm,min_inliers:12,max_iterations:100,max_primitives:1')
  expect(report.siteCount).toBe(17)
  expect(report.primitives).toHaveLength(1)
  expect(report.primitives[0]).toMatchObject({kind:'Plane',inlierIndices:grid.map((_,index)=>index)})
  expect(report.primitives[0].rmsMm).toBeLessThan(1e-8)
},180000)

it('orients inconsistent signs and finds a single flat region on a plane',async()=>{
  const points=JSON.stringify(grid)
  const flipped=JSON.stringify(normals.map((normal,index)=>index%2?normal.map(x=>-x):normal))
  const oriented=await inspect('orient_cloud_normals','points:'+points+',normals:'+flipped)
  expect(oriented).toMatchObject({components:1,flipped:8})
  expect(new Set(oriented.normals.map((normal:number[])=>JSON.stringify(normal))).size).toBe(1)
  const regions=await inspect('segment_point_cloud','points:'+points+',normals:'+JSON.stringify(normals)+',normal_angle_deg:20deg,curvature_threshold:0.01,min_region_size:4,neighbor_count:8')
  expect(regions.regions).toHaveLength(1)
  expect(regions.regions[0].kind).toBe('Planar')
  expect(regions.regions[0].pointIndices.slice().sort((a:number,b:number)=>a-b)).toEqual(grid.map((_,index)=>index))
},180000)

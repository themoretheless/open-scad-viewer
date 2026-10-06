import type {PolygonMesh} from './geometry/polygon'
import {planPolygonMeshToolpaths,sectionPolygonMesh} from './geometry/polygon'
import type {LaserMachineProfile,LaserOperation,LaserPlan} from './geometry/laser'

export interface LaserProcessSettings {
  line:{output:boolean;speedMmMin:number;power:number;passes:number;airAssist:boolean;kerfMm:number;kerfMode:'center'|'part'|'cavity';pathOrder:'preserve'|'nearest'|'inner-first'|'inner-first-nearest'}
  fill:{output:boolean;speedMmMin:number;power:number;passes:number;airAssist:boolean;spacingMm:number;pathOrder:'preserve'|'nearest'}
  offset:[number,number]
}

const translated=(points:readonly (readonly number[])[],offset:readonly [number,number]):[number,number][]=>
  points.map(point=>[point[0]!+offset[0],point[1]!+offset[1]])

/** Build laser Line/Fill operations from one exact horizontal mesh section. */
export function planLaserMeshSection(
  mesh:PolygonMesh,
  zMm:number,
  machine:LaserMachineProfile,
  settings:LaserProcessSettings,
):LaserPlan {
  if(!Number.isFinite(zMm))throw new Error('Laser section Z must be finite.')
  const section=sectionPolygonMesh(mesh,zMm)
  if(!section.contours.length)throw new Error('The selected Z plane has no closed contours.')
  const operations:LaserOperation[]=[]
  operations.push({
    name:'Line',kind:'line',...settings.line,
    paths:section.contours.map(contour=>({points:translated(contour.points,settings.offset),closed:true})),
  })
  if(settings.fill.output){
    const layer=planPolygonMeshToolpaths(mesh,zMm,zMm+0.001,{
      layerHeightMm:0.001,lineWidthMm:0.001,wallCount:1,
      infillSpacingMm:settings.fill.spacingMm,feedrateMmS:1,travelFeedrateMmS:1,
    }).layers[0]
    operations.push({
      name:'Fill',kind:'fill',
      output:true,speedMmMin:settings.fill.speedMmMin,power:settings.fill.power,
      passes:settings.fill.passes,airAssist:settings.fill.airAssist,
      kerfMm:0,kerfMode:'center',pathOrder:settings.fill.pathOrder,
      paths:(layer?.paths??[]).filter(path=>path.role==='hatch')
        .map(path=>({points:translated(path.points,settings.offset),closed:false})),
    })
  }else operations.push({
    name:'Fill',kind:'fill',output:false,
    speedMmMin:settings.fill.speedMmMin,power:settings.fill.power,
    passes:settings.fill.passes,airAssist:settings.fill.airAssist,
    kerfMm:0,kerfMode:'center',pathOrder:settings.fill.pathOrder,paths:[],
  })
  return {machine,operations}
}

import {describe,expect,it} from 'vitest'
import {
  emitLaserFrame,
  emitLaserGrbl,
  preflightLaserPlan,
  type LaserPlan,
} from '../src/services/geometry/laser'
import type {PolygonMesh} from '../src/services/geometry/polygon'
import {planLaserMeshSection} from '../src/services/laserSectionPlanning'

function boxMesh():PolygonMesh {
  const corners=[[0,0,0],[10,0,0],[10,10,0],[0,10,0],[0,0,10],[10,0,10],[10,10,10],[0,10,10]]
  const faces=[[0,1,2,0,2,3],[4,6,5,4,7,6],[0,5,1,0,4,5],[1,6,2,1,5,6],[2,7,3,2,6,7],[3,4,0,3,7,4]]
  const positions:number[]=[],indices:number[]=[]
  for(const face of faces)for(const index of face){positions.push(...corners[index]!);indices.push(indices.length)}
  return {positions:Float64Array.from(positions),indices:Uint32Array.from(indices)}
}

function plan():LaserPlan {
  return {
    machine:{
      widthMm:400,
      heightMm:300,
      maxPower:1000,
      estimatedRapidMmMin:6000,
      powerMode:'m4',
      laserModeConfirmed:true,
      supportsAirAssist:true,
      flipY:false,
      returnToOrigin:true,
    },
    operations:[{
      name:'Outline',
      kind:'line',
      output:true,
      speedMmMin:900,
      power:250,
      passes:2,
      airAssist:true,
      paths:[{points:[[10,20],[30,20],[30,40]],closed:true}],
    }],
  }
}

describe('laser CAM host API',()=>{
  it('preflights and emits deterministic GRBL through the real WASM bridge',()=>{
    const source=plan()
    const summary=preflightLaserPlan(source)
    const program=emitLaserGrbl(source)
    expect(program.dialect).toBe('open-scad-viewer/laser-grbl 1')
    expect(program.summary).toEqual(summary)
    expect(summary).toMatchObject({operationCount:1,pathCount:1,segmentCount:6})
    expect(program.gcode).toContain('M4 S250\n')
    expect(program.gcode).toContain('M8\n')
    expect(program.gcode).toContain('M9\n')
    expect(emitLaserGrbl(source)).toEqual(program)
  })

  it('keeps framing laser-off even when job confirmation and process values are absent',()=>{
    const source=plan()
    const frame=emitLaserFrame({
      ...source,
      machine:{...source.machine,laserModeConfirmed:false,maxPower:0},
      operations:source.operations.map(operation=>({...operation,power:0,passes:0})),
    })
    expect(frame.dialect).toBe('open-scad-viewer/laser-frame 1')
    expect(frame.gcode.match(/^G0 /gm)).toHaveLength(5)
    expect(frame.gcode).not.toMatch(/^(?:M3|M4|G1|S)/m)
  })

  it('fails closed for unconfirmed jobs and out-of-bed geometry',()=>{
    const source=plan()
    expect(()=>emitLaserGrbl({...source,machine:{...source.machine,laserModeConfirmed:false}}))
      .toThrow(expect.objectContaining({code:'LASER_MODE_UNCONFIRMED'}))
    const outside=source.operations.map(operation=>({...operation,paths:[{points:[[-1,0],[1,0]] as [number,number][],closed:false}]}))
    expect(()=>preflightLaserPlan({...source,operations:outside}))
      .toThrow(expect.objectContaining({code:'LASER_OUT_OF_BOUNDS'}))
  })

  it('turns a CAD section into ordered Line and hatched Fill operations',()=>{
    const source=plan()
    const planned=planLaserMeshSection(boxMesh(),5,source.machine,{
      line:{output:true,speedMmMin:900,power:250,passes:1,airAssist:false},
      fill:{output:true,speedMmMin:1800,power:100,passes:1,airAssist:false,spacingMm:2},
      offset:[20,30],
    })
    expect(planned.operations.map(operation=>operation.kind)).toEqual(['line','fill'])
    expect(planned.operations[0]!.paths.length).toBeGreaterThan(0)
    expect(planned.operations[1]!.paths.length).toBeGreaterThan(0)
    expect(planned.operations[0]!.paths.flatMap(path=>path.points).every(([x,y])=>x>=20&&y>=30)).toBe(true)
    expect(preflightLaserPlan(planned)).toMatchObject({operationCount:2})

    const lineOnly=planLaserMeshSection(boxMesh(),5,source.machine,{
      line:{output:true,speedMmMin:900,power:250,passes:1,airAssist:false},
      fill:{output:false,speedMmMin:1800,power:100,passes:1,airAssist:false,spacingMm:2},
      offset:[20,30],
    })
    expect(lineOnly.operations[1]).toMatchObject({kind:'fill',output:false,paths:[]})
    expect(emitLaserGrbl(lineOnly).summary).toMatchObject({operationCount:1})
  })
})

import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const example=readFileSync('examples/rush/verified-gear.r','utf8')

it.each([
  {name:'spur',teeth:8,angle:0,doubleHelix:false,internal:false},
  {name:'left helix',teeth:8,angle:-20,doubleHelix:false,internal:false},
  {name:'herringbone',teeth:8,angle:20,doubleHelix:true,internal:false},
  {name:'ring',teeth:40,angle:0,doubleHelix:false,internal:true},
])('binds the verified $name profile to the actual editor body',async family=>{
  const program=example.replace('teeth:8',`teeth:${family.teeth}`)
    .replace('helix_angle:20,herringbone:true',`helix_angle:${family.angle},herringbone:${family.doubleHelix},internal:${family.internal}`)
  const scene=await buildTextNurbsScene(compileRushFrontend(program).document)
  const artifact=scene.meshes[0]!.nativeGeometry!
  expect(artifact.kind).toBe('brep')
  const model=JSON.parse(artifact.geometryJson).geometry
  expect(model.bodies).toHaveLength(1)
  expect(model.shells).toHaveLength(1)
  expect(model.shells[0].closed).toBe(true)
  const measurement=scene.measurements!.find(r=>r.operation==='brep_nurbs_verified_gear')!
  const proof=JSON.parse(measurement.report)
  expect(proof.profileWinding).toEqual(family.internal?[1,-1]:[1])
  expect(typeof proof.profileErrorUpperMm).toBe('number')
  expect(proof.profileErrorUpperMm).toBeLessThanOrEqual(1e-6)
  expect(typeof proof.idealWallErrorUpperMm).toBe('number')
  expect(proof.idealWallErrorUpperMm).toBeLessThanOrEqual(1.01e-4)
  expect(proof.solidGeometryCertified).toBe(false)
},180000)

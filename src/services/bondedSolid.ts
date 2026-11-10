import {callGeometryRust} from '../kernel/geometry/kernel'
import {parseBondedSolidInput,type BondedSolidResult} from './bondedSolidProtocol'
export function solveBondedSolid(inputJson:string):BondedSolidResult {
  return callGeometryRust('bonded_solid_solve',parseBondedSolidInput(inputJson))
}

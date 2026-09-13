import {callGeometryRust} from './geometry/kernel'
import type {LighteningOptions} from './solidLightening'
export interface LatticePrintSettings {nozzle:number;layer:number;lines:number;skinLayers:number;maxBridge:number;openTop:boolean}
/** Native nozzle/layer quantization of lattice rib, cell, skin, rim and floor dimensions. */
export function fitLatticeToPrint(o:LighteningOptions,p:LatticePrintSettings):LighteningOptions{
 return callGeometryRust('cad_lattice_print_fit',{options:o,settings:p})
}
/** Native cell-opening vs bridge-limit comparison; non-finite inputs never warn. */
export function printBridgeWarning(o:LighteningOptions,p:LatticePrintSettings):boolean{
 return callGeometryRust('cad_lattice_bridge_warning',{cell:o.cell,rib:o.rib,maxBridge:p.maxBridge})
}

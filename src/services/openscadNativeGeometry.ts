/** Native geometry integration API. Unsupported profile/modules return diagnostics; no implicit fallback. */
import {languageRequest} from './languages/kernel'
import {callGeometryRust} from './geometry/kernel'
import {normalizePolygonMesh,type PolygonMesh} from './geometry/polygon'
interface Diagnostic {message:string;start:number;end:number;line:number;column:number;code?:string}
export type NativeOpenScadGeometryResult = {ok:false;diagnostics?:Diagnostic[];aborted?:boolean} | {ok:true;meshes:PolygonMesh[];warnings:string[];reduced:boolean}
export function evaluateNativeOpenScadGeometry(source:string,options:{quality?:'preview'|'full';profile?:'openscad-viewer-subset@1'|'openscad/stable-2021.01'}={}):NativeOpenScadGeometryResult {
  const result=languageRequest(16,{source,quality:options.quality??'full',profile:options.profile??'openscad-viewer-subset@1'}) as {ok:false;diagnostics?:Diagnostic[];aborted?:boolean}|{ok:true;program:object;warnings:string[];reduced:boolean}
  if(!result.ok)return result
  const report=callGeometryRust<{meshes:PolygonMesh[];warnings:string[];reduced:boolean}>('solid_program_execute_report',{program:result.program})
  const meshes=report.meshes.map(normalizePolygonMesh)
  const warnings=[...result.warnings]
  const seen=new Set(warnings)
  for(const warning of report.warnings)if(!seen.has(warning)){seen.add(warning);warnings.push(warning)}
  return {ok:true,meshes,warnings,reduced:result.reduced||report.reduced}
}

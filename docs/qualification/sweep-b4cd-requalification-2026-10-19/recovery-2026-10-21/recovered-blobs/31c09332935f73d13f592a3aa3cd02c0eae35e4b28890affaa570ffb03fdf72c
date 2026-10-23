import type {PrintStrengthProfile} from './trussScreening'
type V3=[number,number,number]
export interface BondedSolidModel {
  nodesMm:V3[]
  tets:{nodes:[number,number,number,number];region:'shell'|'infill';youngMpa:number;poisson:number}[]
  bonds:{shell:V3;infill:V3;normalStiffnessMpaPerMm:number;shearStiffnessMpaPerMm:number;tensionMpa:number;shearMpa:number;compressionMpa:number;propertySource:string}[]
  restrained:[boolean,boolean,boolean][]
  forcesN:V3[]
  safetyFactor:number
  profile:PrintStrengthProfile
  serviceTempC:number
}
export interface BondedSolidResult {
  modelKind:'explicit-tet4-intact-bonds-v1'
  geometryBinding:'explicit-mesh-not-cad-verified'
  displacementsMm:V3[]
  reactionsN:V3[]
  maxDeflectionMm:number
  maxRelativeResidual:number
  stressesMpa:number[][]
  volumesMm3:number[]
  bonds:{areaMm2:number;normal:V3;forceOnShellN:V3;openingMm:V3;normalTractionMpa:V3;shearTractionMpa:V3;utilization:number}[]
  limitReached:boolean
  profileWarnings:string[]
}
export function parseBondedSolidInput(input:string):BondedSolidModel {
  if(typeof input!=='string'||input.length>262144)throw Error('Assembly JSON exceeds 256 KiB.')
  const model=JSON.parse(input)
  if(!model||typeof model!=='object')throw Error('Provide an assembly object.')
  for(const [key,max] of [['nodesMm',125],['tets',400],['bonds',200],['restrained',125],['forcesN',125]] as const){
    if(!Array.isArray(model[key])||!model[key].length||model[key].length>max)throw Error(`${key} exceeds the explicit assembly budget.`)
  }
  return model
}
export function isBondedSolidResult(value:unknown,nodes:number,tets:number,bonds:number):value is BondedSolidResult {
  if(!value||typeof value!=='object')return false
  const v=value as BondedSolidResult
  const finite=(x:unknown):x is number=>typeof x==='number'&&Number.isFinite(x)
  const vector=(x:unknown,size=3):boolean=>Array.isArray(x)&&x.length===size&&Array.from(x).every(finite)
  return v.modelKind==='explicit-tet4-intact-bonds-v1'&&v.geometryBinding==='explicit-mesh-not-cad-verified'
    &&Array.isArray(v.displacementsMm)&&v.displacementsMm.length===nodes&&v.displacementsMm.every(p=>vector(p))
    &&Array.isArray(v.reactionsN)&&v.reactionsN.length===nodes&&v.reactionsN.every(p=>vector(p))
    &&finite(v.maxDeflectionMm)&&v.maxDeflectionMm>=0&&finite(v.maxRelativeResidual)&&v.maxRelativeResidual>=0&&v.maxRelativeResidual<=1e-9
    &&Array.isArray(v.stressesMpa)&&v.stressesMpa.length===tets&&v.stressesMpa.every(p=>vector(p,6))
    &&Array.isArray(v.volumesMm3)&&v.volumesMm3.length===tets&&v.volumesMm3.every(p=>finite(p)&&p>0)
    &&Array.isArray(v.bonds)&&v.bonds.length===bonds&&v.bonds.every(b=>b&&finite(b.areaMm2)&&b.areaMm2>0&&vector(b.normal)
      &&vector(b.forceOnShellN)&&vector(b.openingMm)&&vector(b.normalTractionMpa)&&vector(b.shearTractionMpa)&&b.shearTractionMpa.every(x=>x>=0)&&finite(b.utilization)&&b.utilization>=0)
    &&typeof v.limitReached==='boolean'&&v.limitReached===v.bonds.some(b=>b.utilization>=1)
    &&Array.isArray(v.profileWarnings)&&v.profileWarnings.length<=2&&v.profileWarnings.every(s=>['layer-above-80-percent-nozzle','line-width-not-greater-than-layer'].includes(s))
}

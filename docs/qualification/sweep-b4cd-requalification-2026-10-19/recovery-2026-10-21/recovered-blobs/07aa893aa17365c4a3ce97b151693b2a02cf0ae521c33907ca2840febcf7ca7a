export type StepSemanticConformance =
  | {status:'not-checked';validator:null;detail:string}
  | {status:'externally-conformant';validator:{name:string;version:string;sha256:string};detail:string}
  | {status:'externally-rejected';validator:{name:string;version:string;sha256:string};detail:string}

export const uncheckedStepConformance=():StepSemanticConformance=>({
  status:'not-checked',
  validator:null,
  detail:'Native bounded parsing is not an independent AP242 semantic validation.',
})

/** Refuses unbound validator claims at the product boundary. */
export function parseStepSemanticConformance(value:unknown):StepSemanticConformance {
  if(!value||typeof value!=='object')return uncheckedStepConformance()
  const candidate=value as Partial<StepSemanticConformance>
  if(candidate.status==='not-checked')return uncheckedStepConformance()
  const validator=(candidate as {validator?:{name?:unknown;version?:unknown;sha256?:unknown}}).validator
  if((candidate.status!=='externally-conformant'&&candidate.status!=='externally-rejected')
    ||!validator||typeof validator.name!=='string'||typeof validator.version!=='string'
    ||typeof validator.sha256!=='string'||!/^[0-9a-f]{64}$/.test(validator.sha256)
    ||typeof candidate.detail!=='string')return uncheckedStepConformance()
  return candidate as StepSemanticConformance
}

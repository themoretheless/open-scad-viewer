import type {GcodePreviewResult} from './geometry/polygon'

export type PackedGcodePreview = Omit<GcodePreviewResult,'moves'> & {moveRows:number[]}

/** Private fixed-field wire rows; public callers still receive ordinary move objects. */
export function unpackGcodePreview({moveRows,...metadata}:PackedGcodePreview):GcodePreviewResult {
  if(!Array.isArray(moveRows)||moveRows.length%7!==0||moveRows.length>700_000)throw new Error('Invalid packed G-code moves')
  const moves:GcodePreviewResult['moves']=new Array(moveRows.length/7)
  for(let i=0,j=0;i<moveRows.length;i+=7,j++) {
    const x=moveRows[i],y=moveRows[i+1],z=moveRows[i+2],e=moveRows[i+3],feedrateMmS=moveRows[i+4],layerIndex=moveRows[i+5],flag=moveRows[i+6]
    if(!Number.isFinite(x)||!Number.isFinite(y)||!Number.isFinite(z)||!Number.isFinite(e)||!Number.isFinite(feedrateMmS)
      ||!Number.isInteger(layerIndex)||layerIndex<0||layerIndex>=2048||(flag!==0&&flag!==1))throw new Error('Invalid packed G-code move')
    moves[j]={x,y,z,e,feedrateMmS,layerIndex,extruded:flag===1}
  }
  return {...metadata,moves}
}

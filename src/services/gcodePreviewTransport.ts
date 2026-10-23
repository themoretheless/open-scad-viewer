import type {GcodePreviewMove,GcodePreviewResult} from './geometry/polygon'

export type PackedGcodePreview = Omit<GcodePreviewResult,'moves'> & {moveRows:number[]}
export const GCODE_MOVE_ROW_WIDTH = 7
export const GCODE_MAX_MOVE_ROWS = 700_000

/** Validated packed rows owned by each unpacked preview; never the caller's wire buffer. */
const previewRows=new WeakMap<GcodePreviewResult,Float64Array>()

/**
 * Packed rows backing a preview, for allocation-free hot paths (layer slider).
 * Previews not produced by `unpackGcodePreview` are packed once from their moves.
 */
export function gcodePreviewMoveRows(preview:GcodePreviewResult):Float64Array {
  let rows=previewRows.get(preview)
  if(rows)return rows
  rows=new Float64Array(preview.moves.length*GCODE_MOVE_ROW_WIDTH)
  for(let i=0,j=0;i<preview.moves.length;i++,j+=GCODE_MOVE_ROW_WIDTH){
    const move=preview.moves[i]
    rows[j]=move.x;rows[j+1]=move.y;rows[j+2]=move.z;rows[j+3]=move.e
    rows[j+4]=move.feedrateMmS;rows[j+5]=move.layerIndex;rows[j+6]=move.extruded?1:0
  }
  previewRows.set(preview,rows)
  return rows
}

/** Move count without materializing the lazy `moves` array. */
export function gcodePreviewMoveCount(preview:GcodePreviewResult):number {
  const rows=previewRows.get(preview)
  return rows?rows.length/GCODE_MOVE_ROW_WIDTH:preview.moves.length
}

/**
 * Private fixed-field wire rows. The packed copy is validated eagerly, but the
 * public move objects materialize only if `moves` is actually read — the layer
 * slider and drawing paths work on the packed rows and never trigger it.
 */
export function unpackGcodePreview({moveRows,...metadata}:Omit<PackedGcodePreview,'moveRows'> & {moveRows:number[]|Float64Array}):GcodePreviewResult {
  if((!Array.isArray(moveRows)&&!(moveRows instanceof Float64Array))||moveRows.length%GCODE_MOVE_ROW_WIDTH!==0||moveRows.length>GCODE_MAX_MOVE_ROWS)throw new Error('Invalid packed G-code moves')
  // Copy once, so the unpacked preview never retains or aliases the wire buffer.
  const rows=moveRows instanceof Float64Array?moveRows.slice():Float64Array.from(moveRows)
  for(let i=0;i<rows.length;i+=GCODE_MOVE_ROW_WIDTH){
    const x=rows[i],y=rows[i+1],z=rows[i+2],e=rows[i+3],feedrateMmS=rows[i+4],layerIndex=rows[i+5],flag=rows[i+6]
    if(!Number.isFinite(x)||!Number.isFinite(y)||!Number.isFinite(z)||!Number.isFinite(e)||!Number.isFinite(feedrateMmS)
      ||!Number.isInteger(layerIndex)||layerIndex<0||layerIndex>=2048||(flag!==0&&flag!==1))throw new Error('Invalid packed G-code move')
  }
  const count=rows.length/GCODE_MOVE_ROW_WIDTH
  let moves:GcodePreviewMove[]|null=null
  const result={...metadata} as GcodePreviewResult
  Object.defineProperty(result,'moves',{enumerable:true,configurable:true,get(){
    if(!moves){
      moves=new Array(count)
      for(let i=0,j=0;j<count;i+=GCODE_MOVE_ROW_WIDTH,j++)
        moves[j]={x:rows[i],y:rows[i+1],z:rows[i+2],e:rows[i+3],feedrateMmS:rows[i+4],layerIndex:rows[i+5],extruded:rows[i+6]===1}
    }
    return moves
  }})
  previewRows.set(result,rows)
  return result
}

import {isSpatialPattern,type LighteningOptions} from './solidLightening'
export interface LatticePrintSettings {nozzle:number;layer:number;lines:number;skinLayers:number;maxBridge:number;openTop:boolean}
export function fitLatticeToPrint(o:LighteningOptions,p:LatticePrintSettings):LighteningOptions{
 if(![p.nozzle,p.layer,p.lines,p.skinLayers,p.maxBridge].every(Number.isFinite)||p.nozzle<=0||p.layer<=0||p.layer>p.nozzle||!Number.isInteger(p.lines)||p.lines<1||p.lines>8||!Number.isInteger(p.skinLayers)||p.skinLayers<0||p.maxBridge<=0)throw Error('Проверьте сопло, высоту слоя, число линий и длину моста.')
 const round=(v:number,s:number)=>Math.round(Math.ceil((v-1e-8)/s)*s*1e6)/1e6
 const width=Math.round(p.nozzle*1.125*1e6)/1e6,rib=round(Math.max(o.rib,width*p.lines),width),spatial=isSpatialPattern(o.pattern)
 const result={...o,lineWidth:width,perimeters:p.lines,rib,cell:Math.max(o.cell,rib*2+width)}
 if(spatial){result.skin=o.skin?round(Math.max(o.skin,width*p.lines),width):0;result.openTop=p.openTop;result.step=Math.min(o.step??rib/3,rib/3,result.skin?result.skin/2:Infinity,o.wallDepth?o.wallDepth/2:Infinity)}
 else {result.axis='z';result.bottom=round(Math.max(o.bottom,p.skinLayers*p.layer),p.layer);result.top=p.openTop?0:round(Math.max(o.top,p.skinLayers*p.layer),p.layer);result.rim=round(Math.max(o.rim,width*p.lines),width)}
 return result
}
export function printBridgeWarning(o:LighteningOptions,p:LatticePrintSettings):boolean{return o.cell-o.rib>p.maxBridge}

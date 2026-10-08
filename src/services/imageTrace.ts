import type {DirectSketch} from './directModeling'
import type {SketchPlane} from './directSketchGeometry'
import {withEditableSketchPath} from './editableSketchPath'
import {pathFromPolygon,simplifyPath} from './geometry/path2d'
import type {SvgJob} from './svgWorkerProtocol'

export interface ImageTraceOptions {widthMm:number;pixelWidth:number;pixelHeight:number;threshold:number;resolution:number;tolerance:number;mode:'dark'|'alpha'}
/** SVG is only transport into the native bounded raster/mask/contour implementation. */
export function imageTraceJob(dataUrl:string,o:ImageTraceOptions):SvgJob {
 if(!/^data:image\/(png|jpeg|webp);base64,[A-Za-z0-9+/]+=*$/.test(dataUrl)||dataUrl.length>3_800_000)throw Error('Use a PNG, JPEG or WebP image smaller than 2.8 MiB.')
 if(!Number.isFinite(o.widthMm)||o.widthMm<=0||o.widthMm>10000||![o.pixelWidth,o.pixelHeight].every(v=>Number.isInteger(v)&&v>0&&v<=16384))throw Error('Invalid image dimensions.')
 if(!Number.isFinite(o.threshold)||o.threshold<.01||o.threshold>.99||!Number.isInteger(o.resolution)||o.resolution<128||o.resolution>2048||!Number.isFinite(o.tolerance)||o.tolerance<.0001||o.tolerance>10||!['dark','alpha'].includes(o.mode))throw Error('Invalid trace settings.')
 const h=o.widthMm*o.pixelHeight/o.pixelWidth
 if(h>10000)throw Error('Image height exceeds 10000 mm.')
 const image=`<image width="${o.pixelWidth}" height="${o.pixelHeight}" href="${dataUrl}" image-rendering="optimizeSpeed"/>`
 const darkness=`<defs><filter id="invert" color-interpolation-filters="sRGB"><feColorMatrix type="matrix" values="-.2126 -.7152 -.0722 0 1 -.2126 -.7152 -.0722 0 1 -.2126 -.7152 -.0722 0 1 0 0 0 1 0"/></filter><mask id="ink" maskUnits="userSpaceOnUse" x="0" y="0" width="${o.pixelWidth}" height="${o.pixelHeight}"><g filter="url(#invert)">${image}</g></mask></defs><rect width="${o.pixelWidth}" height="${o.pixelHeight}" mask="url(#ink)"/>`
 return {kind:'contours',svg:`<svg xmlns="http://www.w3.org/2000/svg" width="${o.widthMm}mm" height="${h}mm" viewBox="0 0 ${o.pixelWidth} ${o.pixelHeight}">${o.mode==='alpha'?image:darkness}</svg>`,options:{geometryMode:'silhouette',rasterSize:o.resolution,alphaThreshold:o.mode==='alpha'?o.threshold:1-o.threshold,tolerance:o.tolerance}}
}
/** Each boundary remains editable; negative-winding hole boundaries are retained. */
export function imageTraceSketches(contours:[number,number][][],tolerance:number,plane:SketchPlane,name:string):DirectSketch[] {
 if(!contours.length)throw Error('No contours at this threshold.')
 if(contours.length>200)throw Error('Image has more than 200 contours. Reduce resolution or adjust the threshold.')
 return contours.map((ring,i)=>withEditableSketchPath({id:crypto.randomUUID(),name:`${name.slice(0,60)} · ${i+1}`,points:[],closed:true,plane:structuredClone(plane)},simplifyPath(pathFromPolygon(ring,true),tolerance)))
}

export function imageTraceColorJob(dataUrl:string,o:ImageTraceOptions,colors:number,ignoreWhite:boolean,minArea:number):SvgJob {
 const base=imageTraceJob(dataUrl,{...o,mode:'alpha'})
 if(base.kind!=='contours'||!Number.isInteger(colors)||colors<2||colors>16||!Number.isFinite(minArea)||minArea<0||minArea>1e8)throw Error('Invalid color trace settings.')
 return {kind:'colorContours',svg:base.svg,options:base.options,widthMm:o.widthMm,heightMm:o.widthMm*o.pixelHeight/o.pixelWidth,resolution:o.resolution,alpha:o.threshold,colors,ignoreWhite,minArea,tolerance:o.tolerance}
}
export function imageTraceColorSketches(layers:{color:[number,number,number];contours:[number,number][][]}[],tolerance:number,plane:SketchPlane,name:string):DirectSketch[]{
 const sketches=layers.flatMap(layer=>{const color='#'+layer.color.map(c=>c.toString(16).padStart(2,'0')).join('');return imageTraceSketches(layer.contours,tolerance,plane,name+' '+color).map(s=>({...s,traceColor:color}))})
 if(sketches.length>200)throw Error('Color trace exceeds 200 contours. Increase minimum area or reduce resolution.')
 if(!sketches.length)throw Error('No colors at these trace settings.')
 return sketches
}

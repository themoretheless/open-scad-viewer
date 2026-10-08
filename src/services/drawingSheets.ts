import type {DirectBody} from './directModeling'
import {callGeometryRust} from './geometry/kernel'
export interface DrawingSheetOptions {template:'A5'|'A4'|'A3'|'A2'|'Letter';scale:number;title:string;author:string;perBody:boolean;portrait?:boolean;hidden?:'show'|'hide'|'dash';dimensions?:boolean;occlusion?:'mesh'|'brep';frame?:'simple'|'detail'|'assembly';drawingNumber?:string;revision?:string;material?:string;linkedDimensions?:DrawingDimension[];vertexLabels?:boolean}
export interface DrawingDimension {bodyId:string;vertices:[number,number];topologySignature:string;view:'top'|'front'|'right';axis:'horizontal'|'vertical'}
export interface DrawingReference {bodyId:string;topologySignature:string;vertices:number[][]}
export interface DrawingSheet {svg:string;width:number;height:number;title:string;references?:DrawingReference[]}
export function parseDrawingDimensions(text:string):DrawingDimension[]{
 if(text.length>1048576)throw Error('Dimension definitions exceed 1 MiB.')
 const values:unknown=JSON.parse(text)
 if(!Array.isArray(values)||values.length>64)throw Error('Expected up to 64 dimension definitions.')
 for(const value of values){
  if(!value||typeof value!=='object')throw Error('Invalid dimension definition.')
  const d=value as DrawingDimension
  if(typeof d.bodyId!=='string'||!d.bodyId||d.bodyId.length>120||typeof d.topologySignature!=='string'||d.topologySignature.length>1000000||!Array.isArray(d.vertices)||d.vertices.length!==2||d.vertices.some(v=>!Number.isInteger(v)||v<0||v>=16384)||d.vertices[0]===d.vertices[1]||!['top','front','right'].includes(d.view)||!['horizontal','vertical'].includes(d.axis))throw Error('Invalid dimension reference.')
 }
 return values as DrawingDimension[]
}
const escape=(s:string)=>s.replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&apos;'}[c]!))
const wrapped=(text:string,characters:number)=>{
 const points=Array.from(text),lines:string[]=[]
 for(let i=0;i<points.length;i+=characters)lines.push(points.slice(i,i+characters).join(''))
 return lines
}
const fieldText=(x:number,y:number,text:string,characters:number,size=2.6)=>wrapped(text,characters).map((line,i)=>`<text x="${x}" y="${y+i*3}" font-size="${size}">${escape(line)}</text>`).join('')
/** Native orthographic projection; host work is sheet layout and annotations only. */
export function createDrawingSheets(bodies:DirectBody[],options:DrawingSheetOptions):DrawingSheet[]{
 if(bodies.reduce((n,b)=>n+b.mesh.indices.length/3,0)>100000)throw Error('Drawing exceeds 100000 mesh faces.')
 if(!bodies.length||bodies.length>200)throw Error('Select 1–200 bodies.')
 if(!['A5','A4','A3','A2','Letter'].includes(options.template)||!Number.isFinite(options.scale)||options.scale<=0||options.scale>100)throw Error('Invalid sheet template or scale.')
 if((options.linkedDimensions??[]).some(d=>!bodies.some(b=>b.id===d.bodyId)||!['top','front','right'].includes(d.view)))throw Error('A linked dimension references a missing body or view. Reattach it.')
 if((options.linkedDimensions?.length??0)>64)throw Error('At most 64 linked dimensions.')
 if(options.frame&&!['simple','detail','assembly'].includes(options.frame))throw Error('Invalid drawing frame.')
 if((options.revision?.length??0)>20)throw Error('Revision is limited to 20 characters.')
 if([options.drawingNumber??'',options.material??''].some(s=>s.length>80))throw Error('Drawing frame labels are too long.')
 if(options.title.length>120||options.author.length>80)throw Error('Drawing labels are too long.')
 const formats={A5:[210,148],A4:[297,210],A3:[420,297],A2:[594,420],Letter:[279.4,215.9]}
 const format=formats[options.template];const [width,height]=options.portrait?[format[1],format[0]]:format
 if(options.frame&&options.frame!=='simple'&&width<196)throw Error('Engineering title blocks need a sheet at least 196 mm wide. Select landscape A5 or A4.')
 if(options.hidden&&!['show','hide','dash'].includes(options.hidden))throw Error('Invalid hidden line mode.')
 const groups=options.perBody?bodies.map(b=>[b]):[bodies]
 if(groups.length>40)throw Error('Drawing export is limited to 40 sheets.')
 return groups.map((group,index)=>{
  const title=options.perBody?group[0].name:options.title
  const views=[{view:'top',name:'TOP XY'},{view:'front',name:'FRONT XZ'},{view:'right',name:'RIGHT YZ'}] as const
  const cellWidth=(width-24)/3,cellHeight=height-((options.frame&&options.frame!=='simple')?80:(options.linkedDimensions?.length?72:60))
  let approximate=false
  let references:DrawingReference[]=[]
  const drawings=views.map((view,column)=>{
   const projected=callGeometryRust<{paths:{d:string;hidden:boolean}[];bounds:number[];approximate:boolean;occlusionToleranceMm:number;references:DrawingReference[];dimensions:{a:number[];b:number[];valueMm:number;axis:'horizontal'|'vertical'}[]}>('cad_drawing_project',{bodies:group,view:view.view,hidden:options.hidden??'dash',occlusion:options.occlusion??'mesh',dimensions:(options.linkedDimensions??[]).filter(d=>d.view===view.view&&group.some(b=>b.id===d.bodyId))})
   approximate ||= projected.approximate
   if(view.view==='top')references=projected.references
   const [minX,minY,maxX,maxY]=projected.bounds
   const spanX=maxX-minX,spanY=maxY-minY
   if(projected.dimensions.length>4)throw Error('At most four linked dimensions per view. Use a separate sheet for more.')
   if(spanX*options.scale>cellWidth-(projected.dimensions.length?36:8)||spanY*options.scale>cellHeight-(projected.dimensions.length?40:20))throw Error('Geometry does not fit this scale. Reduce the scale or select a larger sheet.')
   const x=12+column*cellWidth+(cellWidth-spanX*options.scale)/2,y=24+(cellHeight-spanY*options.scale)/2
   const dimensions=options.dimensions?`<path d="M${x},${y+spanY*options.scale+3}h${spanX*options.scale}" fill="none" stroke="black" stroke-width=".12"/><text x="${x}" y="${y+spanY*options.scale+7}">${spanX.toFixed(2)} × ${spanY.toFixed(2)} mm</text>`:''
   const linked=projected.dimensions.map((d,i)=>{
    const ax=x+(d.a[0]-minX)*options.scale,ay=y+(d.a[1]-minY)*options.scale
    const bx=x+(d.b[0]-minX)*options.scale,by=y+(d.b[1]-minY)*options.scale
    const offset=8+i*6
    if(d.axis==='horizontal'){
     const dy=y+spanY*options.scale+offset
     return `<g data-dimension="linked" fill="none" stroke="black" stroke-width=".12"><path d="M${ax} ${ay}V${dy+1} M${bx} ${by}V${dy+1} M${ax} ${dy}H${bx}"/><path d="M${ax-1} ${dy-1}l2 2 M${bx-1} ${dy-1}l2 2"/></g><text x="${(ax+bx)/2}" y="${dy-1}" text-anchor="middle">${d.valueMm.toFixed(2)} mm</text>`
    }
    const dx=x+spanX*options.scale+offset
    return `<g data-dimension="linked" fill="none" stroke="black" stroke-width=".12"><path d="M${ax} ${ay}H${dx+1} M${bx} ${by}H${dx+1} M${dx} ${ay}V${by}"/><path d="M${dx-1} ${ay-1}l2 2 M${dx-1} ${by-1}l2 2"/></g><text x="${dx+1}" y="${(ay+by)/2}">${d.valueMm.toFixed(2)} mm</text>`
   }).join('')
   const markers=options.vertexLabels?projected.references.flatMap(r=>r.vertices.map((p,i)=>`<text x="${x+(p[0]-minX)*options.scale+1}" y="${y+(p[1]-minY)*options.scale-1}" fill="#2870aa" font-size="2">${i+1}</text>`)).join(''):''
   return `<text x="${12+column*cellWidth}" y="18">${view.name}</text><g transform="translate(${x},${y}) scale(${options.scale}) translate(${-minX},${-minY})" fill="none" stroke="black" stroke-width="${.12/options.scale}">${projected.paths.map(p=>`<path d="${escape(p.d)}"${p.hidden?` stroke-dasharray="${.8/options.scale} ${.5/options.scale}"`:''}/>`).join('')}</g>${dimensions}${linked}${markers}`
  }).join('')
  const engineering=options.frame&&options.frame!=='simple'
  const headingLines=[...wrapped(title,65),...wrapped(`${options.drawingNumber??''} · ${options.frame==='assembly'?'Assembly':'Detail'}`,65)]
  const block=engineering?`<g font-size="2.6"><path d="M${width-188} ${height-40}H${width-8}V${height-8}H${width-188}Z M${width-188} ${height-24}H${width-8} M${width-188} ${height-16}H${width-8} M${width-55} ${height-24}V${height-8}" fill="none" stroke="black" stroke-width=".25"/>${headingLines.map((line,i)=>`<text x="${width-184}" y="${height-36+i*3}" font-size="2.4">${escape(line)}</text>`).join('')}${fieldText(width-184,height-21,options.frame==='assembly'?`${group.length} parts`:options.material??'',43)}${fieldText(width-51,height-21,`Rev ${options.revision??''}`,13)}${fieldText(width-184,height-13,options.author,45,2.4)}<text x="${width-51}" y="${height-13}" font-size="2.4">${options.scale}:1</text><text x="${width-51}" y="${height-10}">${index+1}/${groups.length}</text></g>`:''
  const surfaceNote=engineering?`<text x="12" y="${height-43}" font-size="2.4">${options.occlusion==='brep'?'Planar B-rep':'Mesh occlusion'} · band ${options.occlusion==='brep'?'0.00001':'0.02'} mm</text>`:''
  const svg=`<svg xmlns="http://www.w3.org/2000/svg" width="${width}mm" height="${height}mm" viewBox="0 0 ${width} ${height}"><rect width="${width}" height="${height}" fill="white"/><g font-family="sans-serif" font-size="3" fill="black"><rect x="8" y="8" width="${width-16}" height="${height-16}" fill="none" stroke="black" stroke-width=".25"/>${drawings}${engineering?'':`<path d="M8 ${height-32} H${width-8}" stroke="black" stroke-width=".25"/><text x="12" y="${height-25}">${escape(title)}</text><text x="12" y="${height-18}">${escape(options.author)}</text><text x="12" y="${height-11}">Scale ${options.scale}:1 · mm · ${index+1}/${groups.length}</text><text x="${width-95}" y="${height-11}">${approximate?'Mesh/approximated curves':'Retained B-rep curves'} · ${options.occlusion??'mesh'} hidden ${options.hidden??'dash'}</text>`}${block}${surfaceNote}</g></svg>`
  return {svg,width,height,title,references}
 })
}
/** JPEG pages preserve Unicode labels and browser font rendering in the PDF. */
export function drawingPdf(pages:{jpeg:Uint8Array;width:number;height:number;pixelWidth:number;pixelHeight:number}[]):Uint8Array{
 if(!pages.length||pages.length>40)throw Error('Expected 1–40 PDF pages.')
 const enc=new TextEncoder(),chunks:Uint8Array[]=[],offsets=[0];let size=0
 const append=(v:string|Uint8Array)=>{const b=typeof v==='string'?enc.encode(v):v;chunks.push(b);size+=b.length}
 const object=(id:number,body:string|Uint8Array)=>{offsets[id]=size;append(`${id} 0 obj\n`);append(body);append('\nendobj\n')}
 append('%PDF-1.4\n');object(1,'<< /Type /Catalog /Pages 2 0 R >>');object(2,`<< /Type /Pages /Count ${pages.length} /Kids [${pages.map((_,i)=>`${3+i*3} 0 R`).join(' ')}] >>`)
 pages.forEach((page,i)=>{
  const id=3+i*3,w=page.width*72/25.4,h=page.height*72/25.4
  object(id,`<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${w} ${h}] /Resources << /XObject << /Im ${id+2} 0 R >> >> /Contents ${id+1} 0 R >>`)
  const content=`q ${w} 0 0 ${h} 0 0 cm /Im Do Q`
  object(id+1,`<< /Length ${enc.encode(content).length} >>\nstream\n${content}\nendstream`)
  offsets[id+2]=size;append(`${id+2} 0 obj\n<< /Type /XObject /Subtype /Image /Width ${page.pixelWidth} /Height ${page.pixelHeight} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${page.jpeg.length} >>\nstream\n`);append(page.jpeg);append('\nendstream\nendobj\n')
 })
 const start=size,count=3+pages.length*3
 append(`xref\n0 ${count}\n0000000000 65535 f \n${offsets.slice(1).map(o=>String(o).padStart(10,'0')+' 00000 n \n').join('')}trailer\n<< /Size ${count} /Root 1 0 R >>\nstartxref\n${start}\n%%EOF\n`)
 const result=new Uint8Array(size);let cursor=0;for(const chunk of chunks){result.set(chunk,cursor);cursor+=chunk.length}return result
}
/** Native vector PDF; searchable Unicode text uses the embedded bundled font. */
export function vectorDrawingPdf(sheets:DrawingSheet[]):Uint8Array {
 if(!sheets.length||sheets.length>40)throw Error('Expected 1–40 PDF sheets.')
 return new TextEncoder().encode(callGeometryRust<string>('drawing_pdf',{sheets:sheets.map(sheet=>sheet.svg)}))
}
export async function renderDrawingPdf(sheets:DrawingSheet[]):Promise<Uint8Array>{return vectorDrawingPdf(sheets)}

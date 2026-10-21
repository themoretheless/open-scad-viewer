import type {DirectBody} from './directModeling'
import type {DrawingSheet,DrawingSheetOptions} from './drawingSheets'

export interface DrawingPartMetadata {bodyId:string;designation?:string;description?:string;material?:string;note?:string}
export interface DrawingSpecificationRow {position:number;designation:string;description:string;quantity:number;material:string;note:string;bodyIds:string[]}
const escape=(s:string)=>s.replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&apos;'}[c]!))
const wrap=(value:string,max:number)=>value.split(/\r?\n/).flatMap(line=>{const points=Array.from(line),lines:string[]=[];for(let i=0;i<points.length;i+=max)lines.push(points.slice(i,i+max).join(''));return lines.length?lines:['']})
const limits={designation:80,description:120,material:80,note:120} as const
export function parseDrawingParts(text:string):DrawingPartMetadata[]{
 if(text.length>1048576)throw Error('Part definitions exceed 1 MiB.')
 const parts:unknown=JSON.parse(text)
 if(!Array.isArray(parts)||parts.length>200)throw Error('Expected up to 200 part definitions.')
 const seen=new Set<string>()
 for(const value of parts){
  if(!value||typeof value!=='object')throw Error('Invalid part definition.')
  const part=value as DrawingPartMetadata
  if(typeof part.bodyId!=='string'||!part.bodyId||part.bodyId.length>120||seen.has(part.bodyId))throw Error('Invalid or duplicate specification body.')
  seen.add(part.bodyId)
  for(const key of Object.keys(limits) as (keyof typeof limits)[])if(part[key]!==undefined&&(typeof part[key]!=='string'||part[key]!.length>limits[key]))throw Error('Invalid or oversized specification field.')
 }
 return parts as DrawingPartMetadata[]
}
export function drawingSpecificationRows(bodies:DirectBody[],parts:DrawingPartMetadata[]=[]):DrawingSpecificationRow[]{
 if(!bodies.length||bodies.length>200||new Set(bodies.map(b=>b.id)).size!==bodies.length)throw Error('A specification needs 1–200 distinct bodies.')
 const entries=parseDrawingParts(JSON.stringify(parts)),byBody=new Map(entries.map(p=>[p.bodyId,p])),bodyIds=new Set(bodies.map(b=>b.id))
 if(entries.some(p=>!bodyIds.has(p.bodyId)))throw Error('A specification entry references a missing body.')
 const rows:DrawingSpecificationRow[]=[],declared=new Map<string,DrawingSpecificationRow>()
 for(const body of bodies){
  const part=byBody.get(body.id),designation=part?.designation?.trim()??''
  const description=part?.description?.trim()||designation||body.name,material=part?.material?.trim()??'',note=part?.note?.trim()??''
  const existing=designation?declared.get(designation):undefined
  if(existing){
   if(existing.description!==description||existing.material!==material||existing.note!==note)throw Error('One designation has conflicting specification fields. Use a separate designation for distinct parts.')
   existing.quantity++;existing.bodyIds.push(body.id);continue
  }
  const row={position:rows.length+1,designation,description,quantity:1,material,note,bodyIds:[body.id]}
  rows.push(row);if(designation)declared.set(designation,row)
 }
 return rows
}
function escapeCsvCell(value:string):string {
 const normalized=value.replace(/\r\n/g,'\n')
 return /[",\n]/.test(normalized)?`"${normalized.replace(/"/g,'""')}"`:normalized
}
export function exportDrawingSpecificationCsv(rows:readonly DrawingSpecificationRow[],locale:'ru'|'en'='en'):string {
 const headers=locale==='ru'
  ?['Поз.','Обозначение','Наименование','Кол.','Материал','Примечание']
  :['Position','Designation','Description','Quantity','Material','Note']
 const lines=[headers.map(escapeCsvCell).join(',')]
 for(const row of rows){
  lines.push([
   String(row.position),
   row.designation,
   row.description,
   String(row.quantity),
   row.material,
   row.note,
  ].map(escapeCsvCell).join(','))
 }
 return lines.join('\n')
}
function escapeMarkdownCell(value:string):string {
 return value.replace(/\r?\n/g,'<br>').replace(/\|/g,'\\|')
}
export function exportDrawingSpecificationMarkdown(rows:readonly DrawingSpecificationRow[],locale:'ru'|'en'='en'):string {
 const headers=locale==='ru'
  ?['Поз.','Обозначение','Наименование','Кол.','Материал','Примечание']
  :['Item','Designation','Description','Qty','Material','Note']
 const headLine=`| ${headers.join(' | ')} |`
 const sepLine=`| ${headers.map((_,i)=>i===0||i===3?'---:':':---').join(' | ')} |`
 const bodyLines=rows.map(row=>`| ${[
  String(row.position),
  escapeMarkdownCell(row.designation),
  escapeMarkdownCell(row.description),
  String(row.quantity),
  escapeMarkdownCell(row.material),
  escapeMarkdownCell(row.note),
 ].join(' | ')} |`)
 return [headLine,sepLine,...bodyLines].join('\n')
}
/** Presentation only: counts refer to authored body occurrences; no shape
 * equivalence, physical material or mass is inferred from display meshes. */
export function createSpecificationSheets(bodies:DirectBody[],options:DrawingSheetOptions,width:number,height:number):DrawingSheet[]{
 if(!options.specification)return []
 if(width<196)throw Error('Specification tables need a sheet at least 196 mm wide. Select landscape A5 or A4.')
 const rows=drawingSpecificationRows(bodies,options.parts),ru=options.locale!=='en',tableWidth=width-24,font=2.4,lineHeight=3.4
 const ratios=[.06,.18,.28,.06,.17,.25],columns=ratios.map(r=>r*tableWidth)
 const titles=ru?['Поз.','Обозначение','Наименование','Кол.','Материал','Примечание']:['Item','Designation','Description','Qty','Material','Note']
 const heading=wrap(`${ru?'Спецификация':'Parts list'}${options.title?' - '+options.title:''}`,Math.max(1,Math.floor(tableWidth/3.3)))
 const top=16+heading.length*4+6,headerHeight=12,bottom=height-40
 const layout=rows.map(row=>{
  const values=[String(row.position),row.designation,row.description,String(row.quantity),row.material,row.note]
  const lines=values.map((value,i)=>i===0||i===3?[value]:wrap(value,Math.max(1,Math.floor((columns[i]-4)/(font*1.1)))))
  return {row,lines,height:4+lineHeight*Math.max(...lines.map(v=>v.length))}
 })
 const pages:typeof layout[]=[];let current:typeof layout=[],used=top+headerHeight
 for(const item of layout){
  if(item.height>bottom-top-headerHeight)throw Error('A specification row does not fit the selected sheet. Use a larger format or shorten the text.')
  if(used+item.height>bottom){pages.push(current);current=[];used=top+headerHeight}
  current.push(item);used+=item.height
 }
 if(current.length)pages.push(current)
 if(pages.length>40)throw Error('Specification export exceeds 40 sheets. Use a larger format or shorten the text.')
 const text=(x:number,y:number,value:string,size=font)=>`<text x="${x}" y="${y}" font-size="${size}">${escape(value)}</text>`
 return pages.map((items,index)=>{
  let y=top+headerHeight,content='',rowLines='',x=12
  for(let col=0;col<columns.length;col++){
   const headerLines=wrap(titles[col],Math.max(1,Math.floor((columns[col]-4)/2.42)))
   content+=headerLines.map((s,i)=>text(x+2,top+4+i*lineHeight,s,2.2)).join('');x+=columns[col]
  }
  for(const item of items){
   x=12
   item.lines.forEach((lines,col)=>{content+=lines.map((s,i)=>text(x+2,y+4+i*lineHeight,s)).join('');x+=columns[col]})
   y+=item.height;rowLines+=`<path d="M12 ${y}H${width-12}"/>`
  }
  let grid=`<path d="M12 ${top}H${width-12} M12 ${top+headerHeight}H${width-12}"/>`;x=12
  for(const column of [...columns,0]){grid+=`<path d="M${x} ${top}V${y}"/>`;x+=column}
  const footerLine=`${options.drawingNumber??''}${options.revision?' · Rev '+options.revision:''}`
  const footer=wrap(footerLine,Math.max(1,Math.floor(tableWidth/2.64))).map((s,i)=>text(12,height-29+i*4,s)).join('')+wrap(options.author,Math.max(1,Math.floor(tableWidth/2.64))).map((s,i)=>text(12,height-20+i*4,s)).join('')
  const svg=`<svg xmlns="http://www.w3.org/2000/svg" width="${width}mm" height="${height}mm" viewBox="0 0 ${width} ${height}"><rect width="${width}" height="${height}" fill="white"/><g font-family="sans-serif" fill="black"><rect x="8" y="8" width="${width-16}" height="${height-16}" fill="none" stroke="black" stroke-width=".25"/>${heading.map((s,i)=>text(12,16+i*4,s,3)).join('')}<g fill="none" stroke="black" stroke-width=".12">${grid}${rowLines}</g>${content}<path d="M8 ${height-36}H${width-8}" stroke="black" stroke-width=".25"/>${footer}<text x="${width-42}" y="${height-11}" font-size="2.4" data-sheet-number="true">${index+1}/${pages.length}</text></g></svg>`
  return {svg,width,height,title:`${ru?'Спецификация':'Parts list'} ${index+1}`,kind:'specification',specificationRows:items.map(item=>item.row)}
 })
}

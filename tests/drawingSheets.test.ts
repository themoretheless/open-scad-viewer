import {describe,it,expect} from 'vitest'
import {createDrawingSheets,drawingPdf,vectorDrawingPdf,parseDrawingDimensions} from '../src/services/drawingSheets'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
const body=(id:string)=>{const brep=createBrepBox([0,0,0],[20,10,5]);return {id,name:'Деталь <'+id+'>',brep,mesh:tessellateNurbsBrep(brep,1)}}
describe('native drawing sheets',()=>{
 it('exports Unicode labels as embedded searchable text without page images',()=>{
  const sheets=createDrawingSheets([body('a'),body('b')],{template:'A4',scale:1,title:'Сборка',author:'Проверка',perBody:true})
  const pdf=vectorDrawingPdf(sheets),text=new TextDecoder().decode(pdf)
  expect(text).toContain('/Count 2 /Kids [3 0 R 5 0 R]');expect(text).not.toContain('/Subtype /Image')
  expect(text).toContain('/Subtype /Type0');expect(text).toContain('/ToUnicode');expect(text).toContain(' Tj');expect(text).toContain(' l\n');expect(text).toContain(' cm\n')
  const entries=text.split('xref\n0 12\n')[1].split('trailer')[0].trim().split('\n').slice(1)
  entries.forEach((line,i)=>expect(new TextDecoder().decode(pdf.slice(Number(line.slice(0,10)),Number(line.slice(0,10))+`${i+1} 0 obj`.length))).toBe(`${i+1} 0 obj`))
 })
 it('lays out one sheet per body with explicit scale and escaped Unicode labels',()=>{
  const pages=createDrawingSheets([body('a'),body('b')],{template:'A4',scale:1,title:'Сборка',author:'A&B',perBody:true})
  expect(pages).toHaveLength(2);expect(pages[1].svg).toContain('2/2');expect(pages[0].svg).toContain('Деталь &lt;a&gt;');expect(pages[0].svg).toContain('A&amp;B');expect(pages[0].svg).toContain('Scale 1:1')
  expect(()=>createDrawingSheets([body('a')],{template:'A4',scale:100,title:'',author:'',perBody:false})).toThrow(/does not fit/)
 })
 it('recomputes vertex dimensions, preserves references and exports engineering title blocks',()=>{
  const original=body('a')
  const options={template:'A4' as const,scale:1,title:'Test',author:'Автор',perBody:false,occlusion:'brep' as const,frame:'detail' as const,drawingNumber:'ABC-001',revision:'B',material:'Steel'}
  const first=createDrawingSheets([original],options)[0]
  const reference=first.references![0]
  const model=original.brep
  const a=0,b=model.vertices.findIndex(v=>v.point[0]!==model.vertices[a].point[0])
  const dimensions=[{bodyId:'a',vertices:[a,b] as [number,number],topologySignature:reference.topologySignature,view:'top' as const,axis:'horizontal' as const}]
  const defined=createDrawingSheets([original],{...options,linkedDimensions:dimensions})[0]
  expect(defined.svg).toContain('data-dimension="linked"');expect(defined.svg).toContain('20.00 mm');expect(defined.svg).toContain('ABC-001');expect(defined.svg).toContain('Rev B');expect(defined.svg).toContain('Steel')
  const brep=createBrepBox([0,0,0],[40,10,5]);const changed={...original,brep,mesh:tessellateNurbsBrep(brep,1)}
  expect(createDrawingSheets([changed],{...options,linkedDimensions:dimensions})[0].svg).toContain('40.00 mm')
  expect(()=>createDrawingSheets([changed],{...options,linkedDimensions:[{...dimensions[0],topologySignature:'stale'}]})).toThrow(/topology changed/)
  expect(parseDrawingDimensions(JSON.stringify(dimensions))).toEqual(dimensions)
  expect(()=>parseDrawingDimensions('[{"bodyId":"a"}]')).toThrow(/Invalid/)
  expect(()=>createDrawingSheets([changed],{...options,linkedDimensions:[{...dimensions[0],bodyId:'missing'}]})).toThrow(/missing body/)
 })
 it('creates a valid multipage PDF object graph with byte-correct streams and xref',()=>{
  const pdf=drawingPdf([1,2].map(()=>({jpeg:new Uint8Array([255,216,255,217]),width:297,height:210,pixelWidth:10,pixelHeight:10})))
  const text=new TextDecoder('latin1').decode(pdf)
  expect(text).toContain('/Count 2 /Kids [3 0 R 6 0 R]');expect(text).toContain('/Length 4 >>\nstream\n')
  const offsets=text.split('xref\n0 9\n')[1].split('trailer')[0].trim().split('\n').slice(1)
  offsets.forEach((line,i)=>expect(new TextDecoder().decode(pdf.slice(Number(line.slice(0,10)),Number(line.slice(0,10))+`${i+1} 0 obj`.length))).toBe(`${i+1} 0 obj`))
 })
})

import {describe,it,expect} from 'vitest'
import {createSpecificationSheets,drawingSpecificationRows,parseDrawingParts} from '../src/services/drawingSpecification'
import type {DirectBody} from '../src/services/directModeling'
const body=(id:string,name='Деталь '+id):DirectBody=>({id,name,mesh:{positions:new Float64Array(),indices:new Uint32Array()}})
const options={template:'A4' as const,scale:1,title:'Сборка & детали',author:'Конструктор',perBody:false,specification:true}
describe('engineering specifications',()=>{
 it('counts authored occurrences under explicitly shared designations without mutating sources',()=>{
  const bodies=[body('bolt-a'),body('bolt-b'),body('housing','Корпус')],parts=['bolt-a','bolt-b'].map(bodyId=>({bodyId,designation:'M3-10',description:'Винт M3 × 10',material:'Сталь',note:'Покупная деталь'}))
  const before=JSON.stringify({bodies,parts}),rows=drawingSpecificationRows(bodies,parts)
  expect(rows).toHaveLength(2);expect(rows[0]).toMatchObject({position:1,designation:'M3-10',quantity:2,bodyIds:['bolt-a','bolt-b']});expect(rows[1]).toMatchObject({position:2,description:'Корпус',quantity:1})
  expect(JSON.stringify({bodies,parts})).toBe(before)
  expect(drawingSpecificationRows([body('a','Same name'),body('b','Same name')])).toHaveLength(2)
 })
 it('rejects conflicting declarations, dangling metadata and duplicate source bodies',()=>{
  const bodies=[body('a'),body('b')]
  expect(()=>drawingSpecificationRows(bodies,[{bodyId:'a',designation:'X',material:'Steel'},{bodyId:'b',designation:'X',material:'Aluminum'}])).toThrow(/conflicting/)
  expect(()=>drawingSpecificationRows(bodies,[{bodyId:'missing'}])).toThrow(/missing body/)
  expect(()=>drawingSpecificationRows([body('a'),body('a')])).toThrow(/distinct/)
  expect(()=>parseDrawingParts('[{"bodyId":"a"},{"bodyId":"a"}]')).toThrow(/duplicate/)
  expect(()=>parseDrawingParts(JSON.stringify([{bodyId:'a',note:'x'.repeat(121)}]))).toThrow(/oversized/)
 })
 it('paginates every body exactly once and preserves Unicode and escaped fields',()=>{
  const bodies=Array.from({length:50},(_,i)=>body('p'+i,'Деталь <'+i+'>')),parts=bodies.map(b=>({bodyId:b.id,note:'Сборочная деталь '.repeat(5)}))
  const sheets=createSpecificationSheets(bodies,{...options,parts},297,210),rows=sheets.flatMap(s=>s.specificationRows!)
  expect(sheets.length).toBeGreaterThan(1);expect(rows.map(r=>r.position)).toEqual(Array.from({length:50},(_,i)=>i+1));expect(rows.reduce((n,r)=>n+r.quantity,0)).toBe(50)
  expect(new Set(rows.flatMap(r=>r.bodyIds)).size).toBe(50)
  sheets.forEach((s,i)=>{expect(s.kind).toBe('specification');expect(s.svg).toContain('data-sheet-number="true">'+(i+1)+'/'+sheets.length);expect(s.svg).toContain('Сборка &amp; детали');expect(s.svg).not.toContain('NaN')})
  expect(sheets[0].svg).toContain('Деталь &lt;0&gt;');expect(sheets[0].svg).toContain('Конструктор')
 })
 it('keeps authored line breaks and provides bounded format/page errors',()=>{
  const sheets=createSpecificationSheets([body('a')],{...options,parts:[{bodyId:'a',note:'Первая строка\nВторая строка'}]},297,210)
  expect(sheets[0].svg).toContain('>Первая строка</text>');expect(sheets[0].svg).toContain('>Вторая строка</text>')
  expect(()=>createSpecificationSheets([body('a')],options,148,210)).toThrow(/196 mm/)
  const bodies=Array.from({length:200},(_,i)=>body('p'+i)),parts=bodies.map(b=>({bodyId:b.id,note:'Н'.repeat(120)}))
  expect(()=>createSpecificationSheets(bodies,{...options,template:'A5',parts},210,148)).toThrow(/40 sheets/)
  expect(createSpecificationSheets(bodies,{...options,specification:false},297,210)).toEqual([])
 })
})

import { expect, it } from 'vitest'
import { editorBlocks, indentSelection } from '../src/services/editorBlocks'
it('finds nested indentation and ignores delimiters inside strings/comments',()=>{
 const blocks=editorBlocks('fn f x: int\n    ret hull(\n        sphere(x), // }\n        sphere(2)\n    )\nshow sphere(1)')
 expect(blocks.find(b=>b.start===0)).toMatchObject({end:4,depth:0})
 expect(blocks.find(b=>b.start===1)).toMatchObject({end:4,depth:1})
 expect(editorBlocks('text = "{\\n}"\n// {\nshow sphere(1)')).toEqual([])
})
it('indents selections without including the next line and reverses indentation',()=>{
 const source='one\ntwo\nthree',a=indentSelection(source,0,8)
 expect(a.text).toBe('    one\n    two')
 const changed=source.slice(0,a.start)+a.text+source.slice(a.end)
 const b=indentSelection(changed,a.selectionStart,a.selectionEnd,true)
 expect(changed.slice(0,b.start)+b.text+changed.slice(b.end)).toBe(source)
})
it('inserts to a tab stop and handles tab-indented outdent',()=>{
 expect(indentSelection('ab',2,2).text).toBe('  ')
 expect(indentSelection('\tx',1,1,true)).toMatchObject({text:'x',selectionStart:0,selectionEnd:0})
})

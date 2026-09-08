import { expect, it } from 'vitest'
import { formatCode } from '../src/services/codeFormat'
import { compileModelGraphText } from '../src/services/modelGraphText'
it('uses two spaces and attaches closing parentheses without changing geometry',()=>{
 const source='// @modelgraph-text/1\nfn f x: int -> Geometry\n    ret hull(\n        sphere(x),\n        sphere(2),\n    )\nshow f(3)'
 const formatted=formatCode(source)
 expect(formatted).toContain('  ret hull(\n    sphere(x),\n    sphere(2))')
 expect(formatCode(formatted)).toBe(formatted)
 expect(compileModelGraphText(formatted).source).toBe(compileModelGraphText(source).source)
})
it('does not swallow a closing delimiter into a comment or change multiline literals',()=>{
 const source='f(\n    2 // comment\n)\ntext = "a\n    b"'
 expect(formatCode(source)).toBe('f(\n  2 // comment\n)\ntext = "a\n    b"')
})

it('attaches closing parentheses followed by chained methods',()=>{
 const source='// @modelgraph-text/1\nprofile = union(\n    rectangle([4,12]),\n    rectangle([3,24])\n).offset(0.7).offset(delta: -0.7)\nshow profile.extrude(2)'
 const formatted=formatCode(source)
 expect(formatted).toContain('rectangle([3,24])).offset(0.7).offset(delta: -0.7)')
 expect(formatCode(formatted)).toBe(formatted)
 expect(compileModelGraphText(formatted).source).toBe(compileModelGraphText(source).source)
})

import { expect, it } from 'vitest';
import { highlightCode } from '../src/services/codeHighlight';
it('escapes hostile source and keeps comments and strings opaque', () => {
 const html = highlightCode('// cube(1)\n"<img src=x onerror=alert(1)>"; /* function */');
 expect(html).not.toContain('<img');
 expect(html).not.toContain('syntax-function');
 expect(html).toContain('&lt;img');
});
it('recognizes parameters, calls, JSON keys and numeric exponents', () => {
 const html=highlightCode('gap = 0.3; translate([1e-3,0,0])cube(2); {"op":"cube"}');
 expect(html).toContain('syntax-property">gap');
 expect(html).toContain('syntax-function">translate');
 expect(html).toContain('syntax-number">1e-3');
 expect(html).toContain('syntax-property">&quot;op&quot;');
});

it('highlights generator clauses and range operators',()=>{
 const html=highlightCode('[for angle in 0deg..<360deg count 18 where angle > 0deg => body.rotate(z: angle)]')
 expect(html).toContain('syntax-keyword">count')
 expect(html).toContain('syntax-keyword">where')
 expect(html).toContain('syntax-operator">..&lt;')
})

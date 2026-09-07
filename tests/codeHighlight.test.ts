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

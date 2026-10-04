import { expect, it } from 'vitest'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { isModelGraphText, isRush } from '../src/services/modelGraphTextDetect'
import { highlightCode } from '../src/services/codeHighlight'

it('executes Rush through the rebuilt language WASM and existing CAD backend', () => {
  const body = '\nparam radius: 2mm range 1mm..8mm\nfn make x: length -> length\n  return x\nshow circle(make(radius)).extrude(4mm)'
  const rush = '// @rush/1' + body
  expect(isRush(rush)).toBe(true)
  expect(isModelGraphText(rush)).toBe(true)
  const result = compileModelGraphText(rush)
  const legacy = compileModelGraphText('// @modelgraph-text/1' + body.replace('return x', 'ret x'))
  const normalize = (document: any) => {
    const ids = new Map(document.nodes.map((node: any, index: number) => [node.id, `node${index}`]))
    return JSON.parse(JSON.stringify(document), (_key, value) => typeof value === 'string' && ids.has(value) ? ids.get(value) : value)
  }
  expect(normalize(result.document)).toEqual(normalize(legacy.document))
  expect(result.document.root).toMatch(/^r/)
  expect(result.document_sha256).not.toBe(legacy.document_sha256)
  expect(highlightCode('return value')).toContain('syntax-keyword">return</span>')
})

it('routes Rush source through the render entrypoint', async () => {
  const { parseOpenSCAD } = await import('../src/services/openscadParser')
  const result = await parseOpenSCAD('// @rush/1\nshow box(2mm,3mm,4mm)')
  expect(result.meshes).toHaveLength(1)
})

it('preserves explicit Rush part identity through parameter edits and rename', () => {
  const source='// @rush/1\nparam size = 10mm range 1mm..20mm\nbody @id("stable-body") = box([size,size,size])\nshow body'
  const before=compileModelGraphText(source)
  const after=compileModelGraphText(source.replace('10mm','12mm').replace('body @','part @').replace('show body','show part'))
  expect(after.document.root).toBe(before.document.root)
  expect(after.customizer[0].value).toBe(12)
})

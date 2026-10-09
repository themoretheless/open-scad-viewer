import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/services/rushFrontend'
import { isRushFrontend, isRush } from '../src/services/rushFrontendDetect'
import { compileRushGraph } from '../src/services/rushGraphCompiler'
import { highlightCode } from '../src/services/codeHighlight'

it('executes Rush through the rebuilt language WASM and existing CAD backend', () => {
  const body = '\nparam radius: 2mm range 1mm..8mm\nfn make x: length -> length\n  return x\nshow circle(make(radius)).extrude(4mm)'
  const rush = '// @rush/1' + body
  expect(isRush(rush)).toBe(true)
  expect(isRushFrontend(rush)).toBe(true)
  const result = compileRushFrontend(rush)
  const spelling = compileRushFrontend('// @rush/1' + body.replace('return x', 'ret x'))
  const normalize = (document: any) => {
    const ids = new Map(document.nodes.map((node: any, index: number) => [node.id, `node${index}`]))
    return JSON.parse(JSON.stringify(document), (_key, value) => typeof value === 'string' && ids.has(value) ? ids.get(value) : value)
  }
  expect(normalize(result.document)).toEqual(normalize(spelling.document))
  expect(result.document.root).toMatch(/^r/)
  expect(result.document_sha256).toBe(spelling.document_sha256)
  expect(highlightCode('return value')).toContain('syntax-keyword">return</span>')
})

it('routes Rush source through the render entrypoint', async () => {
  const { parseOpenSCAD } = await import('../src/services/openscadParser')
  const result = await parseOpenSCAD('// @rush/1\nshow box(2mm,3mm,4mm)')
  expect(result.meshes).toHaveLength(1)
})

it('preserves explicit Rush part identity through parameter edits and rename', () => {
  const source='// @rush/1\nparam size = 10mm range 1mm..20mm\nbody @id("stable-body") = box([size,size,size])\nshow body'
  const before=compileRushFrontend(source)
  const after=compileRushFrontend(source.replace('10mm','12mm').replace('body @','part @').replace('show body','show part'))
  expect(after.document.root).toBe(before.document.root)
  expect(after.customizer[0].value).toBe(12)
})

it('rejects retired source and graph formats instead of routing them as Rush', () => {
  const retired = 'model' + 'graph'
  expect(() => compileRushGraph({ language: `${retired}/1`, units: 'mm', parameters: [], nodes: [{ id: 'body', op: 'box', size: [1,2,3] }], root: 'body' })).toThrow()
  expect(isRushFrontend(`// @${retired}-text/1\nshow box(1mm,1mm,1mm)`)).toBe(false)
  expect(() => compileRushFrontend(`// @${retired}-text/1\nshow box(1mm,1mm,1mm)`)).toThrow('Unsupported Rush source header')
})

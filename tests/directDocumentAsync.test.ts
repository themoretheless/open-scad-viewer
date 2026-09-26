import { stringifyMeshJson } from '../src/services/meshJson'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { emptyDirectDocument, parseDirectDocument, parseDirectDocumentAsync } from '../src/services/directModeling'
import { createBrepBox, tessellateNurbsBrep } from '../src/services/geometry/brep'

afterEach(() => vi.restoreAllMocks())

const exactBox = () => {
  const brep = createBrepBox([0, 0, 0], [2, 3, 4])
  const { positions, indices } = tessellateNurbsBrep(brep)
  const mesh = { positions, indices }
  return { id: 'box', name: 'Box', mesh, brep }
}

describe('cooperative Solid document validation', () => {
  it('preserves the synchronous normalized document and detached ownership', async () => {
    const document = emptyDirectDocument()
    document.bodies.push(exactBox())
    document.sketches.push({ id: 'sketch', name: 'Sketch', points: [[0, 0], [2, 0], [0, 2]], closed: true })
    const text = stringifyMeshJson(document)
    const expected = parseDirectDocument(text)
    const actual = await parseDirectDocumentAsync(text)
    expect(actual).toEqual(expected)
    actual.bodies[0].mesh.positions[0] = 999
    expect(document.bodies[0].mesh.positions[0]).toBe(0)
    expect(parseDirectDocument(text)).toEqual(expected)
  })

  it.each(['bad mesh', 'bad topology', 'bad bounds', 'duplicate id', 'bad group'])('preserves refusal for %s', async corruption => {
    const document = emptyDirectDocument()
    const body = exactBox()
    document.bodies.push(body)
    if (corruption === 'bad mesh') body.mesh.indices[0] = -1
    if (corruption === 'bad topology') body.brep.edges[0].vertices[0] = 999999
    if (corruption === 'bad bounds') body.mesh.positions[0] = 999
    if (corruption === 'duplicate id') document.bodies.push(body)
    if (corruption === 'bad group') document.groups = [{ name: '', source: '' }]
    const text = stringifyMeshJson(document)
    let expected: unknown
    try { parseDirectDocument(text) } catch (error) { expected = error }
    expect(expected).toBeInstanceOf(Error)
    await expect(parseDirectDocumentAsync(text)).rejects.toThrow((expected as Error).message)
  })

  it('yields between objects and resumes without skipping checks', async () => {
    let now = 0
    vi.spyOn(performance, 'now').mockImplementation(() => now += 10)
    const document = emptyDirectDocument()
    document.bodies = [exactBox(), { ...exactBox(), id: 'second' }]
    const yieldControl = vi.fn(async () => {})
    const text = stringifyMeshJson(document)
    expect(await parseDirectDocumentAsync(text, { yieldControl })).toEqual(parseDirectDocument(text))
    expect(yieldControl).toHaveBeenCalledTimes(2)
  })

  it('cancels before parsing or between bodies without publishing a partial result', async () => {
    const controller = new AbortController()
    let now = 0
    vi.spyOn(performance, 'now').mockImplementation(() => now += 10)
    const document = emptyDirectDocument()
    document.bodies = [exactBox(), { ...exactBox(), id: 'second' }]
    document.bodies[1].mesh.indices[0] = -1
    const yieldControl = vi.fn(async () => { controller.abort() })
    await expect(parseDirectDocumentAsync(stringifyMeshJson(document), { signal: controller.signal, yieldControl }))
      .rejects.toMatchObject({ name: 'AbortError' })
    expect(yieldControl).toHaveBeenCalledOnce()
    await expect(parseDirectDocumentAsync('invalid json', { signal: controller.signal }))
      .rejects.toMatchObject({ name: 'AbortError' })
  })
})

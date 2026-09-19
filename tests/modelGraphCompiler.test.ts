import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import * as graph from '../src/services/modelGraph'
import * as graphRuntime from '../src/services/modelGraphCompiler'
import * as nurbs from '../src/services/modelGraphNurbs'
import * as nurbsRuntime from '../src/services/modelGraphNurbsCompiler'
import { compileModelGraphText } from '../src/services/modelGraphText'

it('preserves facade function and error constructor identity', () => {
  expect(graph.compileModelGraph).toBe(graphRuntime.compileModelGraph)
  expect(graph.ModelGraphError).toBe(graphRuntime.ModelGraphError)
  expect(graph.hashModelGraphDocument).toBe(graphRuntime.hashModelGraphDocument)
  expect(nurbs.compileModelGraphNurbs).toBe(nurbsRuntime.compileModelGraphNurbs)
  expect(nurbs.ModelGraphNurbsError).toBe(nurbsRuntime.ModelGraphNurbsError)
  expect(nurbs.hashNurbsDocument).toBe(nurbsRuntime.hashNurbsDocument)
  expect(() => graphRuntime.compileModelGraph({})).toThrow(graph.ModelGraphError)
  expect(() => nurbsRuntime.compileModelGraphNurbs({})).toThrow(nurbs.ModelGraphNurbsError)
})

it('preserves recorded document revision hashes across the schema/runtime split', () => {
  expect(compileModelGraphText('// @modelgraph-text/1\nshow box(2mm,3mm,4mm)').document_sha256)
    .toBe('2a9ffdc3d69127af444832f241b7adade1f250bd948bed5bc1b90b4656054205')
  const spinner = readFileSync(new URL('../examples/modelgraph-text/planetary-spinner.mg', import.meta.url), 'utf8')
  expect(compileModelGraphText(spinner).document_sha256)
    .toBe('7dbf42db933b48ec0eb6443d1dc1f325e9410a296cd3b7a0d36256072887db08')
  expect(nurbsRuntime.compileModelGraphNurbs(nurbs.MODELGRAPH_NURBS_EXAMPLE).document_sha256)
    .toBe('aac856d8cea3faf64f952ab0869d429f4b8304f7a80bca78ca2aeb1596fe31ac')
  expect(nurbsRuntime.compileModelGraphNurbs(nurbs.MODELGRAPH_NURBS_SURFACE_EXAMPLE).document_sha256)
    .toBe('39ba6a458e6141e4bd1b0965bbac6c7958435dcbe49db6aba2b4b31ccaf3f767')
})

it('retains structured Rust diagnostics through the lightweight compiler', () => {
  try {
    nurbsRuntime.compileModelGraphNurbs({})
    throw new Error('Expected invalid document refusal')
  } catch (error) {
    expect(error).toBeInstanceOf(nurbs.ModelGraphNurbsError)
    expect(error).toMatchObject({ name: 'ModelGraphNurbsError', code: 'invalid_document', path: '/language', message: 'Required field is missing.' })
  }
})

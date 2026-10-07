import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import * as graph from '../src/services/rushGraph'
import * as graphRuntime from '../src/services/rushGraphCompiler'
import * as nurbs from '../src/services/rushGraphNurbs'
import * as nurbsRuntime from '../src/services/rushGraphNurbsCompiler'
import { compileRushFrontend } from '../src/services/rushFrontend'

it('preserves facade function and error constructor identity', () => {
  expect(graph.compileRushGraph).toBe(graphRuntime.compileRushGraph)
  expect(graph.RushGraphError).toBe(graphRuntime.RushGraphError)
  expect(graph.hashRushGraphDocument).toBe(graphRuntime.hashRushGraphDocument)
  expect(nurbs.compileRushGraphNurbs).toBe(nurbsRuntime.compileRushGraphNurbs)
  expect(nurbs.RushGraphNurbsError).toBe(nurbsRuntime.RushGraphNurbsError)
  expect(nurbs.hashNurbsDocument).toBe(nurbsRuntime.hashNurbsDocument)
  expect(() => graphRuntime.compileRushGraph({})).toThrow(graph.RushGraphError)
  expect(() => nurbsRuntime.compileRushGraphNurbs({})).toThrow(nurbs.RushGraphNurbsError)
})

it('pins Rush IR revision hashes after the language migration', () => {
  expect(compileRushFrontend('// @rush/1\nshow box(2mm,3mm,4mm)').document_sha256)
    .toBe('3a4bc71989f1ab9f9cdf4f4875101272e224db01117eae249c8b76c8e4978f19')
  const spinner = readFileSync(new URL('../examples/rush-frontend/planetary-spinner.r', import.meta.url), 'utf8')
  expect(compileRushFrontend(spinner).document_sha256)
    .toBe('ce388dd321a66e3b3f4cbf34cf4661393ffaaf4c2d70d38989a39cf29cc4fa77')
  expect(nurbsRuntime.compileRushGraphNurbs(nurbs.RUSH_GRAPH_NURBS_EXAMPLE).document_sha256)
    .toBe('8ac6d121f6991e21446d0ed24eae327780a2dd2dfa7c625884b7e220e0a31284')
  expect(nurbsRuntime.compileRushGraphNurbs(nurbs.RUSH_GRAPH_NURBS_SURFACE_EXAMPLE).document_sha256)
    .toBe('77d1397ff8a29dd16ec79fb7525f56749a9f4e6ef2676666bf58ed636e8c1f4a')
})

it('retains structured Rust diagnostics through the lightweight compiler', () => {
  try {
    nurbsRuntime.compileRushGraphNurbs({})
    throw new Error('Expected invalid document refusal')
  } catch (error) {
    expect(error).toBeInstanceOf(nurbs.RushGraphNurbsError)
    expect(error).toMatchObject({ name: 'RushGraphNurbsError', code: 'invalid_document', path: '/language', message: 'Required field is missing.' })
  }
})

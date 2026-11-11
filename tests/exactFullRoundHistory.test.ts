import { describe, expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
// Candidate qualification executes real Rust. Ordinary runs use the installed package.
vi.mock('../src/kernel/geometry/kernelCompilation', async original => {
  const path=process.env.OWN_HISTORY_WASM
  if(!path)return original()
  const report=JSON.parse(readFileSync(new URL('../docs/qualification/full-round-history-bridge-wasm-2026-11-02/report.json',import.meta.url),'utf8'))
  const bytes=readFileSync(path)
  if(createHash('sha256').update(bytes).digest('hex')!==report.wasmSha256)throw new Error('Unqualified history WASM')
  const module=new WebAssembly.Module(bytes)
  return {compileGeometryKernelArtifactSync:()=>module,compileGeometryKernelArtifact:async()=>module}
})
import {restoreExactFullRoundHistory,editExactFullRoundHistory,currentExactFullRoundArtifact} from '../src/kernel/exactFullRoundHistory'
const fixture=JSON.parse(readFileSync(new URL('../docs/qualification/full-round-history-bridge-wasm-2026-11-02/native-fixture.json',import.meta.url),'utf8'))
describe('native exact history transport',()=>{
  it('restores persisted history, freezes ownership and selects the document cursor',()=>{
    const state=restoreExactFullRoundHistory(JSON.parse(JSON.stringify(fixture.history)))
    expect(state.document).toEqual(fixture.history)
    expect(state.work).toEqual(fixture.work)
    expect(Object.isFrozen(currentExactFullRoundArtifact(state))).toBe(true)
    const next=editExactFullRoundHistory(state,{action:'redo'})
    expect(currentExactFullRoundArtifact(next)).not.toEqual(currentExactFullRoundArtifact(state))
    expect(editExactFullRoundHistory(next,{action:'undo'}).document).toEqual(state.document)
    expect(state.document.cursor).toBe(0)
  },30000)
  it('does not let metadata authorize a corrupted hidden redo revision',()=>{
    const forged=structuredClone(fixture.history)
    forged.revisions[1].artifact.geometry.vertices[0].point[0]=['999','1']
    expect(()=>restoreExactFullRoundHistory(forged)).toThrow()
    expect(()=>restoreExactFullRoundHistory(fixture.history,{maxBytes:1})).toThrow()
  },30000)
})

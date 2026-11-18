import { mkdirSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { lowerOpenSCADToSemanticProgram } from '../src/language/semantic/semanticProgramLowerer.ts'
import { encodeSemanticProgram, decodeSemanticProgram } from '../src/language/semantic/semanticProgramCodec.ts'
import assert from 'node:assert/strict'
const directory = resolve(process.argv[2] ?? 'docs/qualification/full-round-host-contract-2026-11-02')
mkdirSync(directory, { recursive: true })
const source = '// @language openscad-viewer/brep-1\n// @requires geometry.brep\nfull_round_exact(faces=[5,4,3]) cube([4,8,6]);\n'
const artifact = lowerOpenSCADToSemanticProgram(source)
const program = artifact.program
assert.deepEqual(decodeSemanticProgram(encodeSemanticProgram(program)), program)
const request = { ...program.core, op: 'brep_graph', envelope: program, source: program.source, sourceText: source,
  provenance: program.provenance, tessellationIntents: program.tessellationIntents, diagnostics: program.diagnostics,
  maxNodes: 4, maxBytes: 4194304 }
writeFileSync(resolve(directory, 'source.scad'), source)
writeFileSync(resolve(directory, 'source-program.json'), JSON.stringify(program, null, 2) + '\n')
writeFileSync(resolve(directory, 'native-request.json'), JSON.stringify(request, null, 2) + '\n')
console.log(JSON.stringify({ nodes: program.core.nodes.length, operations: program.core.operations.length, sourceSha256: program.source.sha256, codecRoundtrip: true }))

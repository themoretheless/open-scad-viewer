import { describe, expect, it } from 'vitest'
import {
  OPENSCAD_2021_01_BUILTIN_FUNCTIONS,
  OPENSCAD_2021_01_FUNCTION_COUNT,
} from '../src/core/openScad2021Contract'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'

const fullProfile = { languageProfile: 'openscad/stable-2021.01' as const }

describe('independent OpenSCAD 2021.01 function conformance', () => {
  it('executes all 38 stable built-in smoke programs without the upstream runtime', async () => {
    expect(OPENSCAD_2021_01_BUILTIN_FUNCTIONS).toHaveLength(OPENSCAD_2021_01_FUNCTION_COUNT)

    for (const entry of OPENSCAD_2021_01_BUILTIN_FUNCTIONS) {
      const direct = await parseOpenSCAD(entry.smoke.source, fullProfile)
      expect(direct.meshes, entry.name).toHaveLength(1)
      expect(direct.warnings.some(message => message.startsWith('ECHO:')), entry.name).toBe(true)

      const semantic = lowerOpenSCADToSemanticProgram(entry.smoke.source, fullProfile)
      expect(semantic.terminalError, entry.name).toBeNull()
      expect(semantic.program.core.nodes.length, entry.name).toBeGreaterThan(0)
      expect(semantic.warnings.some(message => message.startsWith('ECHO:')), entry.name).toBe(true)
    }
  })

  it('executes recursive and first-class user functions through both independent paths', async () => {
    const source = `
      function factorial(n, acc = 1) = n <= 1 ? acc : factorial(n - 1, acc * n);
      increment = function(value) value + 1;
      echo(factorial(5), increment(6), is_function(increment));
      cube([factorial(3), increment(2), [2, 3, 4].z]);
    `

    const direct = await parseOpenSCAD(source, fullProfile)
    expect(direct.volume).toBeCloseTo(72, 6)
    expect(direct.warnings).toContain('ECHO: 120, 7, true')

    const semantic = lowerOpenSCADToSemanticProgram(source, fullProfile)
    expect(semantic.warnings).toContain('ECHO: 120, 7, true')
    expect(semantic.program.core.nodes).toHaveLength(1)
  })
})

import { describe, expect, it } from 'vitest'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'

const stableProfile = { languageProfile: 'openscad/stable-2021.01' as const }

const source = `
  echo(
    version(assert(false, "version argument evaluated") 0),
    version_num(
      echo("version_num argument") [2021, 1],
      assert(false, "version_num surplus argument evaluated") 0
    ),
    version_num(
      ignored_name=echo("version_num named first") [2021, 1],
      version=assert(false, "version_num named second evaluated") [2021, 1]
    ),
    abs(
      assert(false, "abs wrong-arity argument evaluated") 0,
      assert(false, "abs wrong-arity argument evaluated") 0
    ),
    pow(assert(false, "pow wrong-arity argument evaluated") 0),
    lookup(assert(false, "lookup wrong-arity argument evaluated") 0),
    norm(
      assert(false, "norm wrong-arity argument evaluated") [1],
      assert(false, "norm wrong-arity argument evaluated") [2]
    ),
    is_num(
      assert(false, "predicate wrong-arity argument evaluated") 1,
      assert(false, "predicate wrong-arity argument evaluated") 2
    ),
    pow(echo("pow left") "bad", echo("pow right") 2),
    lookup(
      echo("lookup first") "bad",
      assert(false, "lookup table evaluated") []
    ),
    log(
      echo("log first") "bad",
      assert(false, "log second evaluated") 2
    ),
    search(
      echo("search 0") "a",
      echo("search 1") "a",
      echo("search 2") 1,
      echo("search 3") 0,
      assert(false, "search surplus evaluated") 0
    ),
    ord(
      assert(false, "ord wrong-arity argument evaluated") "a",
      assert(false, "ord wrong-arity argument evaluated") "b"
    ),
    cross(echo("cross left") "bad", echo("cross right") [1, 2])
  );
  cube(1);
`

const expectedWarnings = [
  'ECHO: "version_num argument"',
  'ECHO: "version_num named first"',
  'abs() expects 1 argument',
  'pow() expects 2 arguments',
  'lookup() expects 2 arguments',
  'norm() expects 1 argument',
  'is_num() expects 1 argument',
  'ECHO: "pow left"',
  'ECHO: "pow right"',
  'pow() argument 1 must be a number',
  'ECHO: "lookup first"',
  'ECHO: "lookup first"',
  'lookup() argument 1 must be a number',
  'ECHO: "log first"',
  'log() argument 1 must be a number',
  'ECHO: "search 0"',
  'ECHO: "search 1"',
  'ECHO: "search 2"',
  'ECHO: "search 3"',
  'ord() expects 1 argument',
  'ECHO: "cross left"',
  'ECHO: "cross right"',
  'cross() argument 1 must be a vector',
  'ECHO: [2021, 1, 0], 20210100, 20210100, undef, undef, undef, undef, undef, undef, undef, undef, [0], undef, undef',
]

describe('OpenSCAD 2021.01 call-by-name built-in arguments', () => {
  it('matches argument reads and ordering in the direct evaluator', async () => {
    const result = await parseOpenSCAD(source, stableProfile)

    expect(result.warnings).toEqual(expectedWarnings)
    expect(result.volume).toBeCloseTo(1, 6)
  })

  it('matches argument reads and ordering in the semantic lowerer', () => {
    const result = lowerOpenSCADToSemanticProgram(source, stableProfile)

    expect(result.terminalError).toBeNull()
    expect(result.warnings).toEqual(expectedWarnings)
    expect(result.program.core.nodes.map(node => node.kind)).toEqual(['box'])
  })
})

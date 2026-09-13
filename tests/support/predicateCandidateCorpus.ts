/** Candidate corpus only: independently evaluates authored leaves with bigint
 * rationals. This module must not import a production predicate implementation.
 */
import {
  compareSquaredDistanceExact, orient2dExact, orient3dExact, rational,
  rationalFromBinary64, type BigRational, type Sign,
} from './referencePredicateOracleV1'

export type CandidateLeaf = {readonly kind: 'binary64'; readonly bits: string}
  | {readonly kind: 'rational'; readonly numerator: string; readonly denominator: string}
export interface CandidatePredicateCase {
  readonly id: string
  readonly predicate: 'orient2d' | 'orient3d' | 'compare_squared_distance'
  readonly dimension: 2 | 3
  readonly leaves: readonly CandidateLeaf[]
  readonly expectedSign: Sign
  /** Finite floating expansions may explicitly refuse exponent extremes. */
  readonly mustDecide: boolean
  readonly tags: readonly string[]
}

function bits(value: number): CandidateLeaf {
  const bytes = new DataView(new ArrayBuffer(8))
  bytes.setFloat64(0, value, false)
  if (!Number.isFinite(value)) throw new Error('Corpus leaves must be finite')
  return {kind: 'binary64', bits: bytes.getBigUint64(0, false).toString(16).padStart(16, '0')}
}
function constant(numerator: bigint, denominator = 1n): CandidateLeaf {
  const reduced = rational(numerator, denominator)
  return {kind: 'rational', numerator: String(reduced.n), denominator: String(reduced.d)}
}
function exact(leaf: CandidateLeaf): BigRational {
  return leaf.kind === 'binary64' ? rationalFromBinary64(BigInt('0x' + leaf.bits))
    : rational(BigInt(leaf.numerator), BigInt(leaf.denominator))
}
export function candidateExactSign(test: Omit<CandidatePredicateCase, 'expectedSign'>): Sign {
  const leaves = test.leaves.map(exact)
  if (test.predicate === 'orient2d') {
    if (leaves.length !== 6 || test.dimension !== 2) throw new Error('Invalid orient2d fixture')
    return orient2dExact(leaves[0], leaves[1], leaves[2], leaves[3], leaves[4], leaves[5])
  }
  if (test.predicate === 'orient3d') {
    if (leaves.length !== 12 || test.dimension !== 3) throw new Error('Invalid orient3d fixture')
    return orient3dExact([leaves[0], leaves[1], leaves[2]], [leaves[3], leaves[4], leaves[5]],
      [leaves[6], leaves[7], leaves[8]], [leaves[9], leaves[10], leaves[11]])
  }
  if (leaves.length !== test.dimension * 2 + 1) throw new Error('Invalid distance fixture')
  return compareSquaredDistanceExact(leaves.slice(0, test.dimension),
    leaves.slice(test.dimension, test.dimension * 2), leaves[leaves.length - 1])
}

function permutations<T>(items: readonly T[]): T[][] {
  return items.length ? items.flatMap((item, i) => permutations(items.filter((_, j) => i !== j)).map(rest => [item, ...rest])) : [[]]
}

export function generatePredicateCandidateCorpus(): readonly CandidatePredicateCase[] {
  const cases: CandidatePredicateCase[] = []
  const put = (test: Omit<CandidatePredicateCase, 'expectedSign'>) => cases.push({...test, expectedSign: candidateExactSign(test)})
  const orient = (id: string, dimension: 2 | 3, coordinates: readonly number[], mustDecide: boolean,
    tags: readonly string[], permute = false) => {
    const leaves = coordinates.map(bits), pointCount = dimension + 1
    const orders = permute ? permutations(Array.from({length: pointCount}, (_, i) => i))
      : [Array.from({length: pointCount}, (_, i) => i)]
    for (const order of orders) put({id: id + (permute ? '-' + order.join('') : ''), dimension,
      predicate: dimension === 2 ? 'orient2d' : 'orient3d',
      leaves: order.flatMap(i => leaves.slice(i * dimension, (i + 1) * dimension)), mustDecide, tags})
  }
  const distance = (id: string, dimension: 2 | 3, coordinates: readonly number[], mustDecide: boolean,
    tags: readonly string[]) => {
    const leaves = coordinates.map(bits)
    put({id, predicate: 'compare_squared_distance', dimension, leaves, mustDecide, tags})
    put({id: id + '-swap', predicate: 'compare_squared_distance', dimension,
      leaves: [...leaves.slice(dimension, dimension * 2), ...leaves.slice(0, dimension), leaves[dimension * 2]], mustDecide, tags})
  }
  const min = Number.MIN_VALUE, max = Number.MAX_VALUE, ulp = Number.EPSILON
  orient('o2-unit', 2, [0, 0, 1, 0, 0, 1], true, ['permutation', 'easy'], true)
  orient('o2-cancel-one-bit', 2, [0, 0, 1 + ulp, 1, 1, 1 - ulp], true, ['permutation', 'cancellation', 'exact-stage'], true)
  orient('o2-collinear', 2, [0, 0, 1, 1, 2, 2], true, ['permutation', 'exact-zero'], true)
  orient('o2-translated-integer', 2, [2 ** 52, 2 ** 52, 2 ** 52 + 1, 2 ** 52 + 1, 2 ** 52 + 2, 2 ** 52 + 3], true, ['translation', 'cancellation'])
  orient('o2-signed-zero', 2, [-0, 0, 1, -0, 0, 1], true, ['signed-zero'])
  orient('o2-subnormal-square', 2, [0, 0, min, 0, 0, min], false, ['subnormal', 'underflow', 'must-not-zero'])
  orient('o2-underflow-square', 2, [0, 0, 2 ** -600, 0, 0, 2 ** -600], false, ['underflow', 'must-not-zero'])
  orient('o2-overflow-square', 2, [0, 0, 2 ** 600, 0, 0, 2 ** 600], false, ['overflow'])
  orient('o2-max-span', 2, [max, max, -max, max, max, -max], false, ['overflow', 'subtraction-overflow'])
  orient('o2-mixed-exponents', 2, [0, 0, 2 ** 900, 0, 0, 2 ** -900], false, ['mixed-exponents'])
  orient('o2-overflow-collinear', 2, [0, 0, max / 2, max / 2, max, max], false, ['overflow', 'exact-zero'])
  orient('o3-unit', 3, [0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1], true, ['permutation', 'handedness'], true)
  orient('o3-cancel-one-bit', 3, [0, 0, 0, 1 + ulp, 1, 0, 1, 1 - ulp, 0, 0, 0, 1], true, ['cancellation', 'exact-stage', 'permutation'], true)
  orient('o3-dense-cancellation', 3, [0, 0, 0, 1, 1, 1, 1 + ulp, 1, 1, 1, 1 + ulp, 1], true,
    ['cancellation', 'exact-stage', 'all-components', 'permutation'], true)
  orient('o3-coplanar', 3, [0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 1, 2], true, ['exact-zero', 'coplanar'])
  orient('o3-near-plane', 3, [0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 1, 2 + 2 * ulp], true, ['near-coplanar'])
  orient('o3-subnormal-cube', 3, [0, 0, 0, min, 0, 0, 0, min, 0, 0, 0, min], false, ['subnormal', 'underflow', 'must-not-zero'])
  orient('o3-underflow-cube', 3, [0, 0, 0, 2 ** -400, 0, 0, 0, 2 ** -400, 0, 0, 0, 2 ** -400], false, ['underflow', 'must-not-zero'])
  orient('o3-overflow-cube', 3, [0, 0, 0, 2 ** 400, 0, 0, 0, 2 ** 400, 0, 0, 0, 2 ** 400], false, ['overflow'])
  orient('o3-max-span', 3, [max, max, max, -max, max, max, max, -max, max, max, max, -max], false, ['overflow', 'subtraction-overflow'])
  orient('o3-mixed-exponents', 3, [0, 0, 0, 2 ** 900, 0, 0, 0, 2 ** -900, 0, 0, 0, min], false, ['subnormal', 'mixed-exponents'])
  orient('o3-overflow-coplanar', 3, [0, 0, 0, max, 0, 0, 0, max, 0, max, max, 0], false, ['overflow', 'exact-zero'])
  distance('d2-equal', 2, [0, 0, 3, 4, 5], true, ['exact-zero', 'symmetry'])
  distance('d2-outside', 2, [0, 0, 3, 4, 4], true, ['positive', 'symmetry'])
  distance('d2-inside', 2, [0, 0, 3, 4, 6], true, ['negative', 'symmetry'])
  distance('d2-ulp-outside', 2, [0, 0, 1 + ulp, 0, 1], true, ['near-equality'])
  distance('d2-ulp-inside', 2, [0, 0, 1 - ulp / 2, 0, 1], true, ['near-equality'])
  distance('d2-min-outside', 2, [0, 0, min, 0, 0], false, ['subnormal', 'underflow', 'must-not-zero'])
  distance('d2-overflow-equality', 2, [0, 0, 2 ** 600, 0, 2 ** 600], false, ['overflow', 'exact-zero'])
  distance('d2-overflow-outside', 2, [max, 0, -max, 0, max], false, ['overflow', 'subtraction-overflow'])
  distance('d3-equal', 3, [0, 0, 0, 1, 2, 2, 3], true, ['exact-zero', 'symmetry'])
  distance('d3-outside', 3, [0, 0, 0, 1, 2, 2, 2], true, ['positive', 'symmetry'])
  distance('d3-inside', 3, [0, 0, 0, 1, 2, 2, 4], true, ['negative', 'symmetry'])
  distance('d3-cancel-one-bit', 3, [0, 0, 0, 1 + ulp, 2, 2, 3], true, ['near-equality', 'all-components'])
  distance('d3-translated-equality', 3,
    [2 ** 52, 2 ** 52, 2 ** 52, 2 ** 52 + 1, 2 ** 52 + 2, 2 ** 52 + 2, 3], true,
    ['translation', 'exact-zero', 'all-components'])
  const zero = constant(0n), third = constant(1n, 3n), seventh = constant(1n, 7n)
  put({id: 'o2-rational-thirds', predicate: 'orient2d', dimension: 2,
    leaves: [zero, zero, third, zero, zero, seventh], mustDecide: true, tags: ['canonical-rational']})
  put({id: 'o3-rational-thirds', predicate: 'orient3d', dimension: 3,
    leaves: [zero, zero, zero, third, zero, zero, zero, seventh, zero, zero, zero, constant(-1n, 11n)],
    mustDecide: true, tags: ['canonical-rational']})
  put({id: 'd2-rational-equality', predicate: 'compare_squared_distance', dimension: 2,
    leaves: [zero, zero, third, zero, third], mustDecide: true, tags: ['canonical-rational', 'exact-zero']})
  put({id: 'd3-rational-inside', predicate: 'compare_squared_distance', dimension: 3,
    leaves: [zero, zero, zero, third, third, third, constant(2n, 3n)],
    mustDecide: true, tags: ['canonical-rational']})
  const binaryThird: CandidateLeaf = {kind: 'binary64', bits: '3fd5555555555555'}
  put({id: 'd2-binary-third-vs-rational', predicate: 'compare_squared_distance', dimension: 2,
    leaves: [zero, zero, binaryThird, zero, third], mustDecide: true,
    tags: ['mixed-leaf-types', 'must-not-round-rational']})
  put({id: 'd2-rational-third-vs-binary', predicate: 'compare_squared_distance', dimension: 2,
    leaves: [zero, zero, third, zero, binaryThird], mustDecide: true,
    tags: ['mixed-leaf-types', 'must-not-round-rational']})
  put({id: 'o2-rational-third-vs-binary', predicate: 'orient2d', dimension: 2,
    leaves: [zero, zero, third, binaryThird, binaryThird, third], mustDecide: true,
    tags: ['mixed-leaf-types', 'must-not-round-rational']})
  put({id: 'd2-u64-rational-boundary', predicate: 'compare_squared_distance', dimension: 2,
    leaves: [zero, zero, bits(0.5), zero, constant(9223372036854775807n, 18446744073709551615n)],
    mustDecide: true, tags: ['canonical-rational', 'integer-width', 'must-not-round-rational']})
  put({id: 'o2-i64-min-rational', predicate: 'orient2d', dimension: 2,
    leaves: [zero, zero, constant(-9223372036854775808n), zero, zero, constant(1n)],
    mustDecide: true, tags: ['canonical-rational', 'integer-width']})
  // Deterministic bounded binary64 leaves. No expected sign is calculated with
  // rounded determinant arithmetic, including these near-collinear variations.
  let state = 0x6f726163
  for (let i = 0; i < 32; i++) {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0
    const x = 1 + (state & 255) * ulp
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0
    const y = 1 + (state & 255) * ulp
    orient(`o2-seeded-${i}`, 2, [0, 0, x, y, y, 1 + ((state >>> 8) & 255) * ulp], true, ['seeded', 'near-collinear'])
  }
  return cases
}

export function renderPredicateCandidateJson(): string {
  return JSON.stringify({schema: 'cad-predicates-candidate-corpus-v1', claim: 'candidate-only-not-qualification',
    oracle: 'tests/support/referencePredicateOracleV1.ts (independent bigint rational arithmetic)',
    cases: generatePredicateCandidateCorpus()}, null, 2) + '\n'
}

/** Include file uses only test-local data structures; no production operations. */
export function renderPredicateCandidateRust(): string {
  const lines = ['// Generated by scripts/generate-predicate-candidate-corpus.ts; candidate evidence only.',
    '// Expected signs come exclusively from the independent bigint rational oracle.',
    'const CANDIDATE_CORPUS: &[CandidateCase] = &[']
  for (const test of generatePredicateCandidateCorpus()) {
    const leaves = test.leaves.map(leaf => leaf.kind === 'binary64' ? `SourceLeaf::Binary64(0x${leaf.bits})`
      : `SourceLeaf::Rational(${leaf.numerator}, ${leaf.denominator})`).join(', ')
    const predicate = {orient2d: 'Orient2d', orient3d: 'Orient3d', compare_squared_distance: 'SquaredDistance'}[test.predicate]
    lines.push(`    CandidateCase { id: ${JSON.stringify(test.id)}, predicate: CandidatePredicate::${predicate}, dimension: ${test.dimension}, leaves: &[${leaves}], expected: ${test.expectedSign}, must_decide: ${test.mustDecide} },`)
  }
  lines.push('];', '')
  return lines.join('\n')
}

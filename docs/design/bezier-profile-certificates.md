# Checkable Rust certificates for retained Bézier profiles

Version 1 certifies **topology of the exact represented planar profile**.
It does not certify a prior construction/Boolean operation, the numerical area
reported by the editor, tessellation, or three-dimensional solid validity.
The existing `geometryStatus: numerical_uncertified` therefore remains valid.

## Interface

- Rust: `brep_core::profile_certificate::{certify, verify}`.
- Geometry ABI: `brep_profile_certify` and `brep_profile_verify_certificate`.
- TypeScript transport: `certifyBrepProfile`, `verifyBrepProfileCertificate`.
- Standalone checker: `crates/brep-core/examples/verify_profile_certificate.rs`.
  It reads one JSON request on stdin: `loops`, `toleranceMm`, `certificate`.
  The optional argument `certify` generates a certificate; the default
  `verify` checks a supplied certificate. Failure exits with status 1.

The checker returns an opaque Rust `VerifiedProfile` only after checking every
claim. The wire response has scope `represented-profile-topology`. A caller
must verify again against the current source after editing or loading it.
Persisting a status string is not a verification step.

## Source and arithmetic

Every finite binary64 coordinate, weight and knot is converted to its exact
integer ratio, including subnormal inputs. Structural layout, degree,
periodicity and tolerance bits are bound in a canonical source string.
Hexadecimal bit strings preserve signed zero and avoid JavaScript integer
rounding. No hash collision assumption is needed.

The certificate concerns the supplied stored definition, which may itself be a
rounded result of an earlier operation. It does not turn that snapshot into an
authenticated original authored input and does not assert equivalence to its
construction history.

The checker uses `num-bigint`/`num-rational`, not the numerical profile
validator, tessellator, generic intersection driver, or the expansion-based
production root finder. It shares exact-arithmetic helper routines with witness
generation, but does not trust that search, its intermediate decisions or
caller flags. Schema, version, source binding, every witness, witness census,
orientation and nesting are checked anew.

## Checked claims and soundness

1. **Representation and closure.** Positive-weight nonperiodic clamped planar
   spans of degree 1..3 are admitted. Every endpoint join is an exact rational
   equality. Empty material has a complete empty certificate.
2. **Exact subdivision.** Homogeneous de Casteljau at 1/2 partitions each
   original span without generating rounded Euclidean poles or knot values.
3. **Span injectivity.** Each piece supplies a coordinate. For its rational
   derivative numerator `X'W-XW'`, exact polynomial GCD and Yun square-free
   decomposition identify odd root multiplicities; a rational Sturm chain
   excludes odd interior roots. The first nonzero coefficient and endpoint
   delta establish the sign. The nonzero numerator has only finitely many
   stationary zeros, so the coordinate is strictly monotone. Even zeros and
   stationary endpoints are allowed.
4. **All-pair separation.** There is exactly one witness for every unordered
   pair of pieces. An exact supporting projection separates positive-weight
   control hulls. For adjacent pieces, a weak separator is allowed only when
   open spans strictly leave the support plane or the other span has only the
   shared endpoint on it. Flat/flat and extra endpoint contacts are rejected.
5. **Algebraic cusp joints.** Sentinel 65535 requests an independent rational
   implicit-polynomial check for same-wire pairs whose hulls overlap. Exact
   common homogeneous factors are cancelled; the denominator of the original
   positive-weight curve has no finite zero. A rational moving-line nullspace
   and the resulting implicit identity are checked exactly. Sturm excludes
   every open-domain eliminant root, including repeated roots. Every endpoint
   eliminant zero must be an allowed exact endpoint equality. Extra eliminant
   roots cause refusal, not dismissal. This is a sufficient, conservative
   proof; it does not claim that every eliminant root is a finite intersection.
6. **Orientation.** If every pair has a hull witness, replacing each piece by
   its chord is a simultaneous injective homotopy: its monotone coordinate
   stays monotone and its image stays in its hull. Exact polygon area therefore
   gives the original orientation. Algebraic witnesses require a separate
   exact Green-integral sign proof. Polynomial contributions are integrated
   as rational polynomials. Rational contributions use complete homogeneous
   Bernstein range bounds, never samples; integer outward rounding to a
   256-bit dyadic grid bounds the summed interval.
7. **Nesting.** Different-wire hulls must be strictly separated. Thus another
   wire's query vertex stays outside every span during chord replacement,
   preserving winding even when a same-wire pair uses an algebraic witness.
   Exact half-open polygon winding determines depth; material-left orientation
   is checked against parity.

Strictly monotone pieces and the complete all-pair census imply simple closed
wires. The verified orientation and nesting define a regular planar material
region. These implications assume correctness of the Rust checker, integer and
rational arithmetic libraries, compiler and runtime. This is a checkable Rust
certificate, not a proof assistant extraction or a third-party certification.

## Refusal and bounded work

The checker accepts at most 32 wires, 128 source spans and 256 exact pieces,
with at most eight geometric subdivision levels. Rational orientation permits
at most 8192 cells and 12 levels. Polynomial division refuses coefficient
numerators/denominators beyond its bit budget. Unknown fields, unknown versions,
missing/extra pair witnesses, source changes and false claims fail.

Not every valid region has a version-1 witness. Examples include externally
touching distinct components (the numerical editor can retain these), cases
with unavoidable extra eliminant roots, and cases exceeding subdivision or
arithmetic budgets. They return `BREP_PROFILE_CERTIFICATE_UNPROVEN`; this means
unproved, not invalid or intersection-free.

## Extended retained representations

General polynomial quadratics, noncircular positive-weight rational quadratics
and rational cubics now use the general profile kernel. Original poles/weights
are retained in normalization, Booleans and extrusion. Nonuniform affine
transformations of circles therefore retain rational elliptical arcs.

Numerical Green area for rational spans uses outward homogeneous jets through
order five and Simpson's fourth-derivative remainder. Failure to meet its
enclosure budget refuses orientation. The production exact root finder also
cancels common homogeneous factors by aligned fraction-free pseudo-quotients,
checking identities against every original coefficient before reduction.

Euclidean offset of a general conic/cubic is still outside the line/circle
offset carrier family and explicitly refuses. Periodic/multispan inputs need
extraction into retained spans; degrees above three are not silently fitted.

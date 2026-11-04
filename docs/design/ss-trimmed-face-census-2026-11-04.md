# Original SS events against finite BRep faces and holes

`brep_core::ss_trimmed_face::QualifiedSsTrimmedFaceCensus` binds native SS events
to an immutable authored BRep face snapshot. Input branch censuses must cover
EVERY nonempty tensor Cox span of that exact third surface, exactly once, and
must share the literal owned SS graph. Missing, duplicate, foreign or excessive
span/record coverage is refused, including empty native queries.

The authored outer loop and holes must pass continuous simplicity, separation,
containment and opposite-orientation audit. Every original world edge/pcurve
boundary lift is verified under the model's existing BRep agreement tolerance.
The point definition and trim region are not moved or enlarged by that tolerance.
Restricted/lifted coedges currently require separate authority and are refused.

For each native inside-span point, the exact original third UV interval is
converted to a VERIFIED outward binary rectangle. The WHOLE rectangle is then
classified against the original rational UV loops. Nonzero winding retains it;
zero winding excludes it from that trim region. Literal closed rectangles and
exact axis-aligned linear-boundary membership have their existing closed-set
proofs. Unknown/boundary-band queries without exact authority abort the complete
face query rather than choosing a midpoint or treating uncertainty as outside.

Each record keeps its source span/candidate index and therefore its literal
original SS point and native membership/exclusion owner. `ExcludedSpan` means
only exclusion from THAT span, not the whole face; all spans remain represented.
`ExcludedTrim` applies to a native member whose UV image is outside the authored
face region. Retained alias records across shared Cox seams stay explicit and
are not declared unique physical vertices. This is complete retained-event
coverage, not an ordered/welded BRep arrangement or Boolean material certificate.

The snapshot Arc and original face TopoId are retained. Replay regenerates all
native branch proofs, trim audits, boundary agreements and whole-box decisions.
Pair/trim/domain/boundary/point work and span/record caps are explicit, with point
and boundary cell ledgers shared across all spans. Existing native branch proof
construction/replay has its own policies outside this face-query ledger.

Qualification uses real planar BRep faces, a hole that excludes one of two actual
SS events, exact closed-boundary retention, original point/snapshot ownership,
and exact two-span refinement of the third surface. It rejects missing/duplicate
span proofs, broken hole joins, restricted coedges and deficient shared work or
record policies. Both UV and 3D boundary calls are exercised.

Remaining work includes exact general curved-boundary contacts, restricted and
periodic/pole coedges, algebraic UV extensions, cross-source junctions, canonical
alias/lineage handling, ordered arrangements/winding and actual face splitting.
Full material trim, intersecting Boolean/healing, history, bridge/WASM and the
wider quad/offset/fillet/G2 requirements remain open.

Follow-up: per-span records can now be canonically grouped and strictly ordered
by exact SS parameter, with ports preserving original selector ownership; see
`ss-ordered-face-events-2026-11-04.md`. Global physical junctions, ordered face
arrangements and actual splitting remain separate requirements.

# Remaining physical face publication

This is a design note, not implemented or qualified functionality. Catalog NURBS-0173 remains planned.

The current exact region owner proves connected open material components on the original embedded rectangle/cylinder/torus support. It does not prove that a selected boundary walk is a manifold face wire. A repeated vertex (including a hole tangent to a natural shore) must not silently become a regular manifold vertex.

## Existing representation constraints

`brep-core/src/lib.rs` aliases both Edge and Coedge to `brep_topology` containers parameterized by `nurbs_core::curve::Curve`. Model is concretely `brep_topology::Model<Curve, Surface, Curve>`. These curves contain floating point author controls. `CoedgeTrim` uses floating point native intervals and integer periodic lifts. It does not represent arbitrary rational coordinates or algebraic root selectors.

`trim_sew::CurvePcurveCorrespondence` proves correspondence of independently represented original rational definitions; it is not a generic composed surface-curve expression. Its admitted support cases are curved isoparametric U/V and affine planar. Do not treat it as authority for an arbitrary original UV curve composed with an arbitrary NURBS surface.

`profile_certificate/winding/periodic/face_build.rs` constructs the existing model from floating point UV wires and checks vertex links. Its source is a distinct, restricted family. Reusing its sewing logic does not by itself admit exact selected arcs with algebraic cuts.

## Required next representation

Retain immutable original Surface and original UV curve ownership, exact native cut selectors, traversal direction, and integer cover shifts. An edge expression on the source support must remain a composition, or must be converted to an exactly equivalent representation with a proof. Rounding generated rational controls to floating point is not such a conversion. Source-backed publication therefore needs an exact expression model or an explicitly bounded and checked conversion into the existing model.

Chart shores are auxiliary face cuts on wrapped axes, not material boundaries or winding operands. Construct chart-local pieces and sew only actual opposite-shore identities from the qualified original surface. Preserve separate chart incidences before quotienting vertices. The full untrimmed torus still requires face seams despite having zero material boundary walks.

For a pinched boundary, a candidate crosscut can attach at an original rational carrier center, follow the already certified side-probe leg to its strict inside probe, follow an ordinary source-qualified material path, and return through the other certified probe leg. The carrier attachment uses the existing complete transverse-side proof. This needs independent simplicity and periodic-copy intersection checks, correct split intervals, and incidence replay before publication. It does not yet establish a manifold face.

## Admission and verification

Every published edge use must replay its original curve/surface/cut binding. Check complete component ownership, orientation, auxiliary-cut pairing, physical seam identity, endpoint incidence, single connected vertex fans, and actual world-boundary agreement. Preserve a source descriptor for non-binary rational or algebraic endpoints. Test the actual seam-split annulus, closed torus, multiple essential cylinder bands, vertex-touching lobes, and the rational tangent hole without replacing their geometry. STEP and existing Model conversion are separate gates until exercised.

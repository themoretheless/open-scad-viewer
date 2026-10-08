# Quintic polygon stations and retained source-cell contacts

All construction, interval arithmetic, topology inspection and proof decisions
are Rust operations. JSON and TypeScript forward and validate receipts.

## Retained station construction

`smooth_polygon_station_walls` accepts matching clamped degree-one rational
polygon edges at each authored station. Original profile endpoints must agree
exactly before any correction. Coordinates are rounded to a caller-declared
normal binary power-of-two lattice; integer magnitude and multiplication
round-trips are checked. Rational profile weights are retained and must match
between stations. Closed first and last sections must match exactly.

Each wall uses degree-five station controls
`[p, p+t, p+2t, q-2t', q-t', q]`. The same lattice tangent is shared by both
incident station patches; their second derivative is exactly zero. Sharp
stations requested through the lower-level constructor retain explicit C0.
Every corrected wall is compared with the degree-elevated original ruled wall
using an outward control-displacement bound. This bounds correction of authored
walls; it is not a continuous error certificate for an ideal moving-frame sweep.
Construction or correction-budget exhaustion publishes no partial geometry.

The bridge operation `brep_nurbs_smooth_polygon_station_body` additionally
replays exact station jets against the actual B-rep coedge ownership and audits
the actual full shell. It publishes a model only if station G2 and retained
Solid geometry both pass. Polygon profile corners remain C0. Its
`continuousBound` is false unless the optional declared closed RMF source also
passes a whole-domain error certificate for every retained wall. This is a
separate premise from retained shell embedding; neither implies the other.

## Fixed positive metric for surface injectivity

Legacy charts preserve their original candidate ordering and shared cell
budget. Degree-one-by-five charts try a positive diagonal row metric before
the unit metric for each fixed projection. A midpoint jet proposes
the fixed scale but supplies no evidence. Each cell must prove that the symmetric
part of the weighted projected Jacobian is strictly positive definite with
outward arithmetic. The metric and projection stay fixed over the full chart.

For distinct chart points, integrate the Jacobian along their connecting line
in the convex parameter rectangle. Strict positive definiteness makes the
projected displacement have positive scalar product with the parameter
displacement, so the surface is injective. Continuous piecewise polynomial
charts with finite knot partitions have the same line-integral property.
Incomplete partitions and exhausted budgets never certify a chart.

For positive profile weights constant in the station direction, the original
rational wall has exactly the polynomial wall image under
`lambda(u) = w1*u / (w0*(1-u)+w1*u)`. Its derivative has a strictly positive
outward lower bound. The original surface remains unchanged. Ray isolation
runs on the polynomial image, then outward inverse mapping returns root boxes
in the original parameter. Original normal bounds include the positive
parameter derivative and the actual face-use orientation. Unresolved roots
remain unresolved; a bijection does not supply missing root isolation.

## Four-wall source volume

Two opposite retained walls and their two actual neighboring walls define a
trivariate Bernstein cell of degree `(1,1,p)` by their four corner-control rows.
Each original wall has degree one across the profile and weights constant along
its station direction. Positive, possibly unequal profile weights only induce
a bijective rational reparameterization across a wall; its entire image equals
the corresponding polynomial cell boundary. Original corner rows, source-face
ownership, natural full rectangular trims and common world edges are checked.
This inference is enabled only after joint exact boundary, trim and face
injectivity prerequisites.

A floating midpoint Jacobian proposes one fixed three-dimensional projection.
A complete adaptive dyadic cube partition encloses the original polynomial
Jacobian using outward de Casteljau subdivision. All three leading principal
minors of its symmetric projected Jacobian must have strictly positive lower
bounds in every accepted cell. Sylvester's criterion and the straight-line
integral prove strict monotonicity and injectivity of the entire cell.

Consequently, opposite source walls are disjoint and adjacent walls meet only
on their shared source edge. Extra shared topological edges or incompatible
corner rows refuse the proof. Every failed proposal cell and every successful
proof cell is charged to both the pair and global work budgets. These proofs
supplement the full shell pair matrix; they do not replace other station-pair,
cap, cavity, orientation or nesting checks.

The audit prepares source-edge ownership once for its immutable model. A
candidate ring still requires exactly two owners per longitudinal edge and
the actual four compatible faces; the cache supplies no positive geometry
claim. For degree-one-by-five walls the volume proof runs before the stitched
surface proof. Piecewise profile weights around polygon corners may not admit
one positive global surface reparameterization. Failed attempts still consume
the original per-pair and global budgets; neither budget is enlarged.

## Qualification boundary

Native regression cases include planar and spatial closed polygon bodies with
6, 7, 10 and 12 stations, exact station jets, unequal rational profile weights,
correction and work exhaustion, singular cells and folded cells. These finite
cases are not universal evidence for arbitrary guides, authored affine frames,
nonpolygon profiles or filled caps. Broader continuous reference-family bounds,
production integration and broader STEP families remain separate required work.
The finite native STEP matrix covers all 16 combinations of those counts,
planar/spatial stations and unit/unequal profile weights. An independent exact
rational divergence integral checks retained volume; OpenCascade checks the
imported geometry, derivatives, topology and orientation. Common positive
weight normalization is accepted only with exact rational proportionality.

The optional RMF reference certificate concatenates the actual retained quintic
wall patches and compares them with the original profile, path, scale and
transported frame over their complete domains. All profile walls share one
caller budget. Exhaustion or an incomplete wall proof refuses publication.
This selected closed polygon family excludes filled caps and alternative
guide or authored-frame reference laws. Production workflow integration and
WASM delivery qualification remain separate from the native STEP matrix.

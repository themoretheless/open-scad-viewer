# ADR: Manifold remains a peer engine (never a B-rep fallback)

Status: Accepted (F8 walking slice)  
Date: 2026-09-16

## Context

The product hosts two permanent geometry engines: Manifold (mesh/CSG, `legacy/current`)
and the Rust B-rep/NURBS kernel (`openscad-viewer/brep-1`). Prior risk: a failing
B-rep path silently re-routing into Manifold, which would launder mesh results as
analytic B-rep success.

## Decision

Manifold is a **peer** engine selected only by the language contract. It must never
be used as a fallback, heal, or cross-route from B-rep failures, quarantines, or
Unavailable/ResearchOnly capabilities.

## Consequences

- `GeometryBuildEngine` / `BrepBackendProvider` refuse Manifold cross-route.
- Mesh fillet/chamfer/shell helpers quarantine when a body carries retained B-rep.
- Capability registry rollback drills call `assertNoManifoldFallbackOnBrepFailure`.
- False-Complete freezes a capability to Unavailable; it does not unlock Manifold.

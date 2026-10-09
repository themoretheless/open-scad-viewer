// @rush/1
// Ordinary arithmetic notation for the cubic saddle, without sampled fitting.
show formula_surface(domain: [-1,1,-1,1], expressions: ["10*u","10*v","4*(u^3-3*u*v^2)"]).tessellate(segments_u: 16, segments_v: 32)

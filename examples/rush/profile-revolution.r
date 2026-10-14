// @rush/1
// Rational rotation of a planar Bezier profile, with open ends.
profile = bezier_curve(points: [[8mm,0mm,0mm],[14mm,0mm,6mm],[10mm,0mm,15mm]])
show profile.surface_revolve(origin: [0mm,0mm,0mm], axis: [0,0,1], angle: 270deg).tessellate(segments_u: 16, segments_v: 32)

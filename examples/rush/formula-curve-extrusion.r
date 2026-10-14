// @rush/1
// Reverse Polish x=8t, y=3t^2, z=t^3; normalized curve parameter maps to [-2,2].
// Formula constants/results are in mm coordinates, while t is dimensionless.
profile = formula_curve(domain: [-2,2], expressions: [[8,"t","*"],[3,"t","t","*","*"],["t","t","*","t","*"]])
show profile.surface_extrude([0mm,0mm,10mm]).tessellate(segments_u: 16, segments_v: 32)

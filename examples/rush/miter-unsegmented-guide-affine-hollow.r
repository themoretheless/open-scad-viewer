// @rush/1
// Original quadratic NURBS profile has four spans with low knot multiplicity.
outer = nurbs_curve(degree: 2, knots: [0,0,0,0.25,0.5,0.75,1,1,1],
 control_points: [[1mm,0mm,0mm],[1mm,1mm,0mm],[-1mm,1mm,0mm],[-1mm,-1mm,0mm],[1mm,-1mm,0mm],[1mm,0mm,0mm]], weights: [1,1,1,1,1,1])
hole = nurbs_curve(degree: 2, knots: [0,0,0,0.25,0.5,0.75,1,1,1],
 control_points: [[0.25mm,0mm,0mm],[0.25mm,-0.25mm,0mm],[-0.25mm,-0.25mm,0mm],[-0.25mm,0.25mm,0mm],[0.25mm,0.25mm,0mm],[0.25mm,0mm,0mm]], weights: [1,1,1,1,1,1])
rail = line_curve(start: [0mm,1mm,0mm], end: [0mm,1mm,10mm])
show brep_progressive_miter_sweep([[outer],[hole]], points: [[0,0,0],[0,0,10mm]], normal: [1,0,0],
 scale: {degree: 1, knots: [0,0,1,1], values: [1,1], weights: [1,1]},
 twist: {degree: 1, knots: [0,0,1,1], values: [0deg,0deg], weights: [1,1]},
 axis_scale: {degree: 1, knots: [0,0,1,1], values: [[2,1,1],[2,1,1]], weights: [1,1]},
 center_law: {degree: 1, knots: [0,0,1,1], values: [[0.125mm,0mm,0mm],[0.125mm,0mm,0mm]], weights: [1,1]},
 orientation_guide: rail,
 initial_steps: 1, max_steps: 16, max_deviation: 0.001mm
).brep_tessellate(4)

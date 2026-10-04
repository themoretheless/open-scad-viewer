// @rush/1
// Explicit periodic storage; exact active-domain blossom extraction certifies closure.
outer = nurbs_curve(degree: 2, knots: [0,1,2,3,4,5,6,7,8],
 control_points: [[1mm,0mm,0mm],[0mm,1mm,0mm],[-1mm,0mm,0mm],[0mm,-1mm,0mm],[1mm,0mm,0mm],[0mm,1mm,0mm]], weights: [1,1,1,1,1,1], periodic: true)
hole = nurbs_curve(degree: 2, knots: [0,1,2,3,4,5,6,7,8],
 control_points: [[0mm,0.25mm,0mm],[0.25mm,0mm,0mm],[0mm,-0.25mm,0mm],[-0.25mm,0mm,0mm],[0mm,0.25mm,0mm],[0.25mm,0mm,0mm]], weights: [1,1,1,1,1,1], periodic: true)
show brep_progressive_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[0,0,10mm]], normal: [1,0,0],
 scale: {degree: 1, knots: [0,0,1,1], values: [1,1], weights: [1,1]},
 twist: {degree: 1, knots: [0,0,1,1], values: [0deg,0deg], weights: [1,1]},
 initial_steps: 1, max_steps: 1, max_deviation: 0.001mm
).brep_tessellate(4)

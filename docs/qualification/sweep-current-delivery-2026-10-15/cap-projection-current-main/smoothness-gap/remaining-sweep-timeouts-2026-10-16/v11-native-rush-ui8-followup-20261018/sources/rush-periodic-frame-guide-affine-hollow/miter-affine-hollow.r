// @rush/1
// Periodic profile storage with independent frame, guide and affine law domains.
outer = nurbs_curve(degree: 2, knots: [0,1,2,3,4,5,6,7,8],
 control_points: [[1mm,0mm,0mm],[0mm,1mm,0mm],[-1mm,0mm,0mm],[0mm,-1mm,0mm],[1mm,0mm,0mm],[0mm,1mm,0mm]], weights: [1,1,1,1,1,1], periodic: true)
hole = nurbs_curve(degree: 2, knots: [0,1,2,3,4,5,6,7,8],
 control_points: [[0mm,0.25mm,0mm],[0.25mm,0mm,0mm],[0mm,-0.25mm,0mm],[-0.25mm,0mm,0mm],[0mm,0.25mm,0mm],[0.25mm,0mm,0mm]], weights: [1,1,1,1,1,1], periodic: true)
rail = nurbs_curve(degree: 1, knots: [31,31,41,41],
 control_points: [[1mm,0mm,0mm],[1mm,0mm,10mm]], weights: [1,1])
show brep_progressive_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[0,0,10mm]], normal: [1,0,0],
 scale: {degree: 1, knots: [0,0,1,1], values: [1,1], weights: [1,1]},
 twist: {degree: 1, knots: [0,0,1,1], values: [0deg,0deg], weights: [1,1]},
 frame_axis: {degree: 1, knots: [2,2,5,5], values: [[0,0,2],[0,0,2]], weights: [1,1]},
 frame_normal: {degree: 1, knots: [7,7,9,9], values: [[3,0,0],[3,0,0]], weights: [1,1]},
 orientation_guide: rail,
 axis_scale: {degree: 1, knots: [17,17,19,19], values: [[2,1,1],[2,1,1]], weights: [1,1]},
 center_law: {degree: 1, knots: [23,23,29,29], values: [[0.125mm,0.25mm,0mm],[0.125mm,0.25mm,0mm]], weights: [1,1]},
 initial_steps: 1, max_steps: 1, max_deviation: 0.001mm
).brep_tessellate(4)

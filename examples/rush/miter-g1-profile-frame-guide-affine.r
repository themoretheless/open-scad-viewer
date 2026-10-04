// @rush/1
// The quartic profile has exact G1 joins and discontinuous normal curvature (G2 is unproved).
a = bezier_curve(points: [[1mm,0,0],[1mm,1mm,0],[0.5mm,1mm,0],[1mm,1mm,0],[0,1mm,0]])
b = bezier_curve(points: [[0,1mm,0],[-1mm,1mm,0],[-1mm,0.5mm,0],[-1mm,1mm,0],[-1mm,0,0]])
c = bezier_curve(points: [[-1mm,0,0],[-1mm,-1mm,0],[-0.5mm,-1mm,0],[-1mm,-1mm,0],[0,-1mm,0]])
d = bezier_curve(points: [[0,-1mm,0],[1mm,-1mm,0],[1mm,-0.5mm,0],[1mm,-1mm,0],[1mm,0,0]])
rail = line_curve(start: [1mm,0,0],end: [1mm,0,10mm])
show brep_progressive_miter_sweep([[a,b,c,d]],points: [[0,0,0],[0,0,10mm]],normal: [1,0,0],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 frame_axis: {degree: 1,knots: [2,2,5,5],values: [[0,0,2],[0,0,2]],weights: [1,1]},
 frame_normal: {degree: 1,knots: [7,7,9,9],values: [[0,3,0],[0,3,0]],weights: [1,1]},
 orientation_guide: rail,
 axis_scale: {degree: 1,knots: [17,17,19,19],values: [[2,1,1],[2,1,1]],weights: [1,1]},
 initial_steps: 1,max_steps: 8,max_deviation: 0.01mm
).brep_tessellate(4)

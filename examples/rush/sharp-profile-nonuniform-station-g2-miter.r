// @rush/1
// Sharp rational profile corners remain C0; all unequal straight station joins are G2.
edge0 = nurbs_curve(degree: 2,knots: [0,0,0,1,1,1],control_points: [[-1mm,-1mm,0],[0mm,-1mm,0],[1mm,-1mm,0]],weights: [1,0.5,1])
edge1 = nurbs_curve(degree: 2,knots: [0,0,0,1,1,1],control_points: [[1mm,-1mm,0],[1mm,0mm,0],[1mm,1mm,0]],weights: [1,0.5,1])
edge2 = nurbs_curve(degree: 2,knots: [0,0,0,1,1,1],control_points: [[1mm,1mm,0],[0mm,1mm,0],[-1mm,1mm,0]],weights: [1,0.5,1])
edge3 = nurbs_curve(degree: 2,knots: [0,0,0,1,1,1],control_points: [[-1mm,1mm,0],[-1mm,0mm,0],[-1mm,-1mm,0]],weights: [1,0.5,1])
show brep_progressive_miter_sweep([[edge0,edge1,edge2,edge3]],
 points: [[0,0,0mm],[0,0,3mm],[0,0,9mm],[0,0,12mm],[0,0,18mm],[0,0,21mm],[0,0,27mm],[0,0,30mm],[0,0,36mm],[0,0,39mm],[0,0,45mm],[0,0,48mm],[0,0,54mm],[0,0,57mm],[0,0,63mm],[0,0,66mm],[0,0,72mm]],normal: [1,0,0],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_steps: 3,max_steps: 3,max_deviation: 0.01mm
).brep_tessellate(segments: 2)

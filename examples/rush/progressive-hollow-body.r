// @rush/1
outer = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 3mm)
hole = circle_curve(center: [0,0,0],normal: [0,0,-1],radius: 1mm)
path = bezier_curve(points: [[0,0,0],[0,0,10mm]])
show brep_progressive_sweep([[outer],[hole]],path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,5deg],weights: [1,1]},
 normal: [1,0,0],initial_sections: 3,max_sections: 257,max_deviation: 0.001mm
).brep_tessellate(8)

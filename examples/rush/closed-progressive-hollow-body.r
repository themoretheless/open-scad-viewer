// @rush/1
outer = circle_curve(center: [5mm,0,0],normal: [0,1,0],radius: 0.5mm)
hole = circle_curve(center: [5mm,0,0],normal: [0,-1,0],radius: 0.2mm)
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 5mm)
show brep_progressive_sweep([[outer],[hole]],path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [0,0,1],initial_sections: 5,max_sections: 129,max_deviation: 0.02mm
).brep_tessellate(4)

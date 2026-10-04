// @rush/1
outer = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 0.5mm)
hole = circle_curve(center: [0,0,0],normal: [0,0,-1],radius: 0.2mm)
show brep_progressive_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[0,0,10mm]],normal: [1,0,0],closed: false,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1.5],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,30deg],weights: [1,1]},
 initial_steps: 1,max_steps: 16,max_deviation: 0.02mm
).brep_tessellate(2)

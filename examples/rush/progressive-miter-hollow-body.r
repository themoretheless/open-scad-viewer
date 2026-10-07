// @rush/1
outer = circle_curve(center: [0,0,0],normal: [1,0,0],radius: 0.1mm)
hole = circle_curve(center: [0,0,0],normal: [-1,0,0],radius: 0.04mm)
show brep_progressive_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[10mm,0,0],[10mm,10mm,4mm],[0,10mm,1mm],[0,5mm,-2mm]],
 normal: [0,0,1],miter_limit: 4,closed: true,
 scale: {degree: 2,knots: [0,0,0,1,1,1],values: [1,1.5,1],weights: [1,1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,360deg],weights: [1,1]},
 initial_steps: 1,max_steps: 64,max_deviation: 0.01mm
).brep_tessellate(2)

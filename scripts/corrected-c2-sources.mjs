// Authored QA inputs. Curve/surface construction and every proof run in Rust.
export const correctedC2Path={degree:3,knots:[0,0,0,0,.5,1,1,1,1],
 controlPoints:[[0,0,0],[.16666666666666666,0,0],[.5,.16666666666666666,0],
  [.8333333333333334,.6666666666666666,.5],[1,1,1.125]],
 weights:[1,1,1,1,1],periodic:false}
export const correctedC2BodySources=['corrected-c2-parameter-body-boundary.r','corrected-c2-arc-body-boundary.r',
 'corrected-c2-parameter-caps-body-boundary.r','corrected-c2-arc-caps-body-boundary.r']
export function correctedC2BodySource(spacing,correctCaps=false){
 if(!['parameter','arc_length'].includes(spacing))throw Error('Unknown corrected-C2 spacing')
 const poles=correctedC2Path.controlPoints.map(p=>'['+p.map(v=>`${v}mm`).join(',')+']').join(',')
 return `// @rush/1
// A regular cubic C2 source with a real interior knot; frame second jets remain separate.
outer = circle_curve(center: [0,0,0],normal: [1,0,0],radius: 0.1mm)
hole = circle_curve(center: [0,0,0],normal: [-1,0,0],radius: 0.05mm)
path = nurbs_curve(degree: 3,knots: [0,0,0,0,0.5,1,1,1,1],control_points: [${poles}],weights: [1,1,1,1,1])
show brep_progressive_sweep([[outer],[hole]],path,
 orientation: "corrected_frenet",normal: [0,0,1],spacing: "${spacing}",${spacing==='arc_length'?'length_tolerance: 0.001mm,length_max_cells: 100000,':''}
 circle_correction_tolerance: 0.000000001mm,circle_correction_quantum: 0.0000000000009094947017729282mm,circle_correction_max_work: 1000000,
${correctCaps?' cap_correction_tolerance: 0.000000001mm,cap_correction_quantum: 0.0000000000009094947017729282mm,cap_correction_max_work: 1000000,\n':''} scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 3,max_sections: 129,max_deviation: 0.05mm
).brep_tessellate(4)
`
}

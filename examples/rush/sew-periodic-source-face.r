// @rush/1
rim = polyline_curve(points:[[0mm,0mm,0mm],[1mm,0mm,0mm],[1mm,1mm,0mm],[0mm,1mm,0mm],[0mm,0mm,0mm]],closed:false)
carrier = rim.surface_extrude([0mm,0mm,1mm])
crossing = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0.125,0.25],[-0.375,0.5],[0.125,0.75]],weights:[1,1,1])
return_edge = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0.125,0.75],[0.125,0.25]],weights:[1,1])
sewn = carrier.sew_periodic_source_face(loops:[[crossing,return_edge]],wrapped_axes:"u",base_uv:[2,0.5],base_winding:0,probe_radius_uv:0.0009765625,max_work:100000,max_edges:128)
show sewn.tessellate(16,4)

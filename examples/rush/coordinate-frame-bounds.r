// @rush/1
profile = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[8mm,4mm,0mm],[12mm,4mm,0mm]],weights:[1,2])
local = profile.coordinate_frame_local(origin:[10mm,4mm,0mm],frame_scale:0.5)
check_local = local.geometry_conditioning()
restored = check_local.coordinate_frame_world(origin:[10mm,4mm,0mm],frame_scale:0.5)
sheet = restored.surface_extrude(vector:[0mm,2mm,0mm])
bounds = sheet.geometry_bounds_union(inputs:[local,profile])
show bounds.tessellate(8,8)

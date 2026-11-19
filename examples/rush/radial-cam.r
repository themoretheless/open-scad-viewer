// @rush/1
cam = radial_cam_walls(center:[2mm,-3mm,5mm],base_radius:4mm,roller_radius:0.75mm,phase:0,end_angles:[180,360],end_lifts:[2mm,0mm],laws:["harmonic","cycloidal"],tolerance_mm:0.00001mm,max_spans:1024,vector:[0,0,2mm])
show cam.nurbs_patches_tessellate(16)

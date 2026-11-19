// @rush/1
profile = nurbs_curve(degree:1,knots:[-1,0,1,2,3,4,5],control_points:[[0,0,0],[1mm,0,0],[1mm,1mm,0],[0,1mm,0],[0,0,0]],weights:[1,1,1,1,1],periodic:true)
chart = profile.surface_extrude(vector:[0,0,2mm])
length = chart.surface_trim_range_length(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],max_work:1000000,contour:0,segment:0,range_start:"4/5",range_end:"1/3",tolerance_mm:0.00000001mm,max_cells:100)
show length.tessellate(8,4)

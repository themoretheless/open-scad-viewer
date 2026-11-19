// @rush/1
profile = nurbs_curve(degree:1,knots:[-1,0,1,2,3,4,5],control_points:[[0,0,0],[1mm,0,0],[1mm,1mm,0],[0,1mm,0],[0,0,0]],weights:[1,1,1,1,1],periodic:true)
chart = profile.surface_extrude(vector:[0,0,2mm])
query1 = chart.locate_input_parameter(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],contour:0,segment:0,exact_parameter:"1/2",tolerance_mm:0.00000001mm,max_work:1000000)
query2 = query1.locate_input_range(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],contour:0,segment:0,range_start:"3/4",range_end:"1/4",tolerance_mm:0.00000001mm,max_work:1000000)
query3 = query2.extract_input_range_curves(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],contour:0,segment:0,range_start:"3/4",range_end:"1/4",tolerance_mm:0.00000001mm,max_work:1000000)
query4 = query3.extract_input_range_curve_pairs(contours:[[[3.5,0.125],[0.5,0.125],[0.5,0.875],[3.5,0.875],[3.5,0.125]],[[0.125,0.25],[0.375,0.25],[0.375,0.75],[0.125,0.75],[0.125,0.25]]],contour:0,segment:0,range_start:"3/4",range_end:"1/4",tolerance_mm:0.00000001mm,max_work:1000000)
show query4.tessellate(8,4)

export function curveMeasurementLabel(operation: string, russian: boolean): string {
  if (['lift_exact_periodic_surface_region_components', 'build_exact_periodic_surface_trim_components', 'build_exact_periodic_surface_trim_patches', 'build_exact_periodic_surface_trim_edges'].includes(operation)) return russian ? 'Компоненты обрезки' : 'Trim components'
  if (operation.includes('periodic_surface_region') || operation.includes('periodic_surface_contour_forest') || operation.includes('periodic_surface_paths')) return russian ? 'Область на периодической поверхности' : 'Periodic chart region'
  if (operation.includes('periodic_surface_path') || operation.includes('wrapped_surface_point')) return russian ? 'Измерение в периодических координатах' : 'Periodic chart measurement'
  if (operation.includes('input_contour') || operation.includes('input_segment') || operation.includes('input_patch_max')) {
    if (operation.includes('project')) return russian ? 'Проекция контура' : 'Contour projection'
    return russian ? 'Длина и положение вдоль контура' : 'Distance and position along contour'
  }
  if (['locate_input_parameter', 'locate_edge_parameter', 'locate_input_range', 'extract_input_range_curves', 'extract_input_range_curve_pairs'].includes(operation)) return russian ? 'Привязка ребра' : 'Edge binding'
  if (['evaluate_input_point','evaluate_input_tangents','classify_input_tangent_join','classify_input_vertex_join'].includes(operation)) return russian ? 'Геометрия ребра' : 'Edge geometry'
  if (operation === 'naca_five_digit_profile') return russian ? 'Пятизначный профиль NACA' : 'Five-digit NACA section'
  if (operation === 'cst_spline_profile') return russian ? 'Сплайн по профилю CST' : 'CST spline profile'
  if (operation === 'fit_cst_sides') return russian ? 'Подбор профиля CST по двум сторонам' : 'CST fit to upper and lower samples'
  if (operation === 'join_affine_c1') return russian ? 'Аффинная склейка C1' : 'Affine C1 join'
  if (operation === 'join_affine_c2') return russian ? 'Аффинная склейка C2' : 'Affine C2 join'
  if (operation === 'join_projective_c2') return russian ? 'Дробно-линейная склейка C2' : 'Projective C2 join'
  if (operation === 'join_stationary_c2') return russian ? 'Склейка C2 при нулевой касательной' : 'Stationary C2 join'
  if (operation === 'store_periodic_curve') return russian ? 'Периодическая запись кривой C2' : 'Store periodic C2 curve'
  if (operation === 'rotate_periodic_knot') return russian ? 'Перенос шва на узел' : 'Rotate seam to knot'
  if (operation === 'insert_periodic_seam') return russian ? 'Вставка и перенос шва' : 'Insert and rotate seam'
  if (operation === 'open_periodic_curve') return russian ? 'Раскрытие периода' : 'Open periodic curve'
  if (operation === 'trim_periodic_curve') return russian ? 'Участок через шов' : 'Trim across seam'
  if (operation === 'trim_periodic_turns') return russian ? 'Участок через несколько периодов' : 'Trim across periods'
  if (operation === 'partition_periodic_curve') return russian ? 'Разбиение периода' : 'Partition period'
  if (operation === 'join_mixed_degree') return russian ? 'Склейка кривых разной степени' : 'Mixed-degree curve join'
  if (operation === 'compact_join_seam') return russian ? 'Удаление лишних узлов стыка' : 'Compact curve seam'
  if (operation === 'compare_parametric_seam') return russian ? 'Параметрическая гладкость стыка' : 'Parametric seam continuity'
  if (operation === 'compare_geometric_seam') return russian ? 'Геометрическая гладкость стыка' : 'Geometric seam continuity'
  if (operation === 'merge_weighted_curves') return russian ? 'Склейка рациональных кривых' : 'Weighted curve merge'
  if (operation === 'join_projective_chain') return russian ? 'Склейка цепочки C2' : 'Projective C2 chain join'
  if (operation === 'compact_projective_chain') return russian ? 'Удаление лишних узлов цепочки C2' : 'Compact C2 chain'
  if (operation === 'close_projective_chain') return russian ? 'Замыкание цепочки C2' : 'Close C2 chain'
  if (operation === 'merge_weighted_chain') return russian ? 'Склейка цепочки рациональных кривых' : 'Weighted curve chain merge'
  if (operation === 'merge_curve_exact') return russian ? 'Точное объединение кривых' : 'Exact curve merge'
  if (operation === 'merge_surface_exact') return russian ? 'Точное объединение поверхностей' : 'Exact surface merge'
  if (operation === 'bounded_surface_partial_field') return russian ? 'С допуском · Поле частной производной' : 'Bounded Partial derivative field'
  if (operation === 'surface_partial_field') return russian ? 'Поле частной производной' : 'Partial derivative field'
  if (operation === 'bounded_surface_area_normal_field') return russian ? 'С допуском · Поле нормалей площади' : 'Bounded Area normal field'
  if (operation === 'surface_area_normal_field') return russian ? 'Поле нормалей площади' : 'Area normal field'
  if (operation === 'bounded_surface_gaussian_field') return russian ? 'С допуском · Поле гауссовой кривизны' : 'Bounded Gaussian curvature field'
  if (operation === 'bounded_surface_mean_squared_field') return russian ? 'С допуском · Поле квадрата средней кривизны' : 'Bounded Squared mean curvature field'
  if (operation === 'bounded_surface_principal_gap_squared_field') return russian ? 'С допуском · Поле квадрата разности главных кривизн' : 'Bounded Squared principal curvature gap field'
  if (operation === 'bounded_surface_metric_field') return russian ? 'С допуском · Первая фундаментальная форма' : 'Bounded First fundamental form'
  if (operation === 'surface_gaussian_field') return russian ? 'Поле гауссовой кривизны' : 'Gaussian curvature field'
  if (operation === 'surface_mean_squared_field') return russian ? 'Поле квадрата средней кривизны' : 'Squared mean curvature field'
  if (operation === 'surface_principal_gap_squared_field') return russian ? 'Поле квадрата разности главных кривизн' : 'Squared principal curvature gap field'
  if (operation === 'surface_metric_field') return russian ? 'Первая фундаментальная форма' : 'First fundamental form'
  if (operation === 'bounded_surface_mixed_partial_field') return russian ? 'С допуском · Поле смешанной производной' : 'Bounded Mixed partial derivative field'
  if (operation === 'surface_mixed_partial_field') return russian ? 'Поле смешанной производной' : 'Mixed partial derivative field'
  if (operation === 'bounded_curve_derivative_field') return russian ? 'С допуском · Поле производной кривой' : 'Bounded Curve derivative field'
  if (operation === 'curve_derivative_field') return russian ? 'Поле производной кривой' : 'Curve derivative field'
  if (operation === 'bounded_curve_torsion_field') return russian ? 'С допуском · Поле кручения' : 'Bounded Torsion field'
  if (operation === 'bounded_curve_speed_squared_field') return russian ? 'С допуском · Поле квадрата скорости' : 'Bounded Squared speed field'
  if (operation === 'curve_torsion_field') return russian ? 'Поле кручения' : 'Torsion field'
  if (operation === 'curve_speed_squared_field') return russian ? 'Поле квадрата скорости' : 'Squared speed field'
  if (operation === 'bounded_curve_binormal_field') return russian ? 'С допуском · Поле ненормированной бинормали' : 'Bounded Unnormalized binormal field'
  if (operation === 'curve_binormal_field') return russian ? 'Поле ненормированной бинормали' : 'Unnormalized binormal field'
  if (operation === 'bounded_curve_curvature_squared_field') return russian ? 'С допуском · Поле квадрата кривизны' : 'Bounded Squared curvature field'
  if (operation === 'curve_curvature_squared_field') return russian ? 'Поле квадрата кривизны' : 'Squared curvature field'
  if (operation === 'projective_curve_exact') return russian ? 'Точное проективное преобразование кривой' : 'Exact projective curve transform'
  if (operation === 'projective_surface_exact') return russian ? 'Точное проективное преобразование поверхности' : 'Exact projective surface transform'
  if (operation === 'surface_cell_interpolation_error') return russian ? 'Ошибка интерполяции UV-ячейки' : 'UV cell interpolation error'
  if (operation === 'surface_triangle_interpolation_error') return russian ? 'Отклонение сохранённого треугольника' : 'Stored triangle deviation'
  if (operation === 'surface_grid_interpolation_error') return russian ? 'Отклонение сохранённой сетки' : 'Stored grid deviation'
  if (operation === 'surface_umbilic_cells') return russian ? 'Умбилические ячейки' : 'Umbilic cells'
  if (operation === 'surface_umbilic_regions') return russian ? 'Уточнённые умбилические области' : 'Refined umbilic regions'
  if (operation === 'surface_gap_comparison') return russian ? 'Сравнение разности главных кривизн' : 'Squared curvature gap comparison'
  if (operation === 'surface_gap_ranges') return russian ? 'Диапазоны квадрата разности кривизн' : 'Squared curvature gap ranges'
  if (operation === 'surface_gap_regions') return russian ? 'Пороговые области главных кривизн' : 'Refined curvature gap regions'
  if (operation === 'surface_gap_edge_roots') return russian ? 'Корни порога кривизны на ребре' : 'Curvature threshold edge roots'
  if (operation === 'surface_gap_contour_graph') return russian ? 'График контура кривизны' : 'Curvature contour graph'
  if (operation === 'surface_gap_contour_samples') return russian ? 'Интервалы точек контура кривизны' : 'Curvature contour sample brackets'
  if (operation === 'surface_gap_world_polyline') return russian ? 'Пространственная ломаная контура' : 'World contour polyline'
  if (operation === 'surface_gap_world_length') return russian ? 'Длина пространственного контура' : 'World contour length'
  if (operation === 'split_tensor_surface') return russian ? 'Точное деление поверхности' : 'Exact surface split'
  if (operation === 'tile_tensor_surface') return russian ? 'Точное разбиение поверхности на патчи' : 'Exact surface tiling'
  if (operation === 'mobius_bezier_curve') return russian ? 'Параметризация Мёбиуса кривой' : 'mobius bezier curve'
  if (operation === 'mobius_spline_curve') return russian ? 'Параметризация Мёбиуса кривой' : 'mobius spline curve'
  if (operation === 'fit_mobius_curve') return russian ? 'Подбор параметризации Мёбиуса кривой' : 'fit mobius curve'
  if (operation === 'mobius_bezier_surface') return russian ? 'Параметризация Мёбиуса поверхности' : 'mobius bezier surface'
  if (operation === 'mobius_spline_surface') return russian ? 'Параметризация Мёбиуса поверхности' : 'mobius spline surface'
  if (operation === 'fit_mobius_surface') return russian ? 'Подбор параметризации Мёбиуса поверхности' : 'fit mobius surface'
  if (operation === 'affine_clock_curve') return russian ? 'Смена диапазона параметра кривой' : 'Affine curve parameter range'
  if (operation === 'affine_clock_surface') return russian ? 'Смена диапазонов U и V' : 'Affine surface parameter ranges'
  if (operation === 'piecewise_clock_curve') return russian ? 'Параметризация пролётов кривой' : 'piecewise clock curve'
  if (operation === 'relayout_clock_curve') return russian ? 'Перенос границ пролётов кривой' : 'relayout clock curve'
  if (operation === 'piecewise_clock_surface') return russian ? 'Параметризация пролётов поверхности' : 'piecewise clock surface'
  if (operation === 'relayout_clock_surface') return russian ? 'Перенос границ пролётов поверхности' : 'relayout clock surface'
  if (operation === 'pullback_uv_clock') return russian ? 'Перенос UV-контура в новую параметризацию' : 'Transfer UV contour to new parameters'
  if (operation === 'affine_trim_surface') return russian ? 'Смена диапазонов поверхности с обрезкой' : 'affine trim surface'
  if (operation === 'transpose_trim_surface') return russian ? 'Перестановка осей поверхности с обрезкой' : 'transpose trim surface'
  if (operation === 'remap_trim_surface') return russian ? 'Порядок и диапазоны осей поверхности с обрезкой' : 'remap trim surface'
  if (operation === 'reclock_trim_surface') return russian ? 'Перепараметризация поверхности с контурами' : 'Reparameterize trimmed surface'
  if (operation === 'affine_trim_piece') return russian ? 'Смена диапазонов границы обрезки' : 'affine trim piece'
  if (operation === 'transpose_trim_piece') return russian ? 'Перестановка осей границы обрезки' : 'transpose trim piece'
  if (operation === 'remap_trim_piece') return russian ? 'Порядок и диапазоны осей границы обрезки' : 'remap trim piece'
  if (operation === 'reclock_trim_piece') return russian ? 'Перенесённый участок границы' : 'Transferred trim boundary piece'
  if (operation === 'certify_uv_chart') return russian ? 'Проверка границ UV-контура' : 'Check UV contour containment'
  if (operation === 'classify_uv_seam') return russian ? 'Характер контакта UV-кривой со швом' : 'Classify UV seam contact'
  if (operation === 'isolate_uv_line') return russian ? 'Контакты UV-кривой с прямой' : 'UV curve contacts with a line'
  if (operation === 'isolate_uv_seam') return russian ? 'Локализация контакта UV-кривой со швом' : 'Isolate UV seam contact'
  if (operation === 'split_uv_seam') return russian ? 'Разделение UV-кривой по шву' : 'Split UV curve at seam'
  if (operation === 'lift_uv_edge') return russian ? 'Пространственное ребро по UV-кривой' : 'Lift UV curve onto surface'
  if (operation === 'transfer_uv_grid_piece') return russian ? 'Перенос UV-кривой на новую сетку' : 'Transfer UV curve across span grids'
  if (operation === 'relayout_trim_surface') return russian ? 'Смена UV-сетки поверхности с контурами' : 'Relayout trimmed surface'
  if (operation === 'relayout_trim_piece') return russian ? 'Участок контура на новой UV-сетке' : 'Relayout trim boundary piece'
  if (operation === 'verify_uv_edge') return russian ? 'Сверка ребра с UV-контуром' : 'Verify edge against surface trace'
  if (operation === 'compare_seam_orientations') return russian ? 'Направления общего шва' : 'Seam orientation comparison'
  if (operation === 'compare_uv_traces') return russian ? 'Сверка пространственных следов UV-шва' : 'Compare surface traces at seam'
  if (operation === 'compare_uv_tangents') return russian ? 'Сверка касательных плоскостей UV-шва' : 'Compare seam tangent planes'
  if (operation === 'check_seam_gap') return russian ? 'Допуск зазора UV-шва' : 'Seam gap tolerance'
  if (operation === 'check_seam_angle') return russian ? 'Допуск угла UV-шва' : 'Seam angle tolerance'
  if (operation === 'isolate_seam_gap') return russian ? 'Участки превышения зазора' : 'Gap violation intervals'
  if (operation === 'isolate_seam_angle') return russian ? 'Участки превышения угла' : 'Angle violation intervals'
  if (operation === 'exact_wire_area') return russian ? 'Точная площадь контура' : 'exact wire area'
  if (operation === 'bound_wire_moments') return russian ? 'Интервалы моментов рационального контура' : 'Rational wire moment bounds'
  if (operation === 'cap_section_wire') return russian ? 'Крышка по контуру' : 'cap section wire'
  if (operation === 'attach_expanded_rims') return russian ? 'Сшить выбранные контуры с точной опорой' : 'attach expanded rims'
  if (operation === 'attach_section_rims') return russian ? 'Пришить крышку по рёбрам' : 'attach section rims'
  if (operation === 'discover_expanded_rims') return russian ? 'Найти и сшить контуры с точной опорой' : 'discover expanded rims'
  if (operation === 'discover_section_rims') return russian ? 'Найти и закрыть свободный контур' : 'discover section rims'
  if (operation === 'attach_bounded_rims') return russian ? 'Пришить крышку с допуском' : 'attach bounded rims'
  if (operation === 'build_expanded_plane_solid') return russian ? 'Построить тело по точным расширенным плоскостям' : 'build expanded plane solid'
  if (operation === 'build_outward_plane_solid') return russian ? 'Исправить ориентацию тела по плоскостям' : 'build outward plane solid'
  if (operation === 'build_section_solid') return russian ? 'Построить тело и проверить объём' : 'build section solid'
  if (operation === 'infer_rim_planes') return russian ? 'Определить плоскости открытых контуров' : 'infer rim planes'
  if (operation === 'close_inferred_rims') return russian ? 'Закрыть открытые плоские контуры' : 'close inferred rims'
  if (operation === 'close_bounded_inferred_rims') return russian ? 'Закрыть открытые контуры с допуском' : 'close bounded inferred rims'
  if (operation === 'inspect_inferred_solid') return russian ? 'Проверить границу и ориентацию тела' : 'inspect inferred solid'
  if (operation === 'build_outward_inferred_solid') return russian ? 'Определить плоскости и исправить ориентацию тела' : 'build outward inferred solid'
  if (operation === 'build_expanded_solid') return russian ? 'Построить тело с точными расширенными опорами' : 'build expanded solid'
  if (operation === 'build_inferred_solid') return russian ? 'Построить тело по открытым контурам' : 'build inferred solid'
  if (operation === 'close_expanded_planes') return russian ? 'Закрыть плоскости с расширенными опорами' : 'close expanded planes'
  if (operation === 'project_polyhedral_body') return russian ? 'Точная проекция многогранника' : 'project polyhedral body'
  if (operation === 'measure_polyhedral_frame') return russian ? 'Границы тела в заданном базисе' : 'measure polyhedral frame'
  if (operation === 'measure_polyhedral_axis_gap') return russian ? 'Зазор тел вдоль оси' : 'measure polyhedral axis gap'
  if (operation === 'section_polyhedral_faces') return russian ? 'Сечения граней многогранника' : 'section polyhedral faces'
  if (operation === 'assemble_polyhedral_section_loops') return russian ? 'Замкнутые контуры сечения' : 'assemble polyhedral section loops'
  if (operation === 'measure_polyhedral_section_area') return russian ? 'Точная площадь сечения' : 'measure polyhedral section area'
  if (operation === 'measure_polyhedral_section_centroid') return russian ? 'Точный центр площади сечения' : 'measure polyhedral section centroid'
  if (operation === 'bound_polyhedral_section_perimeter') return russian ? 'Границы периметра сечения' : 'bound polyhedral section perimeter'
  if (operation === 'clip_polyhedral_segment') return russian ? 'Отсечь отрезок по телу' : 'clip polyhedral segment'
  if (operation === 'locate_polyhedral_point') return russian ? 'Расположение точки в теле' : 'locate polyhedral point'
  if (operation === 'measure_polyhedral_boundary_distance') return russian ? 'Расстояние до границы тела' : 'measure polyhedral boundary distance'
  if (operation === 'measure_polyhedral_signed_distance') return russian ? 'Знаковое расстояние до тела' : 'measure polyhedral signed distance'
  if (operation === 'sample_polyhedral_signed_distances') return russian ? 'Знаковые расстояния для точек' : 'sample polyhedral signed distances'
  if (operation === 'measure_swept_ball_gap') return russian ? 'Зазор сферы на отрезке' : 'measure swept ball gap'
  if (operation === 'bound_path_material_length') return russian ? 'Длина пути в материале' : 'bound path material length'
  if (operation === 'clip_exterior_path') return russian ? 'Внешние участки пути' : 'clip exterior path'
  if (operation === 'clip_interior_path') return russian ? 'Внутренние участки пути' : 'clip interior path'
  if (operation === 'partition_solid_path') return russian ? 'Разбиение пути по положению' : 'partition solid path'
  if (operation === 'bound_path_length_breakdown') return russian ? 'Длины внутренних, граничных и внешних участков' : 'bound path length breakdown'
  if (operation === 'find_path_face_contacts') return russian ? 'Контакты пути с гранями' : 'find path face contacts'
  if (operation === 'clip_common_path') return russian ? 'Общий материал вдоль пути' : 'clip common path'
  if (operation === 'clip_union_path') return russian ? 'Объединение материалов вдоль пути' : 'clip union path'
  if (operation === 'subtract_path_cutters') return russian ? 'Путь после вычитания тел' : 'subtract path cutters'
  if (operation === 'classify_path_membership') return russian ? 'Принадлежность участков пути телам' : 'classify path membership'
  if (operation === 'bound_overlap_lengths') return russian ? 'Длина пути по числу тел' : 'bound overlap lengths'
  if (operation === 'recover_overlap_paths') return russian ? 'Участки пути по числу тел' : 'recover overlap paths'
  if (operation === 'triangulate_planar_path') return russian ? 'Триангуляция плоского контура' : 'triangulate planar path'
  if (operation === 'measure_path_second_moments') return russian ? 'Центральные моменты контура' : 'measure path second moments'
  if (operation === 'bound_path_principal_moments') return russian ? 'Главные моменты контура' : 'bound path principal moments'
  if (operation === 'measure_region_mass') return russian ? 'Площадь и центр профиля' : 'measure region mass'
  if (operation === 'intersect_transverse_profiles') return russian ? 'Пересечение поперечных профилей' : 'intersect transverse profiles'
  if (operation === 'measure_profile_overlap_area') return russian ? 'Площадь перекрытия профилей' : 'measure profile overlap area'
  if (operation === 'measure_profile_overlap_centroid') return russian ? 'Центр перекрытия профилей' : 'measure profile overlap centroid'
  if (operation === 'classify_profile_overlap') return russian ? 'Взаимное положение профилей' : 'classify profile overlap'
  if (operation === 'decompose_profile_boolean') return russian ? 'Ячейки булевой операции' : 'decompose profile boolean'
  if (operation === 'extract_profile_boolean_edges') return russian ? 'Рёбра булевой области' : 'extract profile boolean edges'
  if (operation === 'assemble_profile_boolean_loops') return russian ? 'Контуры булевой области' : 'assemble profile boolean loops'
  if (operation === 'triangulate_profile_boolean') return russian ? 'Триангуляция булевой области' : 'triangulate profile boolean'
  if (operation === 'measure_parallel_sweep_overlap') return russian ? 'Объём пересечения выдавливаний' : 'measure parallel sweep overlap'
  if (operation === 'mesh_parallel_sweep_overlap') return russian ? 'Оболочка пересечения выдавливаний' : 'mesh parallel sweep overlap'
  if (operation === 'build_parallel_sweep_overlap') return russian ? 'Тело пересечения выдавливаний' : 'build parallel sweep overlap'
  if (operation === 'build_profile_boolean_solid') return russian ? 'Проверенное тело выдавливания' : 'build profile boolean solid'
  if (operation === 'measure_profile_extrusion_inertia') return russian ? 'Инерция выдавленного профиля' : 'measure profile extrusion inertia'
  if (operation === 'locate_profile_extrusion_points') return russian ? 'Точки внутри выдавленного профиля' : 'locate profile extrusion points'
  if (operation === 'section_profile_extrusion') return russian ? 'Поперечное сечение выдавливания' : 'section profile extrusion'
  if (operation === 'section_profile_extrusion_parallel') return russian ? 'Продольное сечение выдавливания' : 'section profile extrusion parallel'
  if (operation === 'split_profile_extrusion_volume') return russian ? 'Разбиение объёма плоскостью' : 'split profile extrusion volume'
  if (operation === 'clip_profile_extrusion_path') return russian ? 'Отсечение пути выдавленным телом' : 'clip profile extrusion path'
  if (operation === 'bound_profile_extrusion_area') return russian ? 'Площадь поверхности выдавливания' : 'bound profile extrusion area'
  if (operation === 'extrude_profile_boolean_shell') return russian ? 'Выдавливание булевой области' : 'extrude profile boolean shell'
  if (operation === 'decompose_profile_overlap') return russian ? 'Ячейки перекрытия профилей' : 'decompose profile overlap'
  if (operation === 'clip_path_to_perforated_region') return russian ? 'Отсечение пути профилем' : 'clip path to perforated region'
  if (operation === 'measure_region_second_moments') return russian ? 'Центральные моменты профиля' : 'measure region second moments'
  if (operation === 'measure_planar_path_area') return russian ? 'Ориентированная площадь' : 'measure planar path area'
  if (operation === 'measure_planar_path_centroid') return russian ? 'Центр площади контура' : 'measure planar path centroid'
  if (operation === 'simplify_exact_path') return russian ? 'Точное упрощение ломаной' : 'simplify exact path'
  if (operation === 'project_to_planar_region') return russian ? 'Проекция на заполненный контур' : 'project to planar region'
  if (operation === 'project_point_to_path') return russian ? 'Проекция точки на ломаную' : 'project point to path'
  if (operation === 'locate_planar_path_points') return russian ? 'Положение точек относительно контура' : 'locate planar path points'
  if (operation === 'clip_path_to_planar_region') return russian ? 'Участки пути в плоской области' : 'clip path to planar region'
  if (operation === 'intersect_spatial_paths') return russian ? 'Пересечения ломаных' : 'intersect spatial paths'
  if (operation === 'node_spatial_path') return russian ? 'Узлы самопересечения' : 'node spatial path'
  if (operation === 'find_path_self_intersections') return russian ? 'Самопересечения ломаной' : 'find path self intersections'
  if (operation === 'measure_spatial_path_distance') return russian ? 'Расстояние между ломаными' : 'measure spatial path distance'
  if (operation === 'simplify_path_with_tolerance') return russian ? 'Упрощение ломаной с допуском' : 'simplify path with tolerance'
  if (operation === 'recover_material_path') return russian ? 'Восстановленные участки пути' : 'Recovered material path'
  if (operation === 'clip_path_to_material') return russian ? 'Участки пути в материале' : 'clip path to material'
  if (operation === 'find_path_boundary_events') return russian ? 'Переходы пути через границу' : 'find path boundary events'
  if (operation === 'measure_swept_path_gap') return russian ? 'Минимальный зазор вдоль пути' : 'measure swept path gap'
  if (operation === 'classify_swept_path_material') return russian ? 'Проход сферы относительно материала' : 'classify swept path material'
  if (operation === 'bound_swept_first_contact') return russian ? 'Начало контакта вдоль пути' : 'bound swept first contact'
  if (operation === 'bound_swept_face_ranges') return russian ? 'Интервалы контакта с гранями' : 'bound swept face ranges'
  if (operation === 'measure_polyhedral_ball_clearance') return russian ? 'Сферический зазор относительно тела' : 'measure polyhedral ball clearance'
  if (operation === 'locate_section_point') return russian ? 'Положение точки в сечении' : 'locate section point'
  if (operation === 'measure_section_contour_distance') return russian ? 'Расстояние до контура сечения' : 'measure section contour distance'
  if (operation === 'measure_section_material_distance') return russian ? 'Расстояние до материала сечения' : 'measure section material distance'
  if (operation === 'clip_section_segment') return russian ? 'Отсечь отрезок по сечению' : 'clip section segment'
  if (operation === 'clip_section_polyline') return russian ? 'Отсечь ломаную по сечению' : 'clip section polyline'
  if (operation === 'measure_section_path_length') return russian ? 'Длина пути внутри сечения' : 'measure section path length'
  if (operation === 'classify_polyhedral_plane') return russian ? 'Положение тела относительно плоскости' : 'classify polyhedral plane'
  if (operation === 'intersect_polyhedral_edges') return russian ? 'Пересечения рёбер с плоскостью' : 'intersect polyhedral edges'
  if (operation === 'measure_polyhedral_volume') return russian ? 'Точный объём многогранника' : 'measure polyhedral volume'
  if (operation === 'measure_polyhedral_centroid') return russian ? 'Точный центр масс многогранника' : 'measure polyhedral centroid'
  if (operation === 'measure_polyhedral_inertia') return russian ? 'Точный тензор инерции многогранника' : 'measure polyhedral inertia'
  if (operation === 'measure_polyhedral_face_areas') return russian ? 'Точные площади граней' : 'measure polyhedral face areas'
  if (operation === 'bound_polyhedral_area') return russian ? 'Границы площади многогранника' : 'bound polyhedral area'
  if (operation === 'bound_polyhedral_edge_length') return russian ? 'Границы длины рёбер' : 'bound polyhedral edge length'
  if (operation === 'close_section_planes') return russian ? 'Закрыть оболочку по плоскостям' : 'close section planes'
  if (operation === 'build_bounded_section_solid') return russian ? 'Построить тело с допусками и проверить объём' : 'build bounded section solid'
  if (operation === 'close_bounded_planes') return russian ? 'Закрыть оболочку по плоскостям с допусками' : 'close bounded planes'
  if (operation === 'discover_bounded_rims') return russian ? 'Найти контуры и закрыть с допуском' : 'discover bounded rims'
  if (operation === 'expanded_section_sheets') return russian ? 'Построить точную расширенную опору' : 'expanded section sheets'
  if (operation === 'section_brep_sheets') return russian ? 'Листовые B-rep грани сечения' : 'section brep sheets'
  if (operation === 'bounded_section_brep_sheets') return russian ? 'Листовые B-rep грани с допуском опоры' : 'bounded section brep sheets'
  if (operation === 'cap_section_holes') return russian ? 'Крышка с отверстиями' : 'cap section holes'
  if (operation === 'arrange_section_caps') return russian ? 'Крышки по вложенным контурам' : 'arrange section caps'
  if (operation === 'exact_wire_mass') return russian ? 'Первые моменты и центр площади' : 'exact wire mass'
  if (operation === 'exact_wire_inertia') return russian ? 'Вторые моменты площади' : 'exact wire inertia'
  if (operation === 'bound_world_face_mass') return russian ? 'Границы центра и моментов площади' : 'bound world face mass'
  if (operation === 'bound_principal_face_moments') return russian ? 'Интервалы главных моментов' : 'bound principal face moments'
  if (operation === 'bound_principal_face_directions') return russian ? 'Границы главных направлений' : 'bound principal face directions'
  if (operation === 'world_face_mass') return russian ? 'Моменты площади в мировых координатах' : 'world face mass'
  if (operation === 'principal_face_moments') return russian ? 'Главные моменты площади' : 'principal face moments'
  if (operation === 'principal_face_directions') return russian ? 'Главные направления площади' : 'principal face directions'
  if (operation === 'coplanar_ray_contacts') return russian ? 'Контакты луча в плоскости грани' : 'coplanar ray contacts'
  if (operation === 'coplanar_ray_partition') return russian ? 'Участки луча внутри грани' : 'coplanar ray partition'
  if (operation === 'cast_trimmed_face_ray') return russian ? 'Пересечение луча с обрезанной гранью' : 'Ray intersection with trimmed face'
  if (operation === 'planar_face_perimeter') return russian ? 'Периметр грани и отверстий' : 'planar face perimeter'
  if (operation === 'planar_boundary_distance') return russian ? 'Расстояние до границы' : 'planar boundary distance'
  if (operation === 'planar_material_distance') return russian ? 'Расстояние до заполненной грани' : 'planar material distance'
  if (operation === 'signed_planar_face_area') return russian ? 'Знаковая площадь плоской грани' : 'Signed planar face area'
  if (operation === 'project_planar_wire') return russian ? 'Проекция контура на общую плоскость' : 'project planar wire'
  if (operation === 'lift_planar_wire') return russian ? 'Подъём плоского контура в 3D' : 'lift planar wire'
  if (operation === 'active_surface_hull') return russian ? 'Аффинная размерность поверхности' : 'active surface hull'
  if (operation === 'project_plane_chart') return russian ? 'Плоская проекция поверхности' : 'project plane chart'
  if (operation === 'store_section_edge') return russian ? 'Хранение ребра пересечения' : 'store section edge'
  if (operation === 'optimize_section_edge') return russian ? 'Согласованное округление ребра' : 'optimize section edge'
  if (operation === 'store_section_edges') return russian ? 'Хранение разделённых рёбер' : 'store section edges'
  if (operation === 'partition_face_events') return russian ? 'События пересечения контуров' : 'Partition boundary events'
  if (operation === 'enclose_face_events') return russian ? 'Пространственные границы точек пересечения' : 'Enclose boundary events'
  if (operation === 'match_quadratic_contacts') return russian ? 'Точные квадратичные контакты контуров' : 'Exact quadratic boundary contacts'
  if (operation === 'shared_boundary_spans') return russian ? 'Общие участки границ' : 'shared boundary spans'
  if (operation === 'shared_boundary_contacts') return russian ? 'Общие точки границ' : 'shared boundary contacts'
  if (operation === 'refine_boundary_section') return russian ? 'Уточнение общих участков' : 'refine boundary section'
  if (operation === 'resolve_boundary_section') return russian ? 'Уточнение контакта до пространственного допуска' : 'resolve boundary section'
  if (operation === 'bound_boundary_section') return russian ? 'Границы области контакта' : 'bound boundary section'
  if (operation === 'classify_section_gaps') return russian ? 'Материал между событиями сечения' : 'classify section gaps'
  if (operation === 'assemble_section_material') return russian ? 'Компоненты материала сечения' : 'assemble section material'
  if (operation === 'build_material_segments') return russian ? 'Геометрия компонентов сечения' : 'build material segments'
  if (operation === 'material_segment_curve') return russian ? 'Кривая компонента сечения' : 'material segment curve'
  if (operation === 'sample_plane_branch') return russian ? 'Точки ветви пересечения' : 'sample plane branch'
  if (operation === 'plane_branch_uv_polyline') return russian ? 'Аппроксимация ветви в UV' : 'plane branch uv polyline'
  if (operation === 'store_plane_branch') return russian ? 'Пространственная кривая сечения' : 'store plane branch'
  if (operation === 'split_cell_by_plane') return russian ? 'Разрез узловой ячейки плоскостью' : 'Split knot cell by plane'
  if (operation === 'pair_plane_branch') return russian ? 'Согласованные кривые сечения' : 'pair plane branch'
  if (operation === 'stitch_plane_strip') return russian ? 'Сшивка ветви через узловые ячейки' : 'stitch plane strip'
  if (operation === 'close_plane_strip') return russian ? 'Замыкание сечения на периодическом шве' : 'close plane strip'
  if (operation === 'split_periodic_plane_strip') return russian ? 'Разрез периодической поверхности' : 'Split periodic surface strip'
  if (operation === 'store_closed_plane_piece') return russian ? 'Кривая замкнутого сечения' : 'store closed plane piece'
  if (operation === 'trace_plane_strip') return russian ? 'Пространственная кривая через узловые ячейки' : 'trace plane strip'
  if (operation === 'plane_branch_xyz_polyline') return russian ? 'Пространственная аппроксимация ветви' : 'plane branch xyz polyline'
  if (operation === 'audit_plane_side') return russian ? 'Положение поверхности относительно плоскости' : 'audit plane side'
  if (operation === 'refine_plane_side') return russian ? 'Уточнение положения поверхности' : 'refine plane side'
  if (operation === 'audit_plane_graphs') return russian ? 'Ветви пересечения с плоскостью' : 'audit plane graphs'
  if (operation === 'measure_section_material') return russian ? 'Длина материала в сечении' : 'measure section material'
  if (operation === 'classify_face_section') return russian ? 'Классификация контактов двух граней' : 'Face contact classification'
  if (operation === 'clip_plane_section') return russian ? 'Общие участки двух обрезанных граней' : 'clip plane section'
  if (operation === 'build_plane_section_segments') return russian ? 'Отрезки пересечения и парные контуры' : 'build plane section segments'
  if (operation === 'intersect_support_planes') return russian ? 'Пересечение опорных плоскостей' : 'Support plane intersection'
  if (operation === 'lift_plane_chart') return russian ? 'Подъём плоской поверхности' : 'lift plane chart'
  if (operation === 'shared_plane_chart') return russian ? 'Общая плоскость поверхностей' : 'shared plane chart'
  if (operation === 'compress_tensor_surface') return russian ? 'Точное упрощение поверхности' : 'Exact surface compression'
  if (operation === 'merge_patch_strip') return russian ? 'Объединение полосы патчей' : 'Merge patch strip'
  if (operation === 'merge_patch_grid') return russian ? 'Объединение сетки патчей' : 'Merge patch grid'
  if (operation === 'inspect_patch_grid') return russian ? 'Распознавание сетки патчей' : 'Inspect patch grid'
  if (operation === 'compress_tensor_surface_batch') return russian ? 'Точное упрощение набора поверхностей' : 'Exact surface batch compression'
  if (operation === 'surface_gap_pair_distance') return russian ? 'Расстояние между контурами кривизны' : 'Curvature contour pair distance'
  if (operation === 'surface_gap_plane_section') return russian ? 'Сечение контура плоскостью' : 'Contour plane section'
  if (operation === 'surface_gap_world_distance') return russian ? 'Расстояние до пространственного контура' : 'Point to world contour distance'
  if (operation === 'surface_gap_world_bounds') return russian ? 'Габариты пространственного контура' : 'World contour bounds'
  if (operation === 'surface_gap_uv_polyline') return russian ? 'Отклонение UV-ломаной контура' : 'UV contour polyline deviation'
  if (operation === 'surface_gap_adaptive_polyline') return russian ? 'Адаптивная UV-ломаная контура' : 'Adaptive UV contour polyline'
  if (operation === 'surface_gap_second_derivative') return russian ? 'Вторая производная контура' : 'Contour second derivative'
  if (operation === 'surface_gap_second_order_polyline') return russian ? 'UV-ломаная второго порядка' : 'Second-order UV contour polyline'
  if (operation === 'surface_gap_contour_slope') return russian ? 'Диапазон наклона контура кривизны' : 'Curvature contour slope range'
  if (operation === 'surface_quadric_support') return russian ? 'Принадлежность поверхности квадрике' : 'Surface membership in a quadric'
  if (operation === 'cst_section_integrals') return russian ? 'Площадь и моменты сечения CST' : 'CST section area and moments'
  if (operation === 'fit_sphere_cloud') return russian ? 'Сфера по облаку точек' : 'Sphere fitted to cloud'
  if (operation === 'fit_cylinder_cloud') return russian ? 'Цилиндр по облаку точек' : 'Cylinder fitted to cloud'
  if (operation === 'fit_cone_cloud') return russian ? 'Конус по облаку точек' : 'Cone fitted to cloud'
  if (operation === 'fit_torus_cloud') return russian ? 'Тор по облаку точек' : 'Torus fitted to cloud'
  if (operation === 'fit_plane_cloud') return russian ? 'Плоскость по облаку точек' : 'Plane fitted to cloud'
  if (operation === 'surface_highlight_lines') return russian ? 'Бликовые линии' : 'Highlight lines'
  if (operation === 'surface_reflection_lines') return russian ? 'Линии отражения' : 'Reflection lines'
  if (operation === 'surface_isophote_analysis') return russian ? 'Изофоты поверхности' : 'Surface isophotes'
  if (operation === 'surface_dent_detection') return russian ? 'Зоны вмятин' : 'Dent zones'
  if (operation === 'curve_from_curvature_samples') return russian ? 'Кривая по знаковой кривизне' : 'Curve from signed curvature'
  if (operation === 'fit_biarc_piece') return russian ? 'Участок биарки' : 'Fitted biarc piece'
  if (operation === 'simplify_linear_curve') return russian ? 'Точное упрощение линейной кривой' : 'Exact linear curve simplification'
  if (operation === 'extend_terminal_curve') return russian ? 'Продление конца кривой' : 'Terminal curve extension'
  if (operation === 'extend_surface_boundary') return russian ? 'Продление границы поверхности' : 'Surface boundary extension'
  if (operation === 'affine_control_support') return russian ? 'Аффинная размерность контрольных точек' : 'Affine dimension of controls'
  if (operation === 'surface_trace_agreement') return russian ? 'Согласованность пространственной и UV-кривой' : 'Surface trace agreement'
  if (operation === 'signed_shell_volume') return russian ? 'Знаковый объём ориентированной оболочки' : 'Signed volume of an oriented shell'
  if (operation === 'geometry_bounds_union') return russian ? 'Общие габариты' : 'Combined geometry bounds'
  if (operation === 'geometry_conditioning') return russian ? 'Численные масштабы NURBS' : 'NURBS conditioning indicators'
  if (operation === 'shared_patch_mesh') return russian ? 'Сетка с общей границей патчей' : 'Shared patch mesh'
  if (operation === 'bounded_curve_chord') return russian ? 'Хорда кривой с допуском' : 'Bounded curve chord'
  if (operation === 'bounded_surface_mesh') return russian ? 'Сетка поверхности с допуском' : 'Bounded surface mesh'
  if (operation === 'parallel_plane_sections') return russian ? 'Сечения параллельными плоскостями' : 'Parallel plane sections'
  if (operation === 'parallel_section_piece') return russian ? 'Кривая плоского сечения' : 'Plane section curve'
  if (operation === 'clean_offset_boundary') return russian ? 'Граница очищенной эквидистанты' : 'Clean offset boundary'
  if (operation === 'planar_toolpath_edge') return russian ? 'Эквидистантная дорожка инструмента' : 'Planar toolpath'
  if (operation === 'planar_offset_piece') return russian ? 'Участок плоской эквидистанты' : 'Planar offset piece'
  if (operation === 'planar_knot_miter') return russian ? 'Угловой стык эквидистанты' : 'Offset miter at knot'
  if (operation === 'phase_aligned_section') return russian ? 'Фаза замкнутого сечения' : 'Closed section phase'
  if (operation === 'compatible_section') return russian ? 'Согласование степени и узлов сечений' : 'Compatible section basis'
  if (operation === 'oriented_section') return russian ? 'Ориентация сечения' : 'Section orientation'
  if (operation === 'uv_loop_closure') return russian ? 'Замкнутость UV-контура' : 'UV loop closure'
  if (operation === 'uv_loop_simplicity') return russian ? 'Простота UV-контура' : 'UV loop simplicity'
  if (operation === 'uv_trim_location') return russian ? 'Положение в области обрезки' : 'Trim domain location'
  if (operation === 'curve_edit_deviation') return russian ? 'Отклонение кривой после редактирования' : 'Curve edit deviation'
  if (operation === 'surface_edit_deviation') return russian ? 'Отклонение поверхности после редактирования' : 'Surface edit deviation'
  if (['surface_seam_g0','surface_seam_g1','surface_seam_g2'].includes(operation)) return (russian ? 'Сопряжение поверхностей ' : 'Surface seam ') + operation.slice(-2).toUpperCase()
  if (operation === 'curve_join_continuity') return russian ? 'Сопряжение кривых G0/G1/G2' : 'Curve join G0/G1/G2'
  if (operation === 'curve_self_contacts') return russian ? 'Самопересечения кривой' : 'Curve self-intersections'
  if (operation === 'surface_self_contacts') return russian ? 'Самопересечения поверхности' : 'Surface self-intersections'
  if (operation === 'surface_span_regularity') return russian ? 'Регулярность поверхности' : 'Surface regularity'
  if (operation === 'curve_plane_roots') return russian ? 'Пересечение кривой с плоскостью' : 'Curve–plane intersections'
  if (operation === 'curve_pair_crossings') return russian ? 'Пересечения кривых' : 'Curve crossings'
  if (operation === 'curve_plane_chart_crossings') return russian ? 'Пересечение кривой с плоской гранью' : 'Curve–planar patch intersections'
  if (operation === 'surface_ray_hits') return russian ? 'Пересечение луча с поверхностью' : 'Ray–surface intersections'
  if (operation === 'curve_curvature_torsion') return russian ? 'Кривизна и кручение кривой' : 'Curve curvature and torsion'
  if (operation === 'surface_principal_curvatures') return russian ? 'Главные кривизны поверхности' : 'Principal surface curvatures'
  if (operation === 'curve_nearest_point') return russian ? 'Ближайшая точка кривой' : 'Nearest curve point'
  if (operation === 'surface_nearest_point') return russian ? 'Ближайшая точка поверхности' : 'Nearest surface point'
  if (operation === 'curve_pair_distance') return russian ? 'Расстояние между кривыми' : 'Curve-to-curve distance'
  if (operation === 'curve_surface_distance') return russian ? 'Расстояние от кривой до поверхности' : 'Curve-to-surface distance'
  if (operation === 'surface_pair_distance') return russian ? 'Расстояние между поверхностями' : 'Surface-to-surface distance'
  if (operation === 'curve_length_bounds') return russian ? 'Длина кривой' : 'Curve length'
  if (operation === 'curve_station_at_length') return russian ? 'Точка на заданной длине' : 'Point at arc length'
  if (operation === 'curve_divide_by_length') return russian ? 'Деление по длине' : 'Equal arc-length division'
  if (operation === 'surface_area_bounds') return russian ? 'Площадь поверхности' : 'Surface area'
  if (operation === 'trimmed_surface_area_in_window') return russian ? 'Площадь внутри окна UV' : 'Area inside the UV window'
  if (operation === 'combine_surface_measures') return russian ? 'Площадь и инерция сборки' : 'Assembly area and inertia'
  if (operation === 'combine_surface_densities') return russian ? 'Масса и инерция сборки' : 'Assembly mass and inertia'
  if (operation === 'mass_weighted_centroid') return russian ? 'Границы центра масс' : 'Mass centroid bounds'
  if (operation === 'physical_principal_moments') return russian ? 'Физические главные моменты' : 'Physical principal moments'
  if (operation === 'physical_moment_gaps') return russian ? 'Разности главных моментов' : 'Principal moment gaps'
  if (operation === 'find_principal_axis') return russian ? 'Поиск главной оси' : 'Find principal axis'
  if (operation === 'bound_axis_projector') return russian ? 'Границы проектора главной оси' : 'Principal axis projector bounds'
  if (operation === 'certify_principal_direction') return russian ? 'Проверка направления главной оси' : 'Principal axis direction certificate'
  if (operation === 'minimum_variable_scene_gap') return russian ? 'Минимальный зазор в сцене при переменном радиусе' : 'Minimum variable scene gap'
  if (operation === 'minimum_profiled_scene_gap') return russian ? 'Минимальный зазор по профилю радиуса' : 'Minimum profiled scene gap'
  if (operation === 'minimum_mixed_gap') return russian ? 'Минимальный зазор в смешанной сцене' : 'Minimum mixed obstacle gap'
  if (operation === 'variable_scene_clearance') return russian ? 'Зазор в сцене при переменном радиусе' : 'Variable scene clearance'
  if (operation === 'profiled_scene_clearance') return russian ? 'Зазор по профилю радиуса' : 'Profiled scene clearance'
  if (operation === 'mixed_path_clearance') return russian ? 'Зазор в смешанной сцене' : 'Mixed obstacle clearance'
  if (operation === 'violation_path_length') return russian ? 'Длина нарушения зазора' : 'Clearance violation length'
  if (operation === 'violation_obstacle_lengths') return russian ? 'Длина нарушения по препятствиям' : 'Violation length per obstacle'
  if (operation === 'violation_path_duration') return russian ? 'Длительность нарушения зазора' : 'Clearance violation duration'
  if (operation === 'chain_violation_duration') return russian ? 'Длительность нарушения на составной траектории' : 'Chain violation duration'
  if (operation === 'last_chain_violation') return russian ? 'Последнее нарушение на составной траектории' : 'Last chain violation'
  if (operation === 'chain_clearance_timeline') return russian ? 'Интервалы зазора на составной траектории' : 'Chain clearance timeline'
  if (operation === 'longest_chain_violation') return russian ? 'Наибольшая длительность нарушения' : 'Longest chain violation'
  if (operation === 'minimum_synchronized_gap') return russian ? 'Минимальный синхронный зазор' : 'Minimum synchronized gap'
  if (operation === 'synchronized_contact_intervals') return russian ? 'Интервалы синхронного контакта' : 'Synchronized contact intervals'
  if (operation === 'minimum_tool_pair_gap') return russian ? 'Минимальный зазор пары инструментов' : 'Minimum tool pair gap'
  if (operation === 'tool_pair_intervals') return russian ? 'Интервалы контакта пар' : 'Tool pair intervals'
  if (operation === 'first_tool_pair_contact') return russian ? 'Первый контакт пары' : 'First tool pair contact'
  if (operation === 'last_tool_pair_contact') return russian ? 'Последний контакт пары' : 'Last tool pair contact'
  if (operation === 'tool_contact_duration') return russian ? 'Общая длительность контакта' : 'Total contact duration'
  if (operation === 'longest_tool_contact') return russian ? 'Наибольшая длительность контакта' : 'Longest continuous contact'
  if (operation === 'peak_tool_pair_count') return russian ? 'Наибольшее число контактирующих пар' : 'Peak contacting pair count'
  if (operation === 'simultaneous_tool_duration') return russian ? 'Длительность одновременного контакта' : 'Simultaneous contact duration'
  if (operation === 'simultaneous_tool_windows') return russian ? 'Окна одновременного контакта' : 'Simultaneous contact windows'
  if (operation === 'peak_tool_contacts') return russian ? 'Пары и время пикового контакта' : 'Peak contact witness'
  if (operation === 'tool_pairs_at_time') return russian ? 'Контактирующие пары в заданный момент' : 'Contact pairs at time'
  if (operation === 'minimum_scheduled_gap') return russian ? 'Минимальный зазор по расписанию' : 'Minimum scheduled gap'
  if (operation === 'scheduled_contact_intervals') return russian ? 'Интервалы контакта по расписанию' : 'Scheduled contact intervals'
  if (operation === 'held_contact_intervals') return russian ? 'Интервалы контакта с остановками' : 'Contact intervals with endpoint holds'
  if (operation === 'minimum_held_gap') return russian ? 'Минимальный зазор с остановками' : 'Minimum gap with endpoint holds'
  if (operation === 'minimum_affine_gap') return russian ? 'Минимальный зазор независимых окон' : 'Minimum gap on independent windows'
  if (operation === 'affine_contact_intervals') return russian ? 'Интервалы контакта независимых окон' : 'Contact intervals on independent windows'
  if (operation === 'first_chain_violation') return russian ? 'Первое нарушение на составной траектории' : 'First chain violation'
  if (operation === 'minimum_chain_gap') return russian ? 'Минимальный зазор составной траектории' : 'Minimum chain gap'
  if (operation === 'mixed_margin_clearance') return russian ? 'Запас зазора в смешанной сцене' : 'Mixed clearance margin'
  if (operation === 'minimum_variable_sphere_gap') return russian ? 'Минимальный зазор до сфер при переменном радиусе' : 'Minimum variable sphere gap'
  if (operation === 'minimum_sphere_gap') return russian ? 'Минимальный зазор до сфер' : 'Minimum sphere gap'
  if (operation === 'minimum_box_gap') return russian ? 'Минимальный зазор до блоков' : 'Minimum box gap'
  if (operation === 'minimum_oriented_box_gap') return russian ? 'Минимальный зазор до ориентированных блоков' : 'Minimum oriented box gap'
  if (operation === 'minimum_capsule_gap') return russian ? 'Минимальный зазор до капсул' : 'Minimum capsule gap'
  if (operation === 'variable_sphere_clearance') return russian ? 'Зазор до сфер при переменном радиусе' : 'Variable sphere clearance'
  if (operation === 'angular_speed_limit') return russian ? 'Ограничение угловой скорости' : 'Angular speed limit'
  if (operation === 'angular_acceleration_limit') return russian ? 'Ограничение углового ускорения' : 'Angular acceleration limit'
  if (operation === 'angular_jerk_limit') return russian ? 'Ограничение углового рывка' : 'Angular jerk limit'
  if (operation === 'orientation_sector') return russian ? 'Сектор ориентации' : 'Orientation sector'
  if (operation === 'relative_orientation_sector') return russian ? 'Сектор относительной ориентации' : 'Relative orientation sector'
  if (operation === 'relative_angular_limits') return russian ? 'Относительные угловые ограничения' : 'Relative angular limits'
  if (operation === 'orientation_winding') return russian ? 'Число оборотов ориентации' : 'Orientation winding'
  if (operation === 'curve_winding_at_point') return russian ? 'Число обходов точки' : 'Curve winding at point'
  if (operation === 'contour_fill_at_point') return russian ? 'Заполнение в точке' : 'Contour fill at point'
  if (operation === 'curve_winding_over_box') return russian ? 'Число обходов прямоугольной области' : 'Curve winding over box'
  if (operation === 'contour_box_distance') return russian ? 'Расстояние от контуров до прямоугольника' : 'Contour box distance'
  if (operation === 'contour_disk_distance') return russian ? 'Расстояние от контуров до диска' : 'Contour disk distance'
  if (operation === 'contour_capsule_distance') return russian ? 'Расстояние от контуров до капсулы' : 'Contour capsule distance'
  if (operation === 'contour_rectangle_distance') return russian ? 'Расстояние от контуров до повёрнутого прямоугольника' : 'Contour rectangle distance'
  if (operation === 'planar_contour_sets_distance') return russian ? 'Расстояние между наборами контуров' : 'Planar contour sets distance'
  if (operation === 'contour_placement') return russian ? 'Положение контуров относительно области' : 'Contour placement'
  if (operation === 'contour_nesting') return russian ? 'Вложенность контуров' : 'Contour nesting'
  if (operation === 'orient_fill_contour') return russian ? 'Ориентировать границу заливки' : 'Orient fill contour'
  if (operation === 'contour_fill_area') return russian ? 'Площадь заливки по чётности' : 'Contour fill area'
  if (operation === 'contour_fill_centroid') return russian ? 'Центр площади заливки' : 'Contour fill centroid'
  if (operation === 'contour_central_moments') return russian ? 'Центральные моменты заливки' : 'Contour central moments'
  if (operation === 'contour_principal_moments') return russian ? 'Главные моменты заливки' : 'Contour principal moments'
  if (operation === 'contour_principal_axes') return russian ? 'Главные оси заливки' : 'Contour principal axes'
  if (operation === 'contour_nonzero_area') return russian ? 'Площадь ненулевой заливки' : 'Contour nonzero area'
  if (operation === 'contour_fill_perimeter') return russian ? 'Периметр заливки' : 'Contour fill perimeter'
  if (operation === 'contour_fill_topology') return russian ? 'Компоненты и отверстия заливки' : 'Contour fill topology'
  if (operation === 'contour_boundary_signed_distance') return russian ? 'Знаковое расстояние до границы заливки' : 'Contour boundary signed distance'
  if (operation === 'contour_hierarchy') return russian ? 'Иерархия контуров' : 'Contour hierarchy'
  if (operation === 'contour_fill_orientations') return russian ? 'Ориентация границ заливки' : 'Contour fill orientations'
  if (operation === 'signed_contour_distance') return russian ? 'Знаковое расстояние до контуров' : 'Signed contour distance'
  if (operation === 'contour_fill_over_box') return russian ? 'Заполнение прямоугольной области' : 'Contour fill over box'
  if (operation === 'signed_contour_box_distance') return russian ? 'Знаковое расстояние по прямоугольной области' : 'Signed contour box distance'
  if (operation === 'angular_travel') return russian ? 'Угловой ход' : 'Angular travel'
  if (operation === 'relative_angular_travel') return russian ? 'Относительный угловой ход' : 'Relative angular travel'
  if (operation === 'angular_motion_limits') return russian ? 'Пределы углового движения' : 'Angular motion limits'
  if (operation === 'angular_slowdown_bounds') return russian ? 'Замедление углового движения' : 'Angular slowdown bounds'
  if (operation === 'angular_duration_bounds') return russian ? 'Длительность углового движения' : 'Angular duration bounds'
  if (operation === 'angular_motion_deadline') return russian ? 'Проверка срока углового движения' : 'Angular motion deadline'
  if (operation === 'sphere_path_clearance') return russian ? 'Зазор до сфер' : 'Sphere path clearance'
  if (operation === 'capsule_path_clearance') return russian ? 'Зазор до капсул' : 'Capsule path clearance'
  if (operation === 'box_path_clearance') return russian ? 'Зазор до прямоугольных блоков' : 'Box path clearance'
  if (operation === 'oriented_box_path_clearance') return russian ? 'Зазор до ориентированных блоков' : 'Oriented box path clearance'
  if (operation === 'first_sphere_contact') return russian ? 'Первый контакт со сферой' : 'First sphere contact'
  if (operation === 'minimum_sphere_partition_gap') return russian ? 'Минимальный зазор по разбиению' : 'Minimum partition gap'
  if (operation === 'point_workspace') return russian ? 'Рабочая область точки' : 'Point workspace'
  if (operation === 'point_obstacles') return russian ? 'Препятствия для точки' : 'Point obstacle clearance'
  if (operation === 'ball_obstacles') return russian ? 'Препятствия для шара' : 'Ball obstacle clearance'
  if (operation === 'profiled_ball_workspace') return russian ? 'Рабочая область для шара переменного радиуса' : 'Profiled ball workspace'
  if (operation === 'minimum_profiled_ball_workspace') return russian ? 'Минимальный запас шара переменного радиуса' : 'Minimum profiled ball workspace'
  if (operation === 'ball_workspace') return russian ? 'Рабочая область шара' : 'Ball workspace'
  if (operation === 'capsule_workspace') return russian ? 'Рабочая область капсулы' : 'Capsule workspace'
  if (operation === 'profiled_hull_workspace') return russian ? 'Рабочая область оболочки с переменным скруглением' : 'Profiled hull workspace'
  if (operation === 'minimum_profiled_hull_workspace') return russian ? 'Минимальный запас оболочки с переменным скруглением' : 'Minimum profiled hull workspace'
  if (operation === 'rotating_hull_workspace') return russian ? 'Рабочая область вращающегося инструмента' : 'Rotating hull workspace'
  if (operation === 'minimum_rotating_hull_gap') return russian ? 'Минимальный запас вращающегося инструмента' : 'Minimum rotating hull gap'
  if (operation === 'profiled_rotating_workspace') return russian ? 'Рабочая область при вращении с переменным скруглением' : 'Profiled rotating workspace'
  if (operation === 'minimum_profiled_rotating_gap') return russian ? 'Минимальный запас при вращении с переменным скруглением' : 'Minimum profiled rotating gap'
  if (operation === 'hull_workspace') return russian ? 'Рабочая область выпуклого инструмента' : 'Rounded hull workspace'
  if (operation === 'rotating_hull_obstacles') return russian ? 'Препятствия для вращающегося инструмента' : 'Rotating hull obstacles'
  if (operation === 'rounded_rotating_obstacles') return russian ? 'Препятствия для вращающегося скруглённого инструмента' : 'Rounded rotating obstacles'
  if (operation === 'rotating_face_obstacles') return russian ? 'Контакт скругления с гранью' : 'Rotating face obstacles'
  if (operation === 'rotating_corner_obstacles') return russian ? 'Диагональный контакт с углом' : 'Rotating corner obstacles'
  if (operation === 'rotating_asymmetric_obstacles') return russian ? 'Несимметричный контакт с углом' : 'Rotating asymmetric obstacles'
  if (operation === 'rotating_skew_obstacles') return russian ? 'Контакт с косым углом' : 'Rotating skew obstacles'
  if (operation === 'rotating_chord_obstacles') return russian ? 'Контакт середины хорды' : 'Rotating chord obstacles'
  if (operation === 'rotating_clipped_chord_obstacles') return russian ? 'Контакт отсечённой хорды' : 'Rotating clipped chord obstacles'
  if (operation === 'rotating_exact_chord_obstacles') return russian ? 'Точное касание хорды' : 'Rotating exact chord obstacles'
  if (operation === 'profiled_rotating_obstacles') return russian ? 'Препятствия при вращении с переменным скруглением' : 'Profiled rotating obstacles'
  if (operation === 'hull_obstacles') return russian ? 'Препятствия для выпуклого инструмента' : 'Rounded hull obstacle clearance'
  if (operation === 'partition_axis_position') return russian ? 'Диапазон координаты траектории' : 'Axial position range'
  if (operation === 'partition_axis_positions') return russian ? 'Диапазоны координат траектории' : 'Multi-axis position ranges'
  if (operation === 'partition_trajectory_bounds') return russian ? 'Ограничения координат и движения' : 'Trajectory position and motion bounds'
  if (operation === 'check_timed_chain') return russian ? 'Ограничения движения по цепочке' : 'Timed chain limits'
  if (operation === 'inspect_knot_turns') return russian ? 'Повороты в узлах' : 'Knot turns'
  if (operation === 'inspect_join_turn') return russian ? 'Поворот на стыке' : 'Join turn'
  if (operation === 'inspect_chain_turns') return russian ? 'Повороты цепочки' : 'Chain turns'
  if (operation === 'inspect_knot_jumps') return russian ? 'Скачки в узлах' : 'Knot derivative jumps'
  if (operation === 'inspect_timed_knot_jumps') return russian ? 'Скачки при заданной длительности' : 'Timed knot jumps'
  if (operation === 'inspect_motion_join') return russian ? 'Проверка стыка траекторий' : 'Motion join check'
  if (operation === 'inspect_motion_chain') return russian ? 'Проверка цепочки траекторий' : 'Motion chain check'
  if (operation === 'tensor_principal_bounds') return russian ? 'Интервалы главных моментов' : 'Principal moment intervals'
  if (operation === 'tensor_axis_bounds') return russian ? 'Момент относительно оси' : 'Moment about an axis'
  if (operation === 'trim_set_axis_bounds') return russian ? 'Осевой момент обрезанных компонент' : 'Trimmed component axis moment'
  if (operation === 'periodic_trim_set_area') return russian ? 'Суммарная площадь компонент' : 'Total trimmed area'
  if (operation === 'periodic_trim_set_centroid') return russian ? 'Центр площади компонент' : 'Combined area centroid'
  if (operation === 'trimmed_surface_centroid') return russian ? 'Центр площади' : 'Area centroid'
  if (operation === 'trimmed_surface_area_inertia') return russian ? 'Тензор инерции площади' : 'Area inertia tensor'
  if (operation === 'finite_external_thread') return russian ? 'Наружная резьба' : 'External thread'
  if (operation === 'finite_internal_thread') return russian ? 'Внутренняя резьба' : 'Internal thread'
  if (operation === 'finite_za_worm') return russian ? 'Цилиндрический червяк ZA' : 'ZA worm'
  if (operation === 'finite_za_wheel') return russian ? 'Червячное колесо ZA' : 'ZA worm wheel'
  if (operation.startsWith('radial_cam_')) return russian ? 'Профиль кулачка' : 'Cam profile'
  if (operation === 'rack_profile' || operation === 'brep_rack') return russian ? 'Зубчатая рейка' : 'Gear rack'
  if (operation === 'open_step_file') return russian ? 'Импорт файла STEP' : 'Imported STEP file'
  if (operation === 'make_step_assembly') return russian ? 'Сборка из размещённых тел' : 'Make step assembly'
  if (operation === 'step_part_tolerances') return russian ? 'Допуски определений' : 'Step part tolerances'
  if (operation === 'set_scaled_step_placement') return russian ? 'Масштаб и отражение экземпляра' : 'Set scaled step placement'
  if (operation === 'step_occurrence_bounds') return russian ? 'Габариты экземпляров' : 'Step occurrence bounds'
  if (operation === 'step_proximity_pairs') return russian ? 'Близкие пары' : 'Step proximity pairs'
  if (operation === 'step_common_material') return russian ? 'Общий материал' : 'Step common material'
  if (operation === 'step_interferences') return russian ? 'Проверка пересечений' : 'Step interferences'
  if (operation === 'step_material_volume') return russian ? 'Суммарный объём' : 'Step material volume'
  if (operation === 'step_material_centroid') return russian ? 'Центр объёма' : 'Step material centroid'
  if (operation === 'step_material_inertia') return russian ? 'Тензор инерции' : 'Step material inertia'
  if (operation === 'step_material_area') return russian ? 'Суммарная площадь' : 'Step material area'
  if (operation === 'step_principal_inertia') return russian ? 'Главные моменты' : 'Step principal inertia'
  if (operation === 'step_mass_properties') return russian ? 'Массовые свойства' : 'Step mass properties'
  if (operation === 'step_ray_crossings') return russian ? 'Пересечения луча' : 'Step ray crossings'
  if (operation === 'step_first_ray_crossing') return russian ? 'Ближайшее пересечение луча' : 'Step first ray crossing'
  if (operation === 'step_ray_candidates') return russian ? 'Кандидаты пересечения луча' : 'Step ray candidates'
  if (operation === 'step_ray_intervals') return russian ? 'Интервалы материала' : 'Step ray intervals'
  if (operation === 'step_ray_length') return russian ? 'Длина материала' : 'Step ray length'
  if (operation === 'qualify_step_instance') return russian ? 'Проверка замкнутого экземпляра' : 'Qualify step instance'
  if (operation === 'step_ray_coverage') return russian ? 'Объединение материала вдоль луча' : 'Step ray coverage'
  if (operation === 'step_ray_multiplicity') return russian ? 'Кратность материала' : 'Step ray multiplicity'
  if (operation === 'step_pair_clearance') return russian ? 'Зазор пары деталей' : 'Step pair clearance'
  if (operation === 'step_clearance_checks') return russian ? 'Проверка минимальных зазоров' : 'Step clearance checks'
  if (operation === 'step_minimum_clearance') return russian ? 'Наименьший зазор сборки' : 'Step minimum clearance'
  if (operation === 'step_polyhedral_sections') return russian ? 'Точные сечения экземпляров' : 'Step polyhedral sections'
  if (operation === 'step_cut_stock') return russian ? 'Вычитание инструментов' : 'Step cut stock'
  if (operation === 'step_fuse_instances') return russian ? 'Объединение экземпляров' : 'Step fuse instances'
  if (operation === 'step_revision_delta') return russian ? 'Разница редакций' : 'Step revision delta'
  if (operation === 'step_split_instance') return russian ? 'Разрез экземпляра' : 'Step split instance'
  if (operation === 'step_stock_delta') return russian ? 'Припуск заготовки' : 'Step stock delta'
  if (operation === 'step_slice_layers') return russian ? 'Слои детали' : 'Step slice layers'
  if (operation === 'step_clip_material') return russian ? 'Материал в области' : 'Step clip material'
  if (operation === 'step_free_space') return russian ? 'Свободное пространство' : 'Step free space'
  if (operation === 'step_interference_regions') return russian ? 'Области пересечений' : 'Step interference regions'
  if (operation === 'step_interference_material') return russian ? 'Материал пересечений' : 'Step interference material'
  if (operation === 'step_interference_groups') return russian ? 'Группы пересечений' : 'Step interference groups'
  if (operation === 'step_group_interferences') return russian ? 'Пересечения групп' : 'Step group interferences'
  if (operation === 'step_group_clearances') return russian ? 'Зазоры групп' : 'Step group clearances'
  if (operation === 'step_group_minimum_gap') return russian ? 'Минимальный зазор групп' : 'Step group minimum gap'
  if (operation === 'move_step_group') return russian ? 'Перемещение группы' : 'Move step group'
  if (operation === 'copy_step_group') return russian ? 'Копирование группы' : 'Copy step group'
  if (operation === 'replace_step_variant') return russian ? 'Замена группы экземпляров' : 'Replace step variant'
  if (operation === 'eccentric_bore_stock') return russian ? 'Перемычка смещённого отверстия' : 'Eccentric bore stock'
  if (operation === 'ring_bolt_stock') return russian ? 'Перемычки крепежа венца' : 'Ring bolt stock'
  if (operation === 'ring_bolt_window') return russian ? 'Диапазон радиуса крепежа' : 'Ring bolt window'
  if (operation === 'external_spur_ratio') return russian ? 'Перекрытие внешней пары' : 'External spur ratio'
  if (operation === 'internal_spur_ratio') return russian ? 'Перекрытие внутренней пары' : 'Internal spur ratio'
  if (operation === 'mounted_spur_ratio') return russian ? 'Перекрытие при установке' : 'Mounted spur ratio'
  if (operation === 'spur_tip_clearance') return russian ? 'Зазоры вершин зубьев' : 'Spur tip clearance'
  if (operation === 'spur_tooth_chord') return russian ? 'Толщина по хорде' : 'Spur tooth chord'
  if (operation === 'spur_tooth_thinning') return russian ? 'Утонение по хорде' : 'Spur tooth thinning'
  if (operation === 'external_spur_backlash') return russian ? 'Зазор внешней пары' : 'External spur backlash'
  if (operation === 'internal_spur_backlash') return russian ? 'Зазор внутренней пары' : 'Internal spur backlash'
  if (operation === 'spur_backlash_range') return russian ? 'Допуск бокового зазора' : 'Spur backlash range'
  if (operation === 'spur_overlap_range') return russian ? 'Допуск перекрытия' : 'Spur overlap range'
  if (operation === 'spur_mounting_window') return russian ? 'Диапазон установки' : 'Spur mounting window'
  if (operation === 'planet_tip_separation') return russian ? 'Зазор между сателлитами' : 'Planet tip separation'
  if (operation === 'spinner_layout_counts') return russian ? 'Количество сателлитов' : 'Spinner layout counts'
  if (operation === 'compress_profile_curve') return russian ? 'Точное упрощение кривой' : 'Compress profile curve'
  if (operation === 'compress_profile_segments') return russian ? 'Точное упрощение сегментов' : 'Compress profile segments'
  if (operation === 'compress_profile_chain') return russian ? 'Сжатие цепочки разных степеней' : 'Compress profile chain'
  if (operation === 'orient_profile_chain') return russian ? 'Ориентация цепочки' : 'Orient profile chain'
  if (operation === 'restore_profile_partition') return russian ? 'Сборка по интервалам параметра' : 'Restore profile partition'
  if (operation === 'assemble_open_profile') return russian ? 'Открытый профиль' : 'Assemble open profile'
  if (operation === 'assemble_closed_profile') return russian ? 'Замкнутый профиль' : 'Assemble closed profile'
  if (operation === 'assemble_planar_profile') return russian ? 'Простой плоский профиль' : 'Assemble planar profile'
  if (operation === 'orient_planar_profile') return russian ? 'Направление обхода профиля' : 'Orient planar profile'
  if (operation === 'assemble_projected_profile') return russian ? 'Пространственный профиль по проекции' : 'Assemble projected profile'
  if (operation === 'assemble_oblique_profile') return russian ? 'Профиль по наклонной проекции' : 'Assemble oblique profile'
  if (operation === 'assemble_spatial_profile') return russian ? 'Пространственный профиль' : 'Assemble spatial profile'
  if (operation === 'curve_patch_clearance') return russian ? 'Зазор кривой до поверхности' : 'Curve patch clearance'
  if (operation === 'patch_pair_clearance') return russian ? 'Зазор между поверхностями' : 'Patch pair clearance'
  if (operation === 'trimmed_face_clearance') return russian ? 'Зазор между обрезанными гранями' : 'Trimmed face clearance'
  if (operation === 'curve_trimmed_face_distance') return russian ? 'Расстояние до обрезанной грани' : 'Curve trimmed face distance'
  if (operation === 'swept_ball_face_clearance') return russian ? 'Зазор сферического инструмента до грани' : 'Swept ball face clearance'
  if (operation === 'routed_tube_pair') return russian ? 'Зазор между трубками' : 'Tube clearance'
  if (operation === 'routed_tube_bundle') return russian ? 'Минимальный зазор в пучке трубок' : 'Minimum tube bundle clearance'
  if (operation === 'spatial_contour_sets_distance') return russian ? 'Расстояние между наборами контуров' : 'Spatial contour sets distance'
  if (operation === 'closest_spatial_contours') return russian ? 'Ближайшая пара контуров' : 'Closest spatial contours'
  if (operation === 'inspect_spatial_clearances') return russian ? 'Монтажные зазоры контуров' : 'Inspect spatial clearances'
  if (operation === 'prove_spatial_clearance_along') return russian ? 'Зазор по опорной плоскости' : 'Prove spatial clearance along'
  if (operation === 'repeat_profile_segment') return russian ? 'Круговой массив сегмента' : 'Repeat profile segment'
  if (operation === 'compress_rotated_segments') return russian ? 'Свёртка повёрнутых сегментов' : 'Compress rotated segments'
  if (operation === 'compress_profile_sectors') return russian ? 'Свёртка многосегментных секторов' : 'Compress profile sectors'
  if (operation === 'discover_profile_sectors') return russian ? 'Поиск повторяющегося сектора' : 'Discover profile sectors'
  if (operation === 'restore_lossless_sector') return russian ? 'Восстановление сектора без потерь' : 'Restore lossless sector'
  if (operation === 'restore_bidirectional_sector') return russian ? 'Двунаправленное восстановление сектора' : 'Restore bidirectional sector'
  if (operation === 'read_shared_step') return russian ? 'Импорт общей STEP-сборки' : 'Read shared step'
  if (operation === 'replace_step_part') return russian ? 'Замена общей детали' : 'Replace step part'
  if (operation === 'replace_step_instance') return russian ? 'Локальная замена экземпляра' : 'Replace step instance'
  if (operation === 'place_step_instance') return russian ? 'Размещение экземпляра' : 'Place step instance'
  if (operation === 'remove_step_instance') return russian ? 'Удаление экземпляра' : 'Remove step instance'
  if (operation === 'add_step_instance') return russian ? 'Новый экземпляр' : 'Add step instance'
  if (operation === 'insert_step_solid') return russian ? 'Добавление тела' : 'Insert step solid'
  if (operation === 'append_step_assembly') return russian ? 'Объединение сборок' : 'Append step assembly'
  if (operation === 'deduplicate_step_parts') return russian ? 'Объединение одинаковых деталей' : 'Deduplicate step parts'
  if (operation === 'extract_step_instances') return russian ? 'Извлечение экземпляров' : 'Extract step instances'
  if (operation === 'planet_gear_occurrences') return russian ? 'Общая геометрия сателлитов' : 'Shared planet gear geometry'
  if (operation === 'planet_occurrences_step') return russian ? 'Сборка сателлитов STEP' : 'STEP planet gear occurrences'
  if (operation === 'planetary_gear_set_step') return russian ? 'Планетарная сборка STEP' : 'STEP planetary gear assembly'
  if (operation === 'verified_bevel_gear') return russian ? 'Точность конической шестерни' : 'Bevel gear accuracy'
  if (operation === 'brep_nurbs_gear_span') return russian ? 'Длина общей нормали' : 'Base tangent measurement'
  if (operation === 'brep_nurbs_verified_gear') return russian ? 'Точность зубчатого профиля' : 'Gear profile accuracy'
  if (operation === 'surface_trim_range_length') return russian ? 'Длина участка контура' : 'Contour range length'
  if (operation.startsWith('surface_trim_sheet')) return russian ? 'Обрезанная оболочка' : 'Trimmed sheet'
  if (operation.startsWith('surface_trim_contour_')) return russian ? 'Измерение контура' : 'Contour measurement'
  if (operation === 'surface_trim_patch_area_vector') return russian ? 'Вектор площади' : 'Area vector'
  if (operation === 'surface_trim_patch_projected_area') return russian ? 'Площадь проекции' : 'Projected area'
  if (operation.startsWith('surface_trim_')) return russian ? 'Свойства обрезанной поверхности' : 'Trimmed surface properties'
  if (operation === 'surface_measure_trimmed_mass') return russian ? 'Масса обрезанных участков' : 'Trimmed surface mass'
  const motionLabels: Record<string, [string, string]> = {
    curve_axis_motion_partition_speed: ['Участки по лимиту скорости', 'Speed limit intervals'],
    curve_axis_motion_partition_motion: ['Участки по ограничениям движения', 'Motion limit intervals'],
    curve_axis_motion_partition_axes: ['Участки по нескольким осям', 'Multi-axis limit intervals'],
    curve_axis_motion_limits: ['Ограничения движения', 'Motion limits'],
    curve_axis_motion_slowdown: ['Замедление по оси', 'Axis slowdown'],
    curve_axis_motion_multi_slowdown: ['Замедление по нескольким осям', 'Multi-axis slowdown'],
    curve_axis_motion_duration: ['Длительность прохода', 'Traversal duration'],
    curve_axis_motion_deadline: ['Проверка срока прохода', 'Traversal deadline'],
  }
  if (Object.hasOwn(motionLabels, operation)) return motionLabels[operation]![russian ? 0 : 1]
  if (operation === 'curve_axial_derivative_peak') return russian ? 'Пик осевой производной' : 'Peak axial derivative'
  return russian ? 'Ход вдоль оси' : 'Travel along axis'
}

/** Present native bounds without rounding a small unresolved interval to zero. */
export function directionalTravelSummary(operation: string, report: string, russian: boolean): string[] {
  let parsed
  try {
    parsed = JSON.parse(report)
  } catch {
    return []
  }
  const unit = russian ? 'мм' : 'mm'
  const format = (value: number) => value.toLocaleString(russian ? 'ru' : 'en', { maximumSignificantDigits: 17 })
  const bounds = (pair: number[]) => pair[0] === pair[1] ? format(pair[0]!) : pair.map(format).join(' … ')
  const pair = (value: unknown): value is number[] => Array.isArray(value) && value.length === 2 && value.every(Number.isFinite)
  if (operation.includes('periodic_surface_region') || operation.includes('periodic_surface_contour_forest') || operation.includes('periodic_surface_path')) {
    if (parsed?.location) return [String(parsed.location)]
    if (Array.isArray(parsed?.locations)) return [parsed.locations.join(', ')]
    if (typeof parsed?.simple === 'boolean') return [russian ? (parsed.simple ? 'Простой контур' : 'Контур с самоконтактом') : (parsed.simple ? 'Simple contour' : 'Self-contacting contour')]
    if (typeof parsed?.disjoint === 'boolean') return [russian ? (parsed.disjoint ? 'Границы раздельны' : 'Границы соприкасаются') : (parsed.disjoint ? 'Disjoint boundaries' : 'Boundary contact')]
    if (parsed?.relation?.kind) return [parsed.relation.kind]
    if (Array.isArray(parsed?.nodes)) return [(russian ? 'Контуры · ' : 'Contours · ') + parsed.nodes.length]
    if (parsed?.area) return [(russian ? 'Площадь UV · ' : 'UV area · ') + parsed.area.numerator + '/' + parsed.area.denominator]
    if (Array.isArray(parsed?.components)) return [(russian ? 'Компоненты · ' : 'Components · ') + parsed.components.length]
    if (parsed?.refusal) return [String(parsed.refusal)]
  }
  if (operation.includes('periodic_surface_path') || operation.includes('wrapped_surface_point')) {
    if (parsed?.refusal) return [String(parsed.refusal)]
    if (parsed?.signedArea) return [(russian ? 'Ориентированная площадь UV' : 'Signed UV area') + ' · ' + parsed.signedArea.numerator + '/' + parsed.signedArea.denominator]
    if (pair(parsed?.winding)) return [(russian ? 'Обороты U, V' : 'U, V winding') + ' · ' + parsed.winding.join(', ')]
    if (Array.isArray(parsed?.point)) return ['XYZ · ' + parsed.point.map(format).join(', ') + ' ' + unit]
    const entries = parsed?.samples ?? parsed?.parameters
    return Array.isArray(entries) ? [(russian ? 'Число точек · ' : 'Points · ') + entries.length] : []
  }
  if (operation.includes('input_contour') || operation.includes('input_segment') || operation.includes('input_patch_max')) {
    if (parsed?.buildRefused || parsed?.refusal) return [russian ? 'Построение или измерение не подтверждено' : 'Construction or measurement unresolved']
    if (pair(parsed?.length?.boundsMm)) return [(russian ? 'Длина · ' : 'Length · ') + bounds(parsed.length.boundsMm) + ' ' + unit]
    if (pair(parsed?.parameterBounds)) return ['u ∈ [' + parsed.parameterBounds.map(format).join(' … ') + ']']
    if (Array.isArray(parsed?.stations)) return [(russian ? 'Число точек · ' : 'Stations · ') + parsed.stations.length]
    if (Array.isArray(parsed?.vertexPrefixBoundsMm)) return [(russian ? 'Число вершин · ' : 'Vertices · ') + parsed.vertexPrefixBoundsMm.length]
    if (pair(parsed?.maximumBoundsMm2)) return [(russian ? 'Максимальная проекция · ' : 'Maximum projection · ') + bounds(parsed.maximumBoundsMm2) + ' ' + unit + '²']
    if (Array.isArray(parsed?.exactAreaVector)) return parsed.exactAreaVector.map((q:{numerator:string;denominator:string},i:number)=>['X','Y','Z'][i]+' · '+q.numerator+'/'+q.denominator+' '+unit+'²')
    if (Array.isArray(parsed?.areaVectorBoundsMm2)) return parsed.areaVectorBoundsMm2.filter(pair).map((v:number[],i:number)=>['X','Y','Z'][i]+' · '+bounds(v)+' '+unit+'²')
    return [russian ? 'Результат неопределён' : 'Result unresolved']
  }
  if (['locate_input_parameter', 'locate_edge_parameter', 'locate_input_range', 'extract_input_range_curves', 'extract_input_range_curve_pairs'].includes(operation)) {
    if (parsed?.refusal) return [String(parsed.refusal)]
    if (Array.isArray(parsed?.result)) return [String(parsed.result.length) + (russian ? ' участков' : ' pieces')]
    return parsed?.result ? [String(parsed.result.contour) + ' / ' + String(parsed.result.segment)] : [russian ? 'Ребро не найдено' : 'Edge not found']
  }
  if (['evaluate_input_point','evaluate_input_tangents','classify_input_tangent_join','classify_input_vertex_join'].includes(operation)) {
    if (parsed?.refusal) return [String(parsed.refusal)]
    const exact = (p:{numerator:string;denominator:string}[]) => p.map(v=>v.numerator+'/'+v.denominator).join(', ')
    if (Array.isArray(parsed?.point)) return ['XYZ · ' + exact(parsed.point) + ' ' + unit]
    if (parsed?.join) {
      const labels: Record<string,string> = {Corner:'Угол',Reversal:'Разворот',OneSided:'Односторонняя касательная',Stationary:'Стационарная точка'}
      return [russian ? labels[parsed.join] ?? String(parsed.join) : String(parsed.join)]
    }
    if (parsed?.tangents) {
      const sides = [['before',russian ? 'До стыка' : 'Before'],['after',russian ? 'После стыка' : 'After']] as const
      return sides.flatMap(([key,label]) => Array.isArray(parsed.tangents[key]) ? [label + ' · ' + exact(parsed.tangents[key])] : [])
    }
    return [russian ? 'Параметр вне ребра' : 'Parameter outside edge']
  }
  if (['lift_exact_periodic_surface_region_components', 'build_exact_periodic_surface_trim_components', 'build_exact_periodic_surface_trim_patches', 'build_exact_periodic_surface_trim_edges'].includes(operation)) {
    if (parsed?.refusal) return [String(parsed.refusal)]
    return [(russian ? 'Компоненты · ' : 'Components · ') + parsed?.components?.length]
  }
  if (['trimmed_surface_area_in_window', 'trimmed_surface_centroid', 'trimmed_surface_area_inertia'].includes(operation)) {
    if (parsed?.refusal) return [String(parsed.refusal)]
    if (parsed?.centroidBoundsMm) return parsed.centroidBoundsMm.map((b:number[],i:number) => 'XYZ'[i] + ' · ' + b.join(' … ') + ' ' + unit)
    if (parsed?.inertiaBoundsMm4) return parsed.inertiaBoundsMm4.map((row:number[][],i:number) => 'XYZ'[i] + ' · ' + row.map(b=>b.join(' … ')).join(' / '))
    return parsed?.measure?.area ? [parsed.measure.area.boundsMm2.join(' … ') + ' ' + unit + '²'] : [russian ? 'Граница не получена' : 'Bound unavailable']
  }
  if (operation.startsWith('radial_cam_')) return [
    (russian ? 'Выборочная погрешность · ' : 'Sampled deviation · ') + format(parsed.maxSampledErrorMm) + ' ' + unit,
    (russian ? 'Участки · ' : 'Spans · ') + parsed.spans,
    ...(parsed.sampledUndercut ? [russian ? 'Обнаружено подрезание' : 'Sampled undercut detected'] : []),
  ]
  if (operation === 'rack_profile' || operation === 'brep_rack') return [
    (russian ? 'Шаг · ' : 'Pitch · ') + format(parsed.pitchMm) + ' ' + unit,
    (russian ? 'Ширина · ' : 'Width · ') + format(parsed.widthMm) + ' ' + unit,
  ]
  if (operation === 'finite_external_thread' || operation === 'finite_internal_thread') return [
    (russian ? 'Погрешность стенок ≤ ' : 'Wall deviation ≤ ') + format(parsed.patchErrorUpperMm) + ' ' + unit,
    (russian ? 'Осевое отклонение обрезки ≤ ' : 'Axial trim deviation ≤ ') + format(parsed.axialErrorBandUpperMm) + ' ' + unit,
  ]
  if (operation === 'finite_za_worm' || operation === 'finite_za_wheel') return [
    (russian ? 'Погрешность стенок ≤ ' : 'Wall deviation ≤ ') + format(parsed.wallErrorUpperMm) + ' ' + unit,
  ]
  if (operation === 'verified_bevel_gear') return [(russian ? 'Погрешность ≤ ' : 'Deviation ≤ ') + format(parsed.worldErrorUpperMm) + ' ' + unit]
  if (operation === 'brep_nurbs_gear_span' || operation === 'brep_nurbs_verified_gear') {
    const lines: string[] = []
    if (operation === 'brep_nurbs_verified_gear') {
      lines.push((russian ? 'Профиль ≤ ' : 'Profile ≤ ') + format(parsed.profileErrorUpperMm) + ' ' + unit)
      lines.push((russian ? 'Боковая поверхность ≤ ' : 'Flank surface ≤ ') + format(parsed.idealWallErrorUpperMm) + ' ' + unit)
    }
    if (Array.isArray(parsed.counts)) lines.push((russian ? 'Охват в зубьях · ' : 'Measurable tooth counts · ') + parsed.counts.join(', '))
    if (pair(parsed.nominalWidthMm)) lines.push((russian ? 'Общая нормаль · ' : 'Nominal span · ') + bounds(parsed.nominalWidthMm) + ' ' + unit)
    if (pair(parsed.inferredThinningMm)) lines.push((russian ? 'Утонение зуба · ' : 'Tooth thinning · ') + bounds(parsed.inferredThinningMm) + ' ' + unit)
    return lines
  }
  if (operation === 'surface_trim_range_length') return pair(parsed?.length?.boundsMm)
    ? [(russian ? 'Длина · ' : 'Length · ') + bounds(parsed.length.boundsMm) + ' ' + unit]
    : [russian ? 'Длина неопределённа' : 'Length unresolved']
  if (operation.startsWith('surface_trim_sheet')) {
    if (Array.isArray(parsed?.edgeBindings)) return [(russian ? 'Привязки рёбер · ' : 'Edge bindings · ') + parsed.edgeBindings.length]
    return [russian ? 'Оболочка построена' : 'Sheet constructed']
  }
  if (operation.startsWith('surface_trim_contour_')) {
    if (Array.isArray(parsed?.joins)) return [(russian ? 'Число стыков · ' : 'Joins · ') + parsed.joins.length]
    if (Array.isArray(parsed?.stations)) return [(russian ? 'Число точек деления · ' : 'Division points · ') + parsed.stations.length]
    if (pair(parsed?.parameterBounds)) return ['u ∈ [' + parsed.parameterBounds.map(format).join(' … ') + ']']
    return [String(parsed?.refusalStage ?? parsed?.status ?? '')]
  }
  if (operation === 'surface_trim_patch_area_vector' && Array.isArray(parsed?.boundsMm2) && parsed.boundsMm2.every(pair)) {
    return parsed.boundsMm2.map((v:number[],i:number) => ['X','Y','Z'][i] + ' · ' + bounds(v) + ' ' + unit + '²')
  }
  if (operation.startsWith('surface_trim_')) {
    return pair(parsed?.boundsMm2) ? [(russian ? 'Площадь · ' : 'Area · ') + bounds(parsed.boundsMm2) + ' ' + unit + '²']
      : [russian ? 'Площадь неопределённа' : 'Area unresolved']
  }
  if (operation === 'surface_measure_trimmed_mass') {
    if (!pair(parsed?.massBoundsKg)) return [russian ? 'Масса не определена при заданных допусках и бюджете' : 'Mass unresolved under the requested tolerances and budget']
    const status = parsed.twoPlaneBalance?.status ?? parsed.axisBalance?.status
    const verdict = status === 'within' ? (russian ? 'Балансировка в допуске' : 'Balance within tolerance')
      : status === 'outside' ? (russian ? 'Дисбаланс превышает допуск' : 'Unbalance exceeds tolerance')
      : status === 'undetermined' ? (russian ? 'Балансировка неопределённа' : 'Balance undetermined') : ''
    const axisLines: string[] = []
    if (Array.isArray(parsed.principalAxes)) {
      parsed.principalAxes.forEach((axis: {direction:number[];distanceBoundsMm?:number[];angleSineBounds?:number[]}|null,index:number) => {
        if (!axis || !Array.isArray(axis.direction) || axis.direction.length !== 3 || !axis.direction.every(Number.isFinite)) return
        const label = (russian ? 'Главная ось ' : 'Principal axis ') + (index + 1)
        axisLines.push(label + ' · [' + axis.direction.map(format).join(', ') + ']')
        if (pair(axis.distanceBoundsMm)) axisLines.push((russian ? 'Расстояние · ' : 'Distance · ') + bounds(axis.distanceBoundsMm) + ' ' + unit)
        if (pair(axis.angleSineBounds)) axisLines.push('sin θ · ' + bounds(axis.angleSineBounds))
      })
    }
    return [...axisLines,...(verdict ? [verdict] : []), (russian ? 'Общая масса · ' : 'Total mass · ') + bounds(parsed.massBoundsKg) + (russian ? ' кг' : ' kg')]
  }
  if (operation.startsWith('curve_axis_motion_') && parsed) {
    const status = (key: unknown) => key === 'within' ? (russian ? 'В пределах лимита' : 'Within limit')
      : key === 'exceeds' ? (russian ? 'Лимит превышен' : 'Limit exceeded')
      : (russian ? 'Результат неопределён' : 'Undetermined')
    const lines: string[] = []
    if (Array.isArray(parsed.cells)) {
      const counts = { within: 0, exceeds: 0, undetermined: 0 }
      for (const cell of parsed.cells) {
        const states = Array.isArray(cell?.axisStatuses) ? cell.axisStatuses.flat()
          : Array.isArray(cell?.statuses) ? cell.statuses : [cell?.status]
        if (states.includes('exceeds')) counts.exceeds++
        else if (states.length && states.every((state: string) => state === 'within')) counts.within++
        else counts.undetermined++
      }
      lines.push(russian
        ? `Участки в пределах лимита: ${counts.within}, с превышением: ${counts.exceeds}, неопределённые: ${counts.undetermined}`
        : `Intervals within limit: ${counts.within}, exceeding: ${counts.exceeds}, unresolved: ${counts.undetermined}`)
    }
    if (parsed.status) lines.push(status(parsed.status))
    for (const [key, label] of [
      ['speedBounds', russian ? 'Скорость' : 'Speed'], ['accelerationBounds', russian ? 'Ускорение' : 'Acceleration'],
      ['jerkBounds', russian ? 'Рывок' : 'Jerk'], ['slowdownBounds', russian ? 'Коэффициент замедления' : 'Slowdown factor'],
      ['durationBounds', russian ? 'Длительность в единицах параметра' : 'Duration in parameter units'],
    ] as const) {
      const endpoints = parsed[key]
      if (Array.isArray(endpoints) && endpoints.length === 2 && endpoints.every(x => Number.isFinite(x) || x === 'unbounded')) {
        lines.push(label + ' · ' + endpoints.map(x => x === 'unbounded' ? '∞' : format(x)).join(' … '))
      }
    }
    return lines
  }
  if (operation === 'curve_axial_derivative_peak') {
    if (!pair(parsed?.peakBounds) || ![1, 2, 3].includes(parsed.order)) return []
    const label = (russian ? ['Скорость', 'Ускорение', 'Рывок'] : ['Speed', 'Acceleration', 'Jerk'])[parsed.order - 1]
    const lines = [`${label} · ${bounds(parsed.peakBounds)} ${unit}/Δu^${parsed.order}`]
    if (Array.isArray(parsed.parameterIntervals)) {
      for (const interval of parsed.parameterIntervals) {
        if (pair(interval)) lines.push(`u ∈ [${interval.map(format).join(' … ')}]`)
      }
    }
    return lines
  }
  if (operation === 'curve_directional_travel_full' && pair(parsed?.totalBoundsMm)) return [(russian ? 'Суммарный ход' : 'Total travel') + ' · ' + bounds(parsed.totalBoundsMm) + ' ' + unit]
  if (!['curve_check_backtracking','curve_directional_travel_signed','curve_measure_directional_travel', 'curve_measure_directional_travel_batch'].includes(operation)) return []
  const rows = operation.endsWith('_batch') ? parsed?.intervals : [parsed]
  if (!Array.isArray(rows)) return []
  return rows.flatMap(row => {
    if (!pair(row?.parameterInterval) || !pair(row.forwardBoundsMm) || !pair(row.reverseBoundsMm)) return []
    const status = row.reverseStatus === 'within' ? (russian ? 'в пределах лимита' : 'within limit')
      : row.reverseStatus === 'exceeds' ? (russian ? 'лимит превышен' : 'limit exceeded')
      : row.reverseStatus === 'undetermined' ? (russian ? 'сравнение с лимитом неопределённо' : 'limit comparison inconclusive') : ''
    return [`[${row.parameterInterval.map(format).join(' … ')}] · ${russian ? 'Вперёд' : 'Forward'} ${bounds(row.forwardBoundsMm)} ${unit} · ${russian ? 'Назад' : 'Reverse'} ${bounds(row.reverseBoundsMm)} ${unit}${status ? ' · ' + status : ''}`]
  })
}

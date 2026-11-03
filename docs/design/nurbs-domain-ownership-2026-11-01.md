# Назначения разделов NURBS API — 1 ноября 2026

В `nurbs-api-domains.json` и `nurbs-core/src/domains.rs` зарегистрированы
13 существующих публичных модулей. Алгоритмы и исходные плоские пути API
сохранены; добавлены согласованные представления через `domains`.

| Раздел | Модули | Основание |
| --- | --- | --- |
| geometry | `bernstein_interpolation` | Интервальное преобразование коэффициентов с явным контрактом степени; само по себе не доказывает полиномиальность выражения |
| joining | `profile_section_bridge`, `profile_expression_bernstein` | Двухконцевые Hermite-переходы и коэффициенты корректируемого выражения перехода |
| queries | `polar_surface_differential`, `rational_pole_differential` | Дифференциальные данные исходной рациональной поверхности и её полюсов |
| queries | `surface_boundary_secant`, `surface_weight_profile` | Исходные секущие и представление/оценка весового профиля |
| validation | `profile_projection_jacobian`, `profile_projected_jordan` | Ориентация и инъективность точного факторизованного профиля, без подмены исходных весов |
| validation | `factored_surface_normals`, `synchronized_boundary_normals` | Проверяемые нормальные поля исходных поверхностей и согласованных границ |
| validation | `profile_expression_radius` | Полная проверка закона радиуса корректируемого выражения |
| parameterization | `curve_coordinate_clock` | Строгая монотонность координаты на полном исходном параметрическом интервале |

Проверка `node scripts/map-nurbs-api.mjs --check` теперь покрывает
242 публичных модуля и 200 записей каталога. Это проверка полноты назначения
разделов и актуальности карты; она не расширяет геометрические контракты
модулей и не доказывает общий паритет библиотек Plasticity.


r64: уточнение proposal-окрестностей остаётся у существующего владельца
`surface_offset::coverage::discover`; новый публичный NURBS root не добавлен.
BRep-оркестрация и replay принадлежат `shell_thickness::contact_pair`.
Полюса/топология BRep не получают авторитет из численного predictor.


r65: `surface_offset::boundary_contact` принадлежит существующему разделу
`offsets` через `domains::offsets::surface_offset`. Section/inward и full-face
qualification остаются nested-модулями этого владельца; нового public root
или записи каталога нет (242/200). BRep-агрегация принадлежит
`shell_thickness::contact_pair`; endpoint proofs не заменяют материал/solid gates.


r66: attachment и bridge принадлежат существующему offsets-владельцу
`surface_offset::boundary_contact`; private relation helper принадлежит
`surface_offset::coverage::graph`. Новых public roots/каталожных пунктов нет.
BRep-вызов принадлежит `shell_thickness::contact_pair`, вычисления остаются Rust.


r67: `contact_curve::{chain,lift}` и private coordinate-plane source identity
принадлежат `offsets::surface_offset`; padded nested refinement принадлежит
`offset_contact_tangent` и переиспользуется UV section owner. Новых public
roots/пунктов каталога нет (242/200). BRep fit_path принадлежит
`shell_thickness::contact_pair`; вычисления и evidence остаются в Rust.


r68: closed `chain::region` и degree-refined `contact_curve::lift::fit`
принадлежат существующему `offsets::surface_offset`. Generic winding/simplicity
переиспользуются у исходных trim-владельцев; нового public root/каталожного
пункта нет (242/200). BRep close_path остаётся оркестрацией в Rust.


r69: Jordan nesting forest принадлежит существующему trim_region_audit owner;
compound normal-offset regions — `offsets::surface_offset::contact_curve`.
Новые public roots/пункты каталога не добавлены (242/200). Canonical coedges
не меняют исходные UV definitions; world lift вычисления и replay остаются Rust.

NURBS-0241: `editing::exact_projective` и его закрытый `proof` принадлежат
существующему разделу `editing` (зарегистрированному корневому владельцу
`editing` в `nurbs-api-domains.json`). Нового корневого alias нет. Здесь
находятся ограниченная точная матричная арифметика, общая проективная калибровка
всей контрольной сетки и проверка сохранения исходного базиса; topology/BRep
полномочия модулю не назначены.

NURBS-0242: `editing::exact_derivative` и закрытые `extraction`/`proof`
принадлежат существующему владельцу `editing`. Это построение новой геометрии
поля производной на исходных интервалах, а не изменение точечных differential
queries. Старый API дифференциалов и topology-полномочия не изменяются.

NURBS-0243: `editing::exact_surface_derivative` и его закрытые polynomial,
extraction/storage принадлежат зарегистрированному владельцу `editing`.
Строятся новые поверхности векторных полей частных производных на всех
исходных UV-ячейках; API точечных differential queries и BRep не меняется.

NURBS-0243 admission: закрытый `editing::exact_surface_derivative::content`
принадлежит тому же `editing`. Общий U-/V-множитель сокращается сразу во всех
полиномиальных каналах сетки; это улучшение существующего API, не новый пункт.

NURBS-0244: `editing::exact_surface_derivative::area_normal` принадлежит
существующему владельцу `editing`. Строится ориентированное НЕ единичное поле
`S_u × S_v` на всех исходных UV-ячейках; направления наружу тела, единичная
нормаль и сертификат регулярности этому геометрическому API не назначаются.

NURBS-0245: `editing::exact_surface_derivative::metric` принадлежит `editing`.
Возвращает носитель трёх скалярных полей `[E,F,G]` на всех исходных UV-ячейках;
это построение метрических коэффициентов, не сертификат регулярности/топологии
и не замена точечным differential/variational queries.

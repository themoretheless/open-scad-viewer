# Бикубический патч Hermite: NURBS-0110

`hermite_patch(corners,tangent_u,tangent_v,twist)` строит поверхность степени
3×3 по четырём позициям, первым производным u/v и смешанным uv-производным.
Массивы индексируются [u][v], параметры нормированы на [0,1]². Все данные
имеют единицы длины, поскольку параметры безразмерны.

Угловые условия переводятся алгебраически в 4×4 контрольную сетку Безье
с единичными весами. Аппроксимация по образцам не используется. Binary64
округления не сертифицируются; построение отказывает нечисловым условиям,
недопустимым контролям и некоторым случаям округления ненулевой авторской
касательной или twist к нулю. Это консервативные precision guards,
не универсальная граница ошибки производных. Вырождения разрешены,
регулярность, injectivity, качество панели и solid-топология не проверяются.

Rust: `hermite_patch::patch`; JSON: `surface_hermite_patch`;
TypeScript: `hermiteNurbsPatch`; Rush: `hermite_patch`.
`Surface::Evaluation::second_derivatives` позволяет получить uu/uv/vv
через типизированный native API. Пример: `examples/rush/hermite-patch.r`.

`cargo test --manifest-path crates/nurbs-core/Cargo.toml --features transport hermite_patch`:
2 теста прошли. Независимый бикубический полином проверяет позиции,
касательные и смешанные производные в углах и внутри. Проверены
nonfinite, precision-отказы и допустимое вырождение. Полная проверка,
WASM, экспорт и визуальный просмотр пока не завершены. Статус native.

Полная native-проверка:
`cargo test --manifest-path crates/nurbs-core/Cargo.toml --features transport`:
263 теста и 2 doctest прошли. TypeScript также проверен.
Языковое ядро пересобрано; геометрический WASM ещё ожидает завершения.

Упаковка WASM завершена, размер 9 781 038 байт.
`npx vitest run tests/nurbsHermitePatch.test.ts tests/nurbsClosedSpline.test.ts tests/nurbsClampedSpline.test.ts tests/nurbsNaturalSpline.test.ts tests/nurbsHermiteCurve.test.ts tests/nurbsPrimitiveGraph.test.ts tests/nurbsTranslationSweep.test.ts tests/geometryPackingPublication.test.ts tests/wasmArtifact.test.ts`:
215 тестов в 9 файлах прошли. Проверены геометрические jets, отказы,
46 примеров через JSON, OBJ, PLY и вход редактора. NURBS-0110 переведён
в integrated. Ранние записи об ожидании описывают предыдущее состояние;
визуальная и STEP-квалификация остаются незавершёнными.

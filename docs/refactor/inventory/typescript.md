# Инвентаризация: TypeScript и Svet (`src/`)

Источник: `tools/refactor-audit/audit-ts.mjs` (TypeScript compiler API, без проверки типов). Для `.svt` анализируется блок `<script>`.

Столбцы: `as` это приведения типов, `!` это non-null assertion, `>лимит` это файлы длиннее лимита архитектурного аудита (500 строк для `.ts`, 400 для `.svt`). Клоны считаются от 20 значимых строк.

| Каталог | Файлов | LOC | Fn | max cog | Fn >100 | cy >15 | par >4 | any | as | ! | >лимит | Клоны % | Churn 60д | Приоритет | Статус |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|:--:|---|
| `src/services/(files)` | 422 | 54473 | 2946 | 458 | 48 | 305 | 161 | 25 | 468 | 940 | 6 | 2 | 1440 | P1 | todo |
| `src/mcp/(files)` | 50 | 8356 | 352 | 113 | 6 | 25 | 3 | 0 | 23 | 70 | 4 | 7 | 104 | P1 | todo |
| `src/features/(files)` | 22 | 6356 | 404 | 128 | 1 | 26 | 6 | 0 | 91 | 57 | 1 | 2 | 25 | P2 | todo |
| `src/forma` | 14 | 4812 | 308 | 239 | 7 | 28 | 11 | 263 | 90 | 13 | 0 | 0 | 27 | P1 | todo |
| `src/components` | 44 | 4214 | 170 | 67 | 0 | 6 | 0 | 3 | 48 | 13 | 0 | 0 | 59 | P2 | todo |
| `src/services/geometry` | 37 | 4193 | 507 | 129 | 0 | 3 | 25 | 0 | 24 | 16 | 0 | 0 | 166 | P1 | todo |
| `src/mcp/server` | 16 | 4083 | 89 | 35 | 7 | 2 | 0 | 0 | 9 | 7 | 0 | 79 | 20 | P3 | todo |
| `src/features/directModeler` | 20 | 3992 | 234 | 319 | 12 | 41 | 2 | 1 | 22 | 17 | 0 | 3 | 45 | P1 | todo |
| `src/semantic` | 17 | 3637 | 149 | 142 | 10 | 17 | 11 | 0 | 83 | 6 | 0 | 88 | 22 | P2 | todo |
| `src/core` | 31 | 3288 | 85 | 43 | 1 | 6 | 3 | 0 | 11 | 4 | 0 | 0 | 119 | P2 | todo |
| `src/parser` | 12 | 3210 | 110 | 92 | 3 | 11 | 10 | 0 | 31 | 1 | 0 | 18 | 17 | P2 | todo |
| `src/composables` | 15 | 2941 | 207 | 168 | 11 | 15 | 0 | 2 | 7 | 2 | 0 | 1 | 35 | P2 | todo |
| `src/services/openScadImport` | 8 | 2821 | 131 | 117 | 5 | 15 | 8 | 0 | 5 | 30 | 0 | 62 | 12 | P2 | todo |
| `src/importers` | 10 | 2706 | 106 | 117 | 4 | 10 | 6 | 0 | 4 | 28 | 0 | 80 | 12 | P2 | todo |
| `src/services/openscadSemanticLowerer` | 9 | 2407 | 87 | 265 | 6 | 10 | 9 | 0 | 21 | 6 | 0 | 48 | 17 | P2 | todo |
| `src/services/semanticProgramValidator` | 8 | 2371 | 88 | 517 | 8 | 18 | 6 | 0 | 82 | 13 | 1 | 46 | 13 | P2 | todo |
| `src/mcp/createServer` | 3 | 1979 | 84 | 195 | 1 | 1 | 0 | 0 | 12 | 7 | 1 | 90 | 4 | P3 | todo |
| `src/(root)` | 3 | 1934 | 79 | 28 | 0 | 3 | 0 | 0 | 10 | 1 | 1 | 1 | 14 | P3 | todo |
| `src/renderer` | 8 | 1778 | 44 | 211 | 3 | 6 | 2 | 0 | 2 | 9 | 0 | 0 | 9 | P3 | todo |
| `src/services/sweep` | 21 | 1647 | 111 | 138 | 0 | 8 | 26 | 9 | 11 | 7 | 0 | 0 | 31 | P2 | todo |
| `src/services/photogrammetry` | 11 | 1222 | 62 | 37 | 0 | 2 | 1 | 0 | 7 | 74 | 0 | 0 | 14 | P3 | todo |
| `src/features/MeshModeler` | 5 | 1187 | 126 | 166 | 3 | 6 | 1 | 0 | 8 | 1 | 0 | 17 | 15 | P2 | todo |
| `src/services/mainSolidProtocol` | 6 | 846 | 41 | 937 | 2 | 6 | 0 | 0 | 73 | 29 | 0 | 0 | 16 | P2 | todo |
| `src/services/openScadText` | 3 | 822 | 33 | 80 | 1 | 5 | 0 | 0 | 0 | 4 | 0 | 0 | 6 | P3 | todo |
| `src/services/rushGraphNurbsKernel` | 7 | 789 | 20 | 341 | 2 | 5 | 2 | 1 | 6 | 8 | 0 | 0 | 6 | P3 | todo |
| `src/workers` | 10 | 699 | 32 | 61 | 1 | 4 | 0 | 1 | 12 | 0 | 0 | 0 | 36 | P3 | todo |
| `src/services/solid` | 6 | 677 | 85 | 51 | 1 | 2 | 3 | 0 | 13 | 4 | 0 | 0 | 24 | P3 | todo |
| `src/services/geometryWorkerProtocol` | 2 | 654 | 25 | 119 | 1 | 7 | 0 | 0 | 42 | 1 | 0 | 0 | 3 | P3 | todo |
| `src/services/semanticProgramExecutor` | 2 | 619 | 26 | 47 | 1 | 4 | 1 | 2 | 10 | 0 | 0 | 0 | 3 | P3 | todo |
| `src/mcp/duckdbModelStore` | 2 | 606 | 24 | 52 | 2 | 6 | 0 | 0 | 30 | 0 | 0 | 0 | 3 | P3 | todo |
| `src/services/openScadSurface` | 2 | 606 | 25 | 58 | 1 | 3 | 3 | 0 | 0 | 2 | 0 | 0 | 4 | P3 | todo |
| `src/mcp/directGeometryProtocol` | 2 | 573 | 27 | 63 | 0 | 7 | 0 | 0 | 9 | 1 | 0 | 0 | 2 | P3 | todo |
| `src/services/openScadBuiltinFunctions` | 4 | 569 | 43 | 43 | 0 | 2 | 2 | 0 | 4 | 0 | 0 | 0 | 6 | P3 | todo |
| `src/data` | 2 | 546 | 7 | 7 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 5 | P3 | todo |
| `src/mcp/publicError` | 2 | 512 | 5 | 96 | 1 | 2 | 0 | 0 | 2 | 0 | 0 | 0 | 2 | P3 | todo |
| `src/services/openScadStableExpressionEval` | 2 | 494 | 30 | 205 | 1 | 3 | 0 | 0 | 2 | 1 | 0 | 0 | 4 | P3 | todo |
| `src/services/semanticProgramCodec` | 1 | 343 | 29 | 25 | 0 | 2 | 0 | 0 | 16 | 0 | 0 | 0 | 2 | P3 | todo |
| `src/mcp/officialOpenScadRuntimeService` | 1 | 335 | 18 | 21 | 0 | 1 | 2 | 0 | 0 | 0 | 0 | 0 | 1 | P3 | todo |
| `src/services/shaders` | 3 | 328 | 8 | 3 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 12 | P3 | todo |
| `src/services/commandRegistry` | 1 | 255 | 18 | 26 | 0 | 3 | 0 | 0 | 3 | 0 | 0 | 0 | 1 | P3 | todo |
| `src/mcp/geometryService` | 1 | 253 | 11 | 45 | 0 | 1 | 0 | 0 | 2 | 6 | 0 | 0 | 1 | P3 | todo |
| `src/services/openscadCompiler` | 1 | 204 | 10 | 105 | 1 | 1 | 0 | 0 | 3 | 0 | 0 | 0 | 1 | P3 | todo |
| `src/services/languages` | 1 | 154 | 10 | 42 | 0 | 1 | 0 | 0 | 11 | 2 | 0 | 0 | 7 | P3 | todo |
| `src/i18n` | 1 | 135 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | P3 | todo |
| `src/services/manifoldPlanBackend` | 1 | 117 | 11 | 7 | 0 | 0 | 0 | 0 | 6 | 0 | 0 | 0 | 3 | P3 | todo |
| `src/services/buildCoordinator` | 1 | 89 | 9 | 15 | 0 | 1 | 0 | 0 | 1 | 0 | 0 | 0 | 1 | P3 | todo |
| `src/dev` | 1 | 70 | 4 | 10 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | P3 | todo |
| `src/services/nurbsConstructors` | 1 | 53 | 9 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 1 | P3 | todo |
| `src/services/workspacePersistence` | 1 | 44 | 4 | 7 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | P3 | todo |
| `src/services/loft` | 1 | 38 | 4 | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 1 | P3 | todo |

## Топ-60 функций по когнитивной сложности

| # | Функция | Файл:строка | LOC | cog | cyclo | nest | params |
|---:|---|---|---:|---:|---:|---:|---:|
| 1 | `mainSolidResult` | `src/services/mainSolidProtocol/mainSolidResult.ts:53` | 421 | 937 | 863 | 4 | 2 |
| 2 | `validateOccurrenceProduction` | `src/services/semanticProgramValidator/validateOccurrenceProduction.ts:8` | 641 | 517 | 218 | 7 | 6 |
| 3 | `validFaceContacts` | `src/services/solidFaceContacts.ts:35` | 86 | 458 | 353 | 6 | 3 |
| 4 | `validCoverage` | `src/services/sourceWallCoverageTransport.ts:62` | 120 | 358 | 241 | 7 | 4 |
| 5 | `createOwnNurbsResolver` | `src/services/rushGraphNurbsKernel/createOwnNurbsResolver.ts:30` | 468 | 341 | 233 | 6 | 6 |
| 6 | `evaluate` | `src/services/semanticProgramValidator/validateOccurrenceProduction.ts:108` | 446 | 332 | 161 | 6 | 1 |
| 7 | `useNurbsMatching` | `src/features/directModeler/useNurbsMatching.ts:34` | 164 | 319 | 145 | 7 | 1 |
| 8 | `directDocumentValidation` | `src/services/directModeling.ts:75` | 180 | 294 | 209 | 4 | 2 |
| 9 | `validSelfIntersection` | `src/services/solidSelfIntersection.ts:85` | 89 | 272 | 158 | 7 | 2 |
| 10 | `evalNode` | `src/services/openscadSemanticLowerer/evalNode.ts:17` | 473 | 265 | 167 | 5 | 2 |
| 11 | `evalNode` | `src/services/openscadParser.ts:261` | 461 | 260 | 174 | 4 | 2 |
| 12 | `get` | `src/services/rushGraphNurbsKernel/createOwnNurbsResolver.ts:46` | 450 | 260 | 228 | 5 | 1 |
| 13 | `createRenderer` | `src/forma/svet-renderer.ts:49` | 430 | 239 | 113 | 4 | 1 |
| 14 | `validOffsetContact` | `src/services/mainSolidProtocol/results/offsetContact.ts:5` | 105 | 221 | 181 | 4 | 3 |
| 15 | `useDraftPersistence` | `src/features/directModeler/useDraftPersistence.ts:44` | 381 | 217 | 105 | 6 | 1 |
| 16 | `validMinkowskiResult` | `src/services/mainSolidProtocol/results/minkowski.ts:4` | 56 | 217 | 185 | 6 | 3 |
| 17 | `encodeAndSubmitFrame` | `src/renderer/framePassEncoder/encodeAndSubmitFrame.ts:55` | 363 | 211 | 121 | 6 | 1 |
| 18 | `createComponentOperations` | `src/forma/svet-component.ts:92` | 295 | 207 | 79 | 5 | 1 |
| 19 | `createStableExpressionEvaluators` | `src/services/openScadStableExpressionEval/createStableExpressionEvaluators.ts:10` | 376 | 205 | 99 | 5 | 1 |
| 20 | `useSourceBodyTools` | `src/features/directModeler/useSourceBodyTools.ts:27` | 177 | 202 | 120 | 4 | 1 |
| 21 | `useViewportLayerAndPolygons` | `src/features/directModeler/useViewportLayerAndPolygons.ts:63` | 433 | 195 | 92 | 7 | 1 |
| 22 | `createOpenScadMcpServer` | `src/mcp/createServer/createOpenScadMcpServer.ts:29` | 1305 | 195 | 99 | 5 | 1 |
| 23 | `buildOwnNurbsResolved` | `src/services/rushGraphNurbsKernel/buildOwnNurbsResolved.ts:28` | 98 | 191 | 112 | 3 | 6 |
| 24 | `useFileExchange` | `src/features/directModeler/useFileExchange.ts:70` | 340 | 184 | 93 | 7 | 1 |
| 25 | `createViewportOperations` | `src/features/directModeler/useViewportLayerAndPolygons/viewportOperations.ts:29` | 295 | 183 | 89 | 9 | 1 |
| 26 | `resolveModelingSnap` | `src/services/modelingSnaps.ts:13` | 95 | 176 | 83 | 6 | 3 |
| 27 | `validSourceBody` | `src/services/sourceBody.ts:157` | 85 | 176 | 148 | 5 | 2 |
| 28 | `useMeasurements` | `src/features/directModeler/useMeasurements.ts:23` | 176 | 172 | 101 | 3 | 1 |
| 29 | `createRendererOperations` | `src/composables/useBuildAndRendererLifecycle/rendererOperations.ts:31` | 309 | 168 | 85 | 4 | 1 |
| 30 | `useMeshModeler` | `src/features/MeshModeler/useMeshModeler.ts:60` | 386 | 166 | 87 | 6 | 2 |
| 31 | `createMeshCommands` | `src/features/MeshModeler/useMeshModeler/meshCommands.ts:58` | 315 | 164 | 75 | 5 | 1 |
| 32 | `validMaterial` | `src/services/solidMaterialVolume.ts:30` | 49 | 160 | 122 | 5 | 2 |
| 33 | `validSolidDistance` | `src/services/solidDistance.ts:35` | 47 | 154 | 132 | 4 | 2 |
| 34 | `useCommandPalette` | `src/features/directModeler/useCommandPalette.ts:112` | 179 | 153 | 112 | 5 | 1 |
| 35 | `evaluateSafeMathExpression` | `src/services/cadQuantity.ts:21` | 131 | 149 | 69 | 5 | 1 |
| 36 | `offsetSolidCurve` | `src/services/solidCurveOffset.ts:30` | 70 | 149 | 110 | 5 | 2 |
| 37 | `parseBlockChildren` | `src/forma/parser.ts:218` | 140 | 143 | 67 | 5 | 3 |
| 38 | `validateNodes` | `src/semantic/schemaPrimitives/sameValueType.ts:59` | 185 | 142 | 78 | 5 | 2 |
| 39 | `executeDirectTransaction` | `src/services/directTransactions.ts:29` | 52 | 142 | 64 | 7 | 2 |
| 40 | `validateNodes` | `src/services/semanticProgramValidator/validateNodes.ts:7` | 188 | 142 | 79 | 5 | 2 |
| 41 | `readSweepPatchViewportEvidence` | `src/services/sweep/viewport/boundaryEvidence.ts:83` | 81 | 138 | 137 | 2 | 1 |
| 42 | `mainSolidExpectation` | `src/services/mainSolidProtocol/mainSolidExpectation.ts:39` | 89 | 136 | 105 | 4 | 1 |
| 43 | `validateProfileIntersectionReport` | `src/services/geometry/profileIntersections.ts:29` | 47 | 129 | 99 | 6 | 3 |
| 44 | `down` | `src/features/DirectModeler.svt:1684` | 54 | 128 | 93 | 4 | 5 |
| 45 | `move` | `src/features/DirectModeler.svt:1742` | 109 | 127 | 78 | 4 | 1 |
| 46 | `validateOccurrences` | `src/semantic/occurrenceValidator/validateOccurrences.ts:28` | 198 | 127 | 57 | 4 | 3 |
| 47 | `isOriginalQuadricGraphReport` | `src/services/originalQuadricGraphProtocol.ts:24` | 55 | 127 | 107 | 4 | 1 |
| 48 | `validateOccurrences` | `src/services/semanticProgramValidator/isChildrenExpansionContinuation.ts:117` | 198 | 127 | 57 | 4 | 3 |
| 49 | `useAdvancedPreview` | `src/features/directModeler/useAdvancedPreview.ts:58` | 71 | 126 | 67 | 5 | 1 |
| 50 | `isOriginalPatchContactsReport` | `src/services/originalPatchContactsProtocol.ts:25` | 45 | 125 | 103 | 4 | 1 |
| 51 | `(callback)` | `src/features/DirectModeler.svt:705` | 78 | 120 | 56 | 5 | 0 |
| 52 | `isGeometryWorkerEvent` | `src/services/geometryWorkerProtocol/isGeometryEvaluationResultPayload.ts:183` | 106 | 119 | 122 | 3 | 1 |
| 53 | `useWorkspacePersistenceLifecycle` | `src/composables/useWorkspacePersistenceLifecycle.ts:54` | 338 | 117 | 63 | 4 | 1 |
| 54 | `useDiagnostics` | `src/features/directModeler/useDiagnostics.ts:17` | 110 | 117 | 65 | 4 | 1 |
| 55 | `parseXml` | `src/importers/xmlAndArchive/XmlElement.ts:75` | 188 | 117 | 57 | 4 | 2 |
| 56 | `parseXml` | `src/services/openScadImport/parseXml.ts:6` | 188 | 117 | 57 | 4 | 2 |
| 57 | `sceneMatches` | `src/services/brepDiagnosticProtocol.ts:127` | 82 | 114 | 92 | 5 | 3 |
| 58 | `decodeBinary` | `src/services/valueBinaryCodec.ts:169` | 115 | 114 | 50 | 5 | 2 |
| 59 | `McpManifoldPlanQualificationSupervisor.evaluate` | `src/mcp/manifoldPlanQualificationSupervisor.ts:179` | 304 | 113 | 43 | 4 | 3 |
| 60 | `validateExecution` | `src/services/semanticProgramValidator/validateResult.ts:75` | 186 | 113 | 60 | 4 | 7 |

## Модули, недостижимые ни из одной точки входа (43 файлов)

Не импортируются ни приложением (`src/main.ts`), ни MCP-сервером (`src/mcp/server.ts`), ни воркерами, ни скриптами, ни тестами. Учитываются статические и динамические импорты, реэкспорты, `new URL(..., import.meta.url)` и `import()`-типы.

| Файл | LOC | Состояние в git |
|---|---:|---|
| `src/semantic/lowererExpressionEval.ts` | 500 | в индексе, без изменений |
| `src/importers/types.ts` | 488 | в индексе, без изменений |
| `src/mcp/server/wireMappers.ts` | 380 | в индексе, без изменений |
| `src/mcp/server/schemas/meshExecutionSchemaForManifest.ts` | 376 | в индексе, без изменений |
| `src/mcp/server/schemas/idSchema.ts` | 365 | в индексе, без изменений |
| `src/mcp/server/registerTools/save_model.ts` | 353 | в индексе, без изменений |
| `src/semantic/lowererGeometryOps/IDENTITY_MATRIX.ts` | 353 | в индексе, без изменений |
| `src/mcp/server/schemas/independentExportInputSchema.ts` | 350 | в индексе, без изменений |
| `src/importers/xmlAndArchive/decodeOpenScadAmfText.ts` | 345 | в индексе, без изменений |
| `src/mcp/server/registerTools/official_export.ts` | 343 | в индексе, без изменений |
| `src/importers/xmlAndArchive/inflateRaw.ts` | 342 | в индексе, без изменений |
| `src/importers/xmlAndArchive/XmlElement.ts` | 332 | в индексе, без изменений |
| `src/semantic/schemaPrimitives/SemanticProgramValidationError.ts` | 331 | в индексе, без изменений |
| `src/importers/xmlAndArchive/requiredAttributeNs.ts` | 321 | в индексе, без изменений |
| `src/mcp/server/registerTools/customize_model.ts` | 321 | в индексе, без изменений |
| `src/mcp/server/registerTools/check.ts` | 317 | в индексе, без изменений |
| `src/importers/dxfImporter/Matrix2.ts` | 314 | в индексе, без изменений |
| `src/semantic/schemaPrimitives/sameValueType.ts` | 308 | в индексе, без изменений |
| `src/importers/dxfImporter/resolvePreparedDxfQueryMetadata.ts` | 302 | в индексе, без изменений |
| `src/mcp/server/registerResourcesAndPrompts/review_model.ts` | 299 | в индексе, без изменений |
| `src/semantic/occurrenceValidator/validateOccurrences.ts` | 292 | в индексе, без изменений |
| `src/mcp/server/registerResourcesAndPrompts/geometry_parity.ts` | 271 | в индексе, без изменений |
| `src/semantic/lowererGeometryOps/makePolyhedron.ts` | 269 | в индексе, без изменений |
| `src/importers/stlOffImporter.ts` | 251 | в индексе, без изменений |
| `src/semantic/occurrenceValidator/SemanticModuleActivationAnchor.ts` | 233 | в индексе, без изменений |
| `src/mcp/server/registerResourcesAndPrompts/independent_export_artifact.ts` | 194 | в индексе, без изменений |
| `src/mcp/server/registerTools.ts` | 169 | в индексе, без изменений |
| `src/mcp/server/officialWireMappers.ts` | 147 | в индексе, без изменений |
| `src/components/AssemblyAnimationControls.svt` | 114 | в индексе, без изменений |
| `src/semantic/occurrenceValidator/validateResult.ts` | 82 | в индексе, без изменений |
| `src/mcp/server/registerResourcesAndPrompts.ts` | 69 | в индексе, без изменений |
| `src/services/untrustedParser.ts` | 40 | в индексе, без изменений |
| `src/services/sweep/construction/correction.ts` | 14 | в индексе, без изменений |
| `src/workers/untrustedParser.worker.ts` | 11 | в индексе, без изменений |
| `src/importers/xmlAndArchive.ts` | 6 | в индексе, без изменений |
| `src/services/sceneUniformCache.ts` | 6 | в индексе, без изменений |
| `src/importers/dxfImporter.ts` | 5 | в индексе, без изменений |
| `src/mcp/officialRuntimeCacheRoot.ts` | 5 | в индексе, без изменений |
| `src/semantic/occurrenceValidator.ts` | 5 | в индексе, без изменений |
| `src/forma/index.ts` | 4 | в индексе, без изменений |
| `src/mcp/server/schemas.ts` | 4 | в индексе, без изменений |
| `src/semantic/lowererGeometryOps.ts` | 4 | в индексе, без изменений |
| `src/semantic/schemaPrimitives.ts` | 3 | в индексе, без изменений |

## Модули, достижимые только из тестов (43 файлов)

Продакшен-код их не использует. Либо это заготовки, либо мёртвый код, который держат живым тесты.

| Файл | LOC | Состояние в git |
|---|---:|---|
| `src/services/gpuTrussSolver.ts` | 490 | в индексе, без изменений |
| `src/services/gpuImageTrace.ts` | 485 | в индексе, без изменений |
| `src/mcp/manifoldPlanQualificationSupervisor.ts` | 483 | в индексе, без изменений |
| `src/semantic/lowererBuilder/SemanticProgramBuilder.ts` | 467 | в индексе, без изменений |
| `src/services/gpuBvhRaycast.ts` | 464 | в индексе, без изменений |
| `src/services/manifoldPlanQualificationWorkerLane.ts` | 373 | в индексе, без изменений |
| `src/services/gpuSubdivision.ts` | 369 | в индексе, без изменений |
| `src/services/manifoldPlanQualificationProtocol.ts` | 345 | в индексе, без изменений |
| `src/services/mainSolidWorkerPool.ts` | 308 | в индексе, без изменений |
| `src/services/frameAnalysis.ts` | 303 | в индексе, без изменений |
| `src/semantic/lowererBuilder/countStatements.ts` | 275 | в индексе, без изменений |
| `src/semantic/lowererBuilder/MAX_SHAPES.ts` | 270 | в индексе, без изменений |
| `src/services/gpuGcodePreview.ts` | 248 | в индексе, без изменений |
| `src/services/cadIges.ts` | 225 | в индексе, без изменений |
| `src/semantic/lowererBuilder/registerOperations.ts` | 215 | в индексе, без изменений |
| `src/services/gpuSubdivisionReference.ts` | 198 | в индексе, без изменений |
| `src/services/geometry/intersections.ts` | 164 | в индексе, без изменений |
| `src/services/manifoldPlanQualificationWorkerRuntime.ts` | 147 | в индексе, без изменений |
| `src/mcp/server/contracts.ts` | 125 | в индексе, без изменений |
| `src/services/cadStepExternalResolver.ts` | 118 | в индексе, без изменений |
| `src/services/untrustedParserWorker.ts` | 108 | в индексе, без изменений |
| `src/services/virtualFileSystem.ts` | 83 | в индексе, без изменений |
| `src/services/structuralDiagnostics.ts` | 56 | в индексе, без изменений |
| `src/mcp/rushGraphHttp.ts` | 53 | в индексе, без изменений |
| `src/services/geometry/displayPolicy.ts` | 40 | в индексе, без изменений |
| `src/services/sweep/certificates/nurbsFilledCapError.ts` | 32 | в индексе, без изменений |
| `src/workers/manifoldPlanQualification.worker.ts` | 29 | в индексе, без изменений |
| `src/services/buildSurfaceInspection.ts` | 22 | в индексе, без изменений |
| `src/services/untrustedParserWorkerRuntime.ts` | 20 | в индексе, без изменений |
| `src/services/sweep/construction/nurbsCircleSectionRepair.ts` | 17 | в индексе, без изменений |
| `src/mcp/manifoldPlanQualification.worker.ts` | 16 | в индексе, без изменений |
| `src/services/openscadNativeGeometry.ts` | 16 | в индексе, без изменений |
| `src/services/geometry/polygonBridge.ts` | 15 | в индексе, без изменений |
| `src/mcp/manifoldPlanQualification.worker.mjs` | 14 | в индексе, без изменений |
| `src/services/sweep/certificates/nurbsErrorComposition.ts` | 8 | в индексе, без изменений |
| `src/core/qualityTargets.ts` | 6 | в индексе, без изменений |
| `src/semantic/lowererBuilder.ts` | 6 | в индексе, без изменений |
| `src/services/latticeDecimation.ts` | 5 | в индексе, без изменений |
| `src/services/photogrammetry/reconstruction.ts` | 4 | в индексе, без изменений |
| `src/services/miterStationSmoothness.ts` | 2 | в индексе, без изменений |
| `src/services/nurbsCircleSectionRepair.ts` | 2 | в индексе, без изменений |
| `src/services/nurbsErrorComposition.ts` | 2 | в индексе, без изменений |
| `src/services/nurbsFilledCapError.ts` | 2 | в индексе, без изменений |

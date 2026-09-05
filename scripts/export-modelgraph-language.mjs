// Run: node --import tsx scripts/export-modelgraph-language.mjs
import { writeFileSync } from 'node:fs'
import { z } from 'zod/v4'
import { modelGraphSchema, MODELGRAPH_GUIDE, MODELGRAPH_EXAMPLE, MODELGRAPH_FUNCTIONAL_GUIDE, MODELGRAPH_FUNCTIONAL_EXAMPLE, MODELGRAPH_UNITS_GUIDE, MODELGRAPH_UNITS_EXAMPLE, MODELGRAPH_SKETCH_GUIDE, MODELGRAPH_SKETCH_EXAMPLE, MODELGRAPH_ASSEMBLY_EXAMPLE, MODELGRAPH_LOFT_EXAMPLE } from '../src/services/modelGraph.ts'
const json = value => JSON.stringify(value, null, 2) + '\n'
writeFileSync(new URL('../docs/languages/modelgraph-1.schema.json', import.meta.url), json(z.toJSONSchema(modelGraphSchema)))
writeFileSync(new URL('../docs/languages/modelgraph-1.example.json', import.meta.url), json(MODELGRAPH_EXAMPLE))
writeFileSync(new URL('../docs/languages/modelgraph-1.functional.example.json', import.meta.url), json(MODELGRAPH_FUNCTIONAL_EXAMPLE))
writeFileSync(new URL('../docs/languages/modelgraph-1.units.example.json', import.meta.url), json(MODELGRAPH_UNITS_EXAMPLE))
writeFileSync(new URL('../docs/languages/modelgraph-1.sketch.example.json', import.meta.url), json(MODELGRAPH_SKETCH_EXAMPLE))
writeFileSync(new URL('../docs/languages/modelgraph-1.loft.example.json', import.meta.url), json(MODELGRAPH_LOFT_EXAMPLE))
writeFileSync(new URL('../docs/languages/modelgraph-1.assembly.example.json', import.meta.url), json(MODELGRAPH_ASSEMBLY_EXAMPLE))
writeFileSync(new URL('../docs/languages/modelgraph-1-prompt.md', import.meta.url), `# ModelGraph/1 — инструкция для модели

Создавай параметрические 3D-модели в JSON по приложенной схеме. Используй чистые функции и именованные параметры для повторно используемых деталей. Не вставляй программный код в строки.

${MODELGRAPH_GUIDE}

## Functional programming

${MODELGRAPH_FUNCTIONAL_GUIDE}

## Пример: функция детали и массив экземпляров

\`\`\`json
${json(MODELGRAPH_FUNCTIONAL_EXAMPLE)}\`\`\`

## Types, units and declared constraints

${MODELGRAPH_UNITS_GUIDE}

Пример с единицами и именованным ограничением доступен в modelgraph-1.units.example.json и поле units_example MCP-ресурса.

## Sketch constraint solver

${MODELGRAPH_SKETCH_GUIDE}

Example: modelgraph-1.sketch.example.json, also exposed as sketch_example in the MCP language resource.

## MCP workflow

1. Read openscad://language/modelgraph-1 for the current schema, guide and examples.
2. Call modelgraph_compile with document. This validates and evaluates expressions but does not build geometry.
3. Call modelgraph_check with document to build actual geometry and inspect measurements.
4. For assemblies, call modelgraph_interference to inspect current-pose volume overlap; unknown is not a clearance result.
5. To change parameters, call modelgraph_set_parameters with document, expected_document_sha256 and updates:[{id,value}]. Keep the returned document and validate geometry again.

The caller owns document storage. The hash checks the supplied document, not concurrent external storage. Generated source can be passed to existing OpenSCAD export tools. Error paths for evaluated instances and source_map.instance_path locate function calls and repetitions; they do not identify stable CAD faces. Runtime geometry errors may still refer to generated SCAD.

The browser editor still accepts OpenSCAD. Geometry is computed by Manifold; this language does not introduce B-rep, CUDA or guarantees of printability. Closures may return geometry values, which evaluate nodes insert into the geometric graph. No pattern matching or static type inference.
`)

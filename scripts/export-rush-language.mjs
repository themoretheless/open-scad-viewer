import { RUSH_FRONTEND_GUIDE } from '../src/services/rushFrontend.ts'
import { MECHANICAL_GENERATOR_EXAMPLES } from '../src/services/mechanicalGeneratorContract.ts'
// Run: node --import tsx scripts/export-rush-language.mjs
import { writeFileSync } from 'node:fs'
import { z } from 'zod/v4'
import { rushGraphSchema, RUSH_GRAPH_GUIDE, RUSH_GRAPH_EXAMPLE, RUSH_GRAPH_FUNCTIONAL_GUIDE, RUSH_GRAPH_FUNCTIONAL_EXAMPLE, RUSH_GRAPH_UNITS_GUIDE, RUSH_GRAPH_UNITS_EXAMPLE, RUSH_GRAPH_SKETCH_GUIDE, RUSH_GRAPH_SKETCH_EXAMPLE, RUSH_GRAPH_ASSEMBLY_EXAMPLE, RUSH_GRAPH_LOFT_EXAMPLE } from '../src/services/rushGraph.ts'
for (const [kind,document] of Object.entries(MECHANICAL_GENERATOR_EXAMPLES)) writeFileSync(new URL(`../docs/languages/rush-1.${kind}.example.json`,import.meta.url),JSON.stringify(document,null,2)+'\n')
const json = value => JSON.stringify(value, null, 2) + '\n'
writeFileSync(new URL('../docs/languages/rush-1.schema.json', import.meta.url), json(z.toJSONSchema(rushGraphSchema)))
writeFileSync(new URL('../docs/languages/rush-1.example.json', import.meta.url), json(RUSH_GRAPH_EXAMPLE))
writeFileSync(new URL('../docs/languages/rush-1.functional.example.json', import.meta.url), json(RUSH_GRAPH_FUNCTIONAL_EXAMPLE))
writeFileSync(new URL('../docs/languages/rush-1.units.example.json', import.meta.url), json(RUSH_GRAPH_UNITS_EXAMPLE))
writeFileSync(new URL('../docs/languages/rush-1.sketch.example.json', import.meta.url), json(RUSH_GRAPH_SKETCH_EXAMPLE))
writeFileSync(new URL('../docs/languages/rush-1.loft.example.json', import.meta.url), json(RUSH_GRAPH_LOFT_EXAMPLE))
writeFileSync(new URL('../docs/languages/rush-1.assembly.example.json', import.meta.url), json(RUSH_GRAPH_ASSEMBLY_EXAMPLE))
writeFileSync(new URL('../docs/languages/rush-1-prompt.md', import.meta.url), `# Rush IR/1 — инструкция для модели

Создавай параметрические 3D-модели в JSON по приложенной схеме. Используй чистые функции и именованные параметры для повторно используемых деталей. Не вставляй программный код в строки.

${RUSH_GRAPH_GUIDE}

## Compact text syntax

${RUSH_FRONTEND_GUIDE}

## Functional programming

${RUSH_GRAPH_FUNCTIONAL_GUIDE}

## Пример: функция детали и массив экземпляров

\`\`\`json
${json(RUSH_GRAPH_FUNCTIONAL_EXAMPLE)}\`\`\`

## Types, units and declared constraints

${RUSH_GRAPH_UNITS_GUIDE}

Пример с единицами и именованным ограничением доступен в rush-1.units.example.json и поле units_example MCP-ресурса.

## Sketch constraint solver

${RUSH_GRAPH_SKETCH_GUIDE}

Example: rush-1.sketch.example.json, also exposed as sketch_example in the MCP language resource.

## MCP workflow

1. Read openscad://language/rush-1 for the current schema, guide and examples.
2. Call rush_compile with document. This validates and evaluates expressions but does not build geometry.
3. Call rush_check with document to build actual geometry and inspect measurements.
4. For assemblies, call rush_interference to inspect current-pose volume overlap; unknown is not a clearance result.
5. To change parameters, call rush_set_parameters with document, expected_document_sha256 and updates:[{id,value}]. Keep the returned document and validate geometry again.

The caller owns document storage. The hash checks the supplied document, not concurrent external storage. Generated source can be passed to existing OpenSCAD export tools. Error paths for evaluated instances and source_map.instance_path locate function calls and repetitions; they do not identify stable CAD faces. Runtime geometry errors may still refer to generated SCAD.

The browser editor also accepts OpenSCAD. Rush parsing and evaluation run in Rust/WASM with the repository-owned geometry kernel. Closures and match expressions may return geometry values. Structural pattern matching follows the compact text guide; it does not provide static exhaustiveness checking or guarantee printability.
`)

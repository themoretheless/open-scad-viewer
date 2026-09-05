import { cpSync, mkdirSync, writeFileSync, existsSync } from 'node:fs'
import { resolve, dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
const project = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const destination = resolve(process.argv[2] ?? join(project, '.local-integrations'))
const target = join(destination, 'modelgraph')
if (existsSync(target)) throw new Error(`Destination already exists: ${target}. Choose a new output directory.`)
mkdirSync(destination, { recursive: true })
cpSync(join(project, 'integrations/modelgraph'), target, { recursive: true })
const references = join(target, 'skills/modelgraph/references')
mkdirSync(references, { recursive: true })
for (const name of ['modelgraph-1-prompt.md', 'modelgraph-1.schema.json', 'modelgraph-1.example.json', 'modelgraph-1.functional.example.json', 'modelgraph-1.units.example.json', 'modelgraph-1.sketch.example.json', 'modelgraph-1.assembly.example.json', 'modelgraph-1.loft.example.json', 'modelgraph-nurbs-1-prompt.md', 'modelgraph-nurbs-1.schema.json', 'modelgraph-nurbs-1.example.json', 'modelgraph-nurbs-1.surface.example.json']) cpSync(join(project, 'docs/languages', name), join(references, name))
const entry = { command: process.execPath, args: [join(target, 'launch.mjs')], env: { MODELGRAPH_PROJECT_ROOT: project } }
const json = (name, value) => writeFileSync(join(destination, name), JSON.stringify(value, null, 2) + '\n')
json('modelgraph/.mcp.json', { mcpServers: { modelgraph: entry } })
for (const client of ['claude-desktop', 'claude-code', 'cursor']) json(`${client}.json`, { mcpServers: { modelgraph: entry } })
json('vscode.json', { servers: { modelgraph: { type: 'stdio', ...entry } } })
// JSON quoted strings/arrays are also valid TOML basic strings/arrays here.
writeFileSync(join(destination, 'codex.toml'), `[mcp_servers.modelgraph]\ncommand = ${JSON.stringify(entry.command)}\nargs = ${JSON.stringify(entry.args)}\n\n[mcp_servers.modelgraph.env]\nMODELGRAPH_PROJECT_ROOT = ${JSON.stringify(project)}\n`)
console.log(`ModelGraph local plugin and client snippets: ${destination}`)

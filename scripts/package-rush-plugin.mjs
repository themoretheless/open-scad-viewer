import { cpSync, mkdirSync, writeFileSync, existsSync } from 'node:fs'
import { resolve, dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
const project = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const destination = resolve(process.argv[2] ?? join(project, '.local-integrations'))
const target = join(destination, 'rush')
if (existsSync(target)) throw new Error(`Destination already exists: ${target}. Choose a new output directory.`)
mkdirSync(destination, { recursive: true })
cpSync(join(project, 'integrations/rush'), target, { recursive: true })
const references = join(target, 'skills/rush/references')
mkdirSync(references, { recursive: true })
for (const name of ['rush-1-prompt.md', 'rush-1.schema.json', 'rush-1.example.json', 'rush-1.functional.example.json', 'rush-1.units.example.json', 'rush-1.sketch.example.json', 'rush-1.assembly.example.json', 'rush-1.loft.example.json', 'rush-nurbs-1-prompt.md', 'rush-nurbs-1.schema.json', 'rush-nurbs-1.example.json', 'rush-nurbs-1.surface.example.json']) cpSync(join(project, 'docs/languages', name), join(references, name))
const entry = { command: process.execPath, args: [join(target, 'launch.mjs')], env: { RUSH_GRAPH_PROJECT_ROOT: project } }
const json = (name, value) => writeFileSync(join(destination, name), JSON.stringify(value, null, 2) + '\n')
json('rush/.mcp.json', { mcpServers: { rush: entry } })
for (const client of ['claude-desktop', 'claude-code', 'cursor']) json(`${client}.json`, { mcpServers: { rush: entry } })
json('vscode.json', { servers: { rush: { type: 'stdio', ...entry } } })
// JSON quoted strings/arrays are also valid TOML basic strings/arrays here.
writeFileSync(join(destination, 'codex.toml'), `[mcp_servers.rush]\ncommand = ${JSON.stringify(entry.command)}\nargs = ${JSON.stringify(entry.args)}\n\n[mcp_servers.rush.env]\nRUSH_GRAPH_PROJECT_ROOT = ${JSON.stringify(project)}\n`)
console.log(`RushGraph local plugin and client snippets: ${destination}`)

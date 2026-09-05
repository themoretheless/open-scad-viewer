import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { resolve, isAbsolute } from 'node:path'
const root = process.env.MODELGRAPH_PROJECT_ROOT
if (!root || !isAbsolute(root) || !existsSync(resolve(root, 'src/mcp/server.ts')) || !existsSync(resolve(root, 'node_modules/tsx/package.json'))) {
  console.error('ModelGraph requires MODELGRAPH_PROJECT_ROOT pointing to an installed open-scad-viewer checkout. Run npm ci there, then regenerate the client configurations.')
  process.exit(1)
}
// Each client gets an isolated ephemeral catalog, avoiding DuckDB writer conflicts.
const child = spawn(process.execPath, ['--import', 'tsx', resolve(root, 'src/mcp/server.ts'), '--memory'], { cwd: root, stdio: 'inherit', shell: false })
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal))
child.on('error', error => { console.error('ModelGraph startup failed:', error.message); process.exitCode = 1 })
child.on('exit', code => { process.exitCode = code ?? 1 })

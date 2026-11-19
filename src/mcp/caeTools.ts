import type { McpServer } from '@modelcontextprotocol/server'
import { registerCaeGuideResource } from './caeGuide'
import { registerCaePrintedSolidAnalyzeTool } from './caePrintedSolidTools'
import { registerCaeStructuralTools } from './caeStructuralTools'

export function registerCaeTools(server: McpServer): void {
  registerCaeGuideResource(server)
  registerCaeStructuralTools(server)
  registerCaePrintedSolidAnalyzeTool(server)
}

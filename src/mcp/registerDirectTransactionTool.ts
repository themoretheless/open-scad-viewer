import type {McpServer} from '@modelcontextprotocol/server'
import {z} from 'zod/v4'
import {toolOutputSchema} from './createServer/officialToolInputSchema'
import {textResult,errorResult} from './createServer/persistedFailureStatus'
/** The action batch owns an isolated worker and returns only an atomic result. */
export function registerDirectTransactionTool(server:McpServer) {
 server.registerTool('openscad_direct_transaction',{
  title:'Apply an atomic direct-model action batch',
  description:'Execute a version 1 JSON action script (1–80 actions) on a direct document. Returns a complete document only if every action succeeds. Does not modify a live UI or saved model.',
  inputSchema:z.object({document:z.string().max(64*1024*1024),script:z.string().max(16*1024*1024)}).strict(),
  outputSchema:toolOutputSchema(z.object({document:z.string()})),
  annotations:{readOnlyHint:true,destructiveHint:false,idempotentHint:true,openWorldHint:false},
 },async({document,script},context)=>{
  try{
   const {runDirectTransaction}=await import('./directTransaction')
   return textResult({document:await runDirectTransaction(document,script,context.mcpReq.signal)})
  }catch(error){return errorResult(error)}
 })
}

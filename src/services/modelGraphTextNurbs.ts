/** Rust resolves text expressions, validates the NURBS graph and prepares numeric fields. */
import { hashNurbsDocument, ModelGraphNurbsError } from './modelGraphNurbsCompiler'
import type { ModelGraphNurbsCompilation } from './modelGraphNurbs'
import { prepareGraphRust } from './languages/kernel'
export function compileTextNurbs(nodes: Record<string,unknown>[],parameters: Record<string,unknown>[],root:string): ModelGraphNurbsCompilation {
 const result=prepareGraphRust<Omit<ModelGraphNurbsCompilation,'document_sha256'>>('textNurbs',{nodes,parameters,root})
 if(!result.ok)throw new ModelGraphNurbsError(result.error.code,result.error.path,result.error.message)
 return {...result.value,document_sha256:hashNurbsDocument(result.value.document)}
}

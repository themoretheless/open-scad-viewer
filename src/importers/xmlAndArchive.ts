// Public entry point; implementations retain their original contracts.
export { parseXml, childElements, childElement, descendants, childElementsNs, childElementNs, descendantsNs, attribute, attributeNs } from './xmlAndArchive/XmlElement'
export type { XmlElement } from './xmlAndArchive/XmlElement'
export { requiredAttributeNs, requiredAttribute, numericAttribute, numericText, unitScale, IDENTITY_4, multiply4, appendRawMesh, parseOpenScadAmfXml, parseOpenScadAmfWithBudget } from './xmlAndArchive/requiredAttributeNs'
export type { Matrix4, RawMesh3D, Rotation4Builder } from './xmlAndArchive/requiredAttributeNs'
export { decodeOpenScadAmfText, parseOpenScad3mfWithBudget, parseOpenScad3mf } from './xmlAndArchive/decodeOpenScadAmfText'

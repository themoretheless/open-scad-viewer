import test from 'node:test';
import assert from 'node:assert/strict';
import {checkRetainedAxisAudit} from '../scripts/retained-axis-audit-contract.mjs';
// Synthetic captured-report mutations test admission, not geometry or G2 itself.
const fixture=()=>{
 const sharp={certified:false,exactIdentity:false,regularityCertified:false,work:3,reason:'strip-jet-identity-different'};
 const smooth={certified:true,exactIdentity:true,regularityCertified:true,work:7,reason:null};
 const g2={exactG1G2Certified:false,exactWork:10,certifiedOrder:null,unresolvedSeams:[0],seams:[sharp,smooth]};
 return {axis:{extractionComplete:true,unclassifiedFaces:[],unpairedEdges:[],edgeIds:[10,11],exactWork:10,maxWork:20},g2};
};
const check=({axis,g2})=>checkRetainedAxisAudit(axis,g2,null,false,false);
test('sharp aggregate preserves independently proved later G2 seam',()=>check(fixture()));
test('old early refusal placeholder cannot qualify a complete retained audit',()=>{
 const f=fixture();Object.assign(f.g2.seams[1],{certified:false,exactIdentity:false,regularityCertified:false,work:0,reason:'seam-audit-skipped-after-refusal'});f.g2.unresolvedSeams=[0,1];f.g2.exactWork=f.axis.exactWork=3;
 assert.throws(()=>check(f),/sharp first seam/);
});
test('all owned seams need outcomes even when whole axis is C0',()=>{
 const f=fixture();f.g2.seams.pop();assert.throws(()=>check(f),/Every owned retained seam/);
});
test('unresolved coverage cannot hide an unproved local seam',()=>{
 const f=fixture();f.g2.unresolvedSeams=[];assert.throws(()=>check(f),/Unresolved retained seam coverage/);
});
test('positive local seam requires exact identity and regularity',()=>{
 for(const field of ['exactIdentity','regularityCertified']){
  const f=fixture();f.g2.seams[1][field]=false;assert.throws(()=>check(f),/Positive retained seam/);
 }
});

// Two ASCII digits encode 13 bits. Exclude JS string escapes and delimiters.
export const alphabet=Array.from({length:94},(_,i)=>String.fromCharCode(i+33)).filter(c=>!"'\\`".includes(c)).join('')
export function encodeBase91(bytes){
 if(bytes.length>0xffffffff)throw Error('Base91 size overflow')
 const parts=['b91:',bytes.length.toString(16).padStart(8,'0')]
 let bits=0,value=0
 for(const byte of bytes){value|=byte<<bits;bits+=8;if(bits>=13){const word=value&8191;parts.push(alphabet[word%91],alphabet[Math.floor(word/91)]);value>>>=13;bits-=13}}
 if(bits)parts.push(alphabet[value%91],alphabet[Math.floor(value/91)])
 return parts.join('')
}

// A backtick is safe inside the single-quoted generated literal.
export const alphabet92=Array.from({length:94},(_,i)=>String.fromCharCode(i+33)).filter(c=>!"'\\".includes(c)).join('')
export const encodeVariableBase91=bytes=>encodeVariable(bytes,alphabet,'b9v:')
export const encodeVariableBase92=bytes=>encodeVariable(bytes,alphabet92,'b92:')
// Literal tabs are valid in a single-quoted JS string and retained by esbuild.
export const encodeVariableBase94=bytes=>encodeVariable(bytes,'\t '+alphabet92,'b94:')
// SOH is a legal one-byte JS string character; emission is independently verified.
export const encodeVariableBase95=bytes=>encodeVariable(bytes,'\x01\t '+alphabet92,'b95:')
export const encodeVariableBase108=bytes=>encodeVariable(bytes,String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+'\t '+alphabet92,'bAx:')
// Additional single-byte literal digits; exclude JS line breaks and escapes.
export const encodeVariableBase118=bytes=>encodeVariable(bytes,String.fromCharCode(22,23,24,25,26,28,29,30,31,127)+String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+'\t '+alphabet92,'bBx:')
export const encodeVariableBase93=bytes=>encodeVariable(bytes,' '+alphabet92,'b93:')
function encodeVariable(bytes,digits,tag){
 if(bytes.length>0xffffffff)throw Error('Base91 size overflow')
 const base=digits.length,threshold=base*base-8193
 const parts=[tag,bytes.length.toString(16).padStart(8,'0')]
 let bits=0,value=0
 for(const byte of bytes){
  value|=byte<<bits;bits+=8
  if(bits>13){let word=value&8191;const width=word>threshold?13:14;if(width===14)word=value&16383;parts.push(digits[word%base],digits[Math.floor(word/base)]);value>>>=width;bits-=width}
 }
 if(bits)parts.push(digits[value%base],digits[Math.floor(value/base)])
 return parts.join('')
}

const rank=new Int16Array(128).fill(-1)
for(let i=0;i<alphabet.length;i++)rank[alphabet.charCodeAt(i)]=i
const rank92=new Int16Array(128).fill(-1)
const digits92=Array.from({length:94},(_,i)=>String.fromCharCode(i+33)).filter(c=>!"'\\".includes(c)).join('')
for(let i=0;i<digits92.length;i++)rank92[digits92.charCodeAt(i)]=i
const rank93=new Int16Array(128).fill(-1)
const digits93=' '+digits92
for(let i=0;i<digits93.length;i++)rank93[digits93.charCodeAt(i)]=i
const rank94=new Int16Array(128).fill(-1)
const digits94='\t'+digits93
for(let i=0;i<digits94.length;i++)rank94[digits94.charCodeAt(i)]=i
const rank95=new Int16Array(128).fill(-1)
const digits95='\x01'+digits94
for(let i=0;i<digits95.length;i++)rank95[digits95.charCodeAt(i)]=i
const rank108=new Int16Array(128).fill(-1)
const digits108=String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+digits94
for(let i=0;i<digits108.length;i++)rank108[digits108.charCodeAt(i)]=i
const rank118=new Int16Array(128).fill(-1)
const digits118=String.fromCharCode(22,23,24,25,26,28,29,30,31,127)+digits108
for(let i=0;i<digits118.length;i++)rank118[digits118.charCodeAt(i)]=i
export function decodeBase91(text,maximum=4*1024*1024+4){
 const dense=text.startsWith('bBx:'),extended=text.startsWith('bAx:'),control=text.startsWith('b95:'),tabbed=text.startsWith('b94:'),widest=text.startsWith('b93:'),wide=text.startsWith('b92:'),base=dense?118:extended?108:control?95:tabbed?94:widest?93:wide?92:91,threshold=base*base-8193
 const digits=dense?rank118:extended?rank108:control?rank95:tabbed?rank94:widest?rank93:wide?rank92:rank,variable=dense||extended||control||tabbed||widest||wide||text.startsWith('b9v:')
 if(!/^b(?:91|9v|92|93|94|95|Ax|Bx):[0-9a-f]{8}/.test(text)||text.length>12+2*Math.ceil(maximum*8/13))throw Error('Base91 size limit')
 const size=parseInt(text.slice(4,12),16)
 if(size>maximum||(text.length-12)%2!==0||(!variable&&text.length!==12+2*Math.ceil(size*8/13)))throw Error('Base91 size mismatch')
 const out=new Uint8Array(size)
 let bits=0,value=0,offset=0
 for(let i=12;i<text.length;i+=2){
  const a=digits[text.charCodeAt(i)]??-1,b=digits[text.charCodeAt(i+1)]??-1,word=a+base*b
  if(a<0||b<0||(!variable&&word>8191)||offset===size)throw Error('Invalid base91 word')
  value|=word<<bits;bits+=variable&&((word&8191)<=threshold)?14:13
  while(bits>=8&&offset<size){out[offset++]=value&255;value>>>=8;bits-=8}
 }
 if(offset!==size||value!==0)throw Error('Noncanonical base91 padding')
 return out
}

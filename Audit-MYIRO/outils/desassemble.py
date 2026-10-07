"""Analyse PE/Capstone statique, sans chargement Windows des DLL examinées."""
from pathlib import Path
import sys,json,hashlib,argparse,re
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'outils/python-libs'))
import pefile,capstone
TARGETS={
 'fdx-x86':'MY-CT1/FDXSDK.dll',
 'fdx-x64-101':'Ergosoft/FDXSDK.dll',
 'fdx-x64-103':'EIZO/plugins/exports/win.x86_64/FDXSDK.dll',
 'myct1':'MY-CT1/SpectrophotometerConfigurationToolMY-CT1.exe',
 'eizo-x64':'EIZO/plugins/exports/win.x86_64/libmeasurementdevice_x64.dll',
 'eizo-x86':'EIZO/plugins/exports/win.x86_64/libmeasurementdevice.dll',
 'fd9-x86':'FD-S2w/Module/FD9SDK.dll',
 'fd9-x64':'Ergosoft/FD9SDK.dll',
 'fds2w':'FD-S2w/Module/FD-S2w.exe',
}
class Binary:
 def __init__(self,key):
  self.key=key;self.path=ROOT/'collecte'/TARGETS[key];self.pe=pefile.PE(str(self.path));self.base=self.pe.OPTIONAL_HEADER.ImageBase
  self.bits=64 if self.pe.FILE_HEADER.Machine==0x8664 else 32
  self.md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_64 if self.bits==64 else capstone.CS_MODE_32)
  self.mem=self.pe.get_memory_mapped_image();self.exports={self.base+x.address:x.name.decode() for x in getattr(getattr(self.pe,'DIRECTORY_ENTRY_EXPORT',None),'symbols',[]) if x.name}
  self.imports={i.address:d.dll.decode()+'!'+(i.name.decode() if i.name else '#'+str(i.ordinal)) for d in getattr(self.pe,'DIRECTORY_ENTRY_IMPORT',[]) for i in d.imports}
  self.ranges=[(self.base+e.struct.BeginAddress,self.base+e.struct.EndAddress) for e in getattr(self.pe,'DIRECTORY_ENTRY_EXCEPTION',[])]
 def string(self,va):
  off=va-self.base
  if not 0<=off<len(self.mem):return ''
  b=self.mem[off:off+180]; m=re.match(rb'[\x20-\x7e]{5,}\0',b)
  if m:return repr(m.group()[:-1].decode())
  m=re.match(rb'(?:[\x20-\x7e]\0){5,}\0\0',b)
  return repr(m.group()[:-2].decode('utf-16le')) if m else ''
 def line(self,i):
  addr,size,mn,op=i; notes=[]
  direct=re.fullmatch('0x([0-9a-f]+)',op)
  if direct:
   v=int(direct[1],16)
   if v in self.exports:notes.append(self.exports[v])
  vals=[]
  for m in re.finditer(r'\[rip ([+-]) (0x[0-9a-f]+)\]',op): vals.append(addr+size+int(m[2],16)*(1 if m[1]=='+' else -1))
  vals.extend(int(v,16) for v in re.findall(r'(?<![a-z])0x[0-9a-f]+',op) if self.base<=int(v,16)<self.base+len(self.mem))
  for v in dict.fromkeys(vals):
   if v in self.imports:notes.append(self.imports[v])
   s=self.string(v)
   if s:notes.append(hex(v)+' '+s)
  return f'{addr:016x}  {mn:8} {op}'+(' ; '+' | '.join(notes) if notes else '')
 def cfg(self,start,limit=3000):
  todo=[start];seen={};truncated=False
  while todo:
   pc=todo.pop()
   if pc in seen:continue
   if len(seen)>=limit:truncated=True;break
   off=pc-self.base
   if not 0<=off<len(self.mem):continue
   for i in self.md.disasm_lite(self.mem[off:off+20000],pc):
    a,size,mn,op=i
    if a in seen:break
    seen[a]=i
    if len(seen)>=limit:truncated=True;break
    if mn.startswith('ret') or mn in ['int3','ud2']:break
    if mn.startswith('j'):
     m=re.fullmatch(r'0x([0-9a-f]+)',op)
     if m:
      target=int(m[1],16)
      if target==start or target not in self.exports:todo.append(target)
     if mn=='jmp':break
  return [self.line(seen[k]) for k in sorted(seen)],truncated
 def dump(self,out):
  out.mkdir(parents=True,exist_ok=True)
  meta={'source':str(self.path),'sha256':hashlib.sha256(self.path.read_bytes()).hexdigest(),'image_base':hex(self.base),'bits':self.bits,'capstone':capstone.__version__,'exports':{hex(k):v for k,v in self.exports.items()},'imports':{hex(k):v for k,v in self.imports.items()},'runtime_functions':[(hex(a),hex(b)) for a,b in self.ranges]}
  (out/'metadata.json').write_text(json.dumps(meta,indent=2),encoding='utf-8')
  if self.key.startswith(('fdx','fd9')):
   exp=out/'exports';exp.mkdir(exist_ok=True)
   for va,name in self.exports.items():
    if name.lower().startswith(('fdx_jig','jig_')):continue
    lines,cut=self.cfg(va)
    (exp/(name+'.asm.txt')).write_text(f'; {self.key} VA={va:#x} RVA={va-self.base:#x} instruction_limit_reached={cut}\n; Parcours des branches directes seulement : tables de saut, exceptions et appels indirects non suivis.\n'+'\n'.join(lines)+'\n',encoding='utf-8')
  else:
   self.md.skipdata=True
   with (out/'code.asm.txt').open('w',encoding='utf-8') as f:
    for s in self.pe.sections:
     if s.Characteristics & 0x20000000:
      for i in self.md.disasm_lite(s.get_data(),self.base+s.VirtualAddress):f.write(self.line(i)+'\n')
  print(self.key,'OK',flush=True)
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('keys',nargs='*');parser.add_argument('--address');parser.add_argument('--end');parser.add_argument('--limit',type=int,default=3000);args=parser.parse_args()
 for key in args.keys or TARGETS:
  b=Binary(key)
  if args.address and args.end:
   start=int(args.address,0);end=int(args.end,0)
   print('; Balayage lineaire : peut inclure des tables de donnees ; verifier les chemins de controle.')
   print('\n'.join(b.line(i) for i in b.md.disasm_lite(b.mem[start-b.base:end-b.base],start)))
  elif args.address:
   lines,cut=b.cfg(int(args.address,0),args.limit);print('; Branches directes seulement : tables de saut et exceptions non suivies.');print('\n'.join(lines));print('instruction_limit_reached=',cut)
  else:b.dump(ROOT/'retroanalyse'/key)

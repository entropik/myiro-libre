"""Reconstitue les index de preuves et controle les binaires sans les executer."""
from pathlib import Path
import json,re,struct,hashlib,csv,ctypes
from desassemble import Binary,ROOT,TARGETS
OUT=ROOT/'retroanalyse'
proofs={};checks=[]
manifest=json.loads((ROOT/'analyse/manifest.json').read_text(encoding='utf-8'))
by_path={str((ROOT/x['copie']).resolve()).lower():x['sha256'] for x in manifest}
for key,rel in TARGETS.items():
 p=ROOT/'collecte'/rel
 actual=hashlib.sha256(p.read_bytes()).hexdigest()
 assert actual==by_path[str(p.resolve()).lower()],str(p)
 checks.append({'controle':'empreinte source '+key,'resultat':'conforme','sha256':actual})
rows=[]
for p in sorted((OUT/'fdx-x86/exports').glob('*.asm.txt')):
 text=p.read_text(encoding='utf-8')
 ret=sorted(set(int(x,16) if x else 0 for x in re.findall(r'\bret\s*(0x[0-9a-f]+|[0-9]+)?\s*$',text,re.M)))
 rows.append({'fonction':p.name.split('.')[0],'octets_depilement_x86':','.join(map(str,ret)),'preuve':str(p.relative_to(OUT))})
with (OUT/'arguments-x86.csv').open('w',encoding='utf-8-sig',newline='') as f:
 w=csv.DictWriter(f,fieldnames=rows[0].keys(),delimiter=';');w.writeheader();w.writerows(rows)
expected={'FDX_GetSDKVersion':4,'FDX_GetDevicePortList':12,'FDX_Connect':8,'FDX_Disconnect':0,'FDX_Calibration':4,'FDX_GetMeasureCondition':4,'FDX_SetMeasureCondition':4,'FDX_StartMeasurement':0,'FDX_StopMeasurement':0,'FDX_GetMeasureData':20,'FDX_GetRAWData':8,'FDX_RegisterDeviceEventHandler':4,'FDX_GetDeviceInfo':4,'FDX_GetError':0,'FDX_CancelScanMeasurement':0}
actual={r['fonction']:r['octets_depilement_x86'] for r in rows}
for name,n in expected.items():
 assert actual[name]==str(n),(name,actual[name])
checks.append({'controle':'depilement x86 des 15 interfaces de l en-tete','resultat':'conforme'})
b=Binary('fdx-x86')
proofs['dispatch_data_type_x86']={str(i):hex(v) for i,v in enumerate(struct.unpack('<14I',b.pe.get_data(0x1e068,56)))}
b64=Binary('eizo-x64')
proofs['dispatch_event_eizo_x64']={str(i):hex(b64.base+v) for i,v in enumerate(struct.unpack('<10I',b64.pe.get_data(0x42154,40)))}
mapping=[];name=None;resolved=False
for i in b64.md.disasm_lite(b64.pe.get_data(0x415ea,0x400),b64.base+0x415ea):
 line=b64.line(i);addr,size,mn,op=i
 m=re.search(r"'(FDX_[A-Za-z0-9_]+)'",line)
 if m:name=m[1];resolved=False
 if name and 'KERNEL32.dll!GetProcAddress' in line:resolved=True
 if name and resolved and mn=='mov':
  m=re.fullmatch(r'qword ptr \[rip \+ (0x[0-9a-f]+)\], rax',op)
  if m:
   slot=addr+size+int(m[1],16)
   mapping.append({'fonction':name,'slot':hex(slot),'offset_objet':hex(slot-0x1800eb400),'instruction_store':hex(addr)})
   name=None;resolved=False
proofs['resolutions_dynamiques_eizo_x64']=mapping
assert next(x for x in mapping if x['fonction']=='FDX_GetMeasureData')['offset_objet']=='0x68'
assert next(x for x in mapping if x['fonction']=='FDX_RegisterDeviceEventHandler')['offset_objet']=='0x30'
checks.append({'controle':'table dynamique EIZO GetMeasureData et callback','resultat':'conforme'})
# Extraits localisables, bornes par VA/RVA, sans reinterpreter les tables comme code.
regions={
 'eizo-x64':[(0x1800415ea,0x180041940,'chargement-dynamique'),(0x180041d80,0x180041e18,'attente-mesure'),(0x180041e20,0x180042154,'callback'),(0x180042180,0x180042230,'enregistrement-callback'),(0x180042270,0x180042369,'calibration'),(0x180042370,0x1800423a6,'grille-spectrale'),(0x1800423b0,0x1800425d0,'mesure-spectrale'),(0x180042830,0x1800428d1,'enumeration'),(0x180042b79,0x180042baa,'connexion'),(0x180042f50,0x180043119,'mesure-xyz')],
 'eizo-x86':[(0x1003a67f,0x1003a757,'mesure-spectrale'),(0x1003a2c5,0x1003a2e0,'callback-ret')],
 'myct1':[(0x402592,0x4025c5,'connexion'),(0x402650,0x4026ed,'infos-instrument'),(0x402740,0x40281e,'enumeration-deux-passes')],
 'fdx-x86':[(0x1001ca83,0x1001cb04,'conditions-retour-switch'),(0x10020550,0x100208cd,'spectre-reflexion-36')],
}
for key,ranges in regions.items():
 b=Binary(key);dest=OUT/'preuves'/key;dest.mkdir(parents=True,exist_ok=True)
 for start,end,label in ranges:
  listing=[b.line(i) for i in b.md.disasm_lite(b.mem[start-b.base:end-b.base],start)]
  (dest/(label+'.asm.txt')).write_text('; Extrait lineaire : verifier les branches et les tables.\n'+'\n'.join(listing)+'\n',encoding='utf-8')
class Buffer32(ctypes.Structure):_fields_=[('address',ctypes.c_uint32),('capacity',ctypes.c_uint32)]
class Buffer64(ctypes.Structure):_fields_=[('address',ctypes.c_uint64),('capacity',ctypes.c_uint32)]
assert ctypes.sizeof(Buffer32)==8 and Buffer32.capacity.offset==4
assert ctypes.sizeof(Buffer64)==16 and Buffer64.capacity.offset==8
checks.append({'controle':'representation des buffers 32/64 bits avec ctypes, sans chargement SDK','resultat':'conforme','note':'Ne constitue pas une compilation du fichier .h.'})
checks.append({'controle':'GetMeasureData type 10 pointe vers branche spectrale','resultat':'conforme' if proofs['dispatch_data_type_x86']['10']=='0x1001d70c' else 'ECHEC'})
assert all(x['resultat']=='conforme' for x in checks)
(OUT/'preuves.json').write_text(json.dumps(proofs,ensure_ascii=False,indent=2),encoding='utf-8')
(OUT/'verification-statique.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps({'sources_verifiees':len(TARGETS),'resolutions_dynamiques':len(mapping),'exports_x86_indexes':len(rows),'controles':len(checks)},ensure_ascii=False))

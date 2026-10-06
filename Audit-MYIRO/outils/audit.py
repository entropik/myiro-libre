from pathlib import Path
import sys, json, hashlib, shutil, re, csv
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'outils/python-libs'))
import pefile
errors=[]; manifest=[]; analyses=[]
def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def copy(p,group,rel=None):
    dst=ROOT/'collecte'/group/(rel or p.name)
    dst.parent.mkdir(parents=True,exist_ok=True)
    if dst.exists() and digest(dst)!=digest(p): raise RuntimeError('Collision '+str(dst))
    shutil.copy2(p,dst)
    h=digest(p)
    if digest(dst)!=h: raise RuntimeError('Hash mismatch '+str(p))
    manifest.append(dict(source=str(p),copie=str(dst.relative_to(ROOT)),octets=p.stat().st_size,sha256=h))
    return dst
def tree(src,group):
    base=Path(src)
    for p in base.rglob('*'):
        if p.is_file():
            try: copy(p,group,p.relative_to(base))
            except Exception as e: errors.append(str(e))
def dec(x): return x.decode('utf-8','replace') if isinstance(x,bytes) else str(x)
def inspect(p):
    pe=pefile.PE(str(p))
    imports={dec(d.dll):[dec(i.name) if i.name else '#'+str(i.ordinal) for i in d.imports] for d in getattr(pe,'DIRECTORY_ENTRY_IMPORT',[])}
    exports=[dict(nom=dec(s.name) if s.name else None,ordinal=s.ordinal,rva=hex(s.address),forwarder=dec(s.forwarder) if s.forwarder else None) for s in getattr(getattr(pe,'DIRECTORY_ENTRY_EXPORT',None),'symbols',[])]
    versions={}
    for group in getattr(pe,'FileInfo',[]):
        for block in group:
            for table in getattr(block,'StringTable',[]): versions.update({dec(k):dec(v) for k,v in table.entries.items()})
    pdb=[]
    for d in getattr(pe,'DIRECTORY_ENTRY_DEBUG',[]):
        if d.struct.Type==2:
            b=pe.get_data(d.struct.AddressOfRawData,d.struct.SizeOfData)
            pdb.append(dec(b[24:].split(b'\0')[0]) if b[:4]==b'RSDS' else b.hex())
    result=dict(fichier=str(p),architecture={0x14c:'x86',0x8664:'x64'}.get(pe.FILE_HEADER.Machine,hex(pe.FILE_HEADER.Machine)),version=versions,exports=exports,imports=imports,pdb=pdb,dotnet=bool(pe.OPTIONAL_HEADER.DATA_DIRECTORY[14].VirtualAddress),sections=[dec(s.Name.rstrip(b'\0')) for s in pe.sections])
    pe.close(); return result
tree('C:/Program Files (x86)/Configuration Tool MY-CT1','MY-CT1')
tree('C:/Program Files (x86)/KONICA MINOLTA/FD-S2w','FD-S2w')
tree('C:/ProgramData/MYIRO','MYIRO-ProgramData')
for base,group in [(Path('C:/Program Files/Ergosoft 16'),'Ergosoft'),(Path('C:/ProgramData/EIZO/ColorNavigator 7'),'EIZO')]:
    for p in base.rglob('*'):
        if p.suffix.lower() not in ['.dll','.exe']: continue
        try:
            info=inspect(p)
            relevant=p.name.lower() in ['fdxsdk.dll','fd9sdk.dll'] or any(k.lower() in ['fdxsdk.dll','fd9sdk.dll'] for k in info['imports'])
            if not relevant:
                relevant=b'FDX_Connect' in p.read_bytes()
            if relevant: copy(p,group,p.relative_to(base))
        except pefile.PEFormatError: pass
        except Exception as e: errors.append(str(p)+': '+str(e))
print('Copies',len(manifest),flush=True)
for row in manifest:
    p=ROOT/row['copie']
    if p.suffix.lower() not in ['.dll','.exe']: continue
    try:
        info=inspect(p); info['fichier']=row['copie']; analyses.append(info)
        important=p.name.lower() in ['fdxsdk.dll','fd9sdk.dll','libfxapi.dll','libclapi.dll','libcalcolor.dll','dicolor.dll','fd9barcodemanager.dll'] or 'FDXSDK.dll' in info['imports'] or 'FD9SDK.dll' in info['imports'] or p.name in ['FD-S2w.exe','SpectrophotometerConfigurationToolMY-CT1.exe']
        if important:
            out=ROOT/'analyse'/p.relative_to(ROOT/'collecte'); out.parent.mkdir(parents=True,exist_ok=True)
            out.with_suffix(out.suffix+'.json').write_text(json.dumps(info,ensure_ascii=False,indent=2),encoding='utf-8')
            data=p.read_bytes(); strings=[]
            for pattern,encoding in [(rb'[\x20-\x7e]{5,}','ascii'),(rb'(?:[\x20-\x7e]\x00){5,}','utf-16le')]:
                for m in re.finditer(pattern,data): strings.append((m.start(),encoding,m.group().decode(encoding)))
            strings.sort()
            out.with_suffix(out.suffix+'.strings.tsv').write_text('offset\tencodage\ttexte\n'+'\n'.join(f'{o:08x}\t{e}\t{s}' for o,e,s in strings),encoding='utf-8')
    except Exception as e: errors.append(str(p)+': '+str(e))
(ROOT/'analyse').mkdir(exist_ok=True)
for name,data in [('manifest',manifest),('binaires',analyses),('erreurs',errors)]: (ROOT/'analyse'/f'{name}.json').write_text(json.dumps(data,indent=2,ensure_ascii=False),encoding='utf-8')
with (ROOT/'analyse/inventaire.csv').open('w',newline='',encoding='utf-8-sig') as f:
    w=csv.DictWriter(f,fieldnames=['source','copie','octets','sha256'],delimiter=';'); w.writeheader(); w.writerows(manifest)
print(json.dumps(dict(fichiers=len(manifest),octets=sum(x['octets'] for x in manifest),binaires=len(analyses),erreurs=errors),ensure_ascii=False))
for a in analyses:
    if a['exports'] and ('SDK' in Path(a['fichier']).name or Path(a['fichier']).name.startswith(('libfx','libcl','libcal','DIColor'))): print(a['fichier'],a['architecture'],a['version'].get('FileVersion'),len(a['exports']),a['pdb'])

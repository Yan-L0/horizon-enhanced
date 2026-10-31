from pathlib import Path
import zipfile,re,json,hashlib
source=Path('<USER_HOME>/Downloads/BSL_v10.1.8.zip');out=Path('outputs/horizon-render-fixes');out.mkdir(exist_ok=True)
profiles={'Balanced':{'shadowMapResolution':'1024','shadowDistance':'128.0','CLOUD_THICKNESS':'4','BLOOM_RADIUS':'2','FOG_DENSITY_END':'0.37','LIGHT_SHAFT_END_SMOKE':'0.25','FXAA':False,'AO':False},'Quality':{'FOG_DENSITY_END':'0.37','LIGHT_SHAFT_END_SMOKE':'0.25'}}
for label,settings in profiles.items():
 target=out/f'BSL_Horizon_{label}_v10.1.8.zip'
 with zipfile.ZipFile(source) as src,zipfile.ZipFile(target,'w',zipfile.ZIP_DEFLATED) as dst:
  for info in src.infolist():
   data=src.read(info)
   if info.filename=='shaders/lib/settings.glsl':
    text=data.decode()
    for key,value in settings.items():
     if value is False:
      text,n=re.subn(r'(?m)^(\s*)#define '+re.escape(key)+r'\s*(?=\r?$)',r'\1//#define '+key,text,count=1)
     elif key in ('shadowMapResolution','shadowDistance'):
      text,n=re.subn(r'(const (?:int|float) '+key+r'\s*=\s*)[^;]+;',r'\g<1>'+value+';',text,count=1)
     else:
      text,n=re.subn(r'(?m)^(\s*#define '+key+r'\s+)\S+',r'\g<1>'+value,text,count=1)
     assert n==1,(label,key,n)
    data=text.encode()
   dst.writestr(info,data)
 (out/(target.stem+'.json')).write_text(json.dumps({'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'changes':settings,'note':'Private BSL preset; render resolution unchanged. All original rights retained.'},indent=2))
 print(target)

from pathlib import Path
import shutil,plistlib,hashlib,json,difflib
base=Path.cwd();out=base/'outputs/horizon-render-fixes';app=out/'Horizon Enhanced.app';res=app/'Contents/Resources';mac=app/'Contents/MacOS';res.mkdir(parents=True,exist_ok=True);mac.mkdir(parents=True,exist_ok=True)
binary=base/'work/upgrader-port/target/fast/horizon-client';shutil.copy2(binary,mac/'horizon-client')
shutil.copy2('<USER_HOME>/Downloads/BSL_v10.1.8.zip',res/'BSL_v10.1.8.zip')
for name in ('BSL_Horizon_Balanced_v10.1.8.zip','BSL_Horizon_Quality_v10.1.8.zip'):shutil.copy2(out/name,res/name)
shutil.copy2(base/'outputs/upgrader-horizon/upgrader-content-0.1.0.hmod',res/'upgrader-content.hmod')
launch=(base/'work/render-bugs/launch.py').read_text()
(res/'launch.py').write_text(launch)
(mac/'launch').write_text('#!/bin/sh\nexec /usr/bin/python3 "$(dirname "$0")/../Resources/launch.py" "$@"\n');(mac/'launch').chmod(0o755)
with (app/'Contents/Info.plist').open('wb') as f:plistlib.dump({'CFBundleIdentifier':'dev.private.horizon.enhanced','CFBundleName':'Horizon Enhanced','CFBundleDisplayName':'Horizon Enhanced','CFBundleExecutable':'launch','CFBundlePackageType':'APPL','CFBundleVersion':'0.2.0','CFBundleShortVersionString':'0.2.0','NSHighResolutionCapable':True},f)
root=base/'work/upgrader-port/horizon-1da354b67a509812c4141b67de2affd6de65c342';before=base/'work/render-upgrade/before';patch=[]
paths = {p.relative_to(before): p for p in before.rglob('*.rs')}
extra = base/'work/render-bugs/before'
for p in extra.rglob('*.rs'):
 rel=p.relative_to(extra)
 if rel not in paths:paths[rel]=p
for rel,path in sorted(paths.items()):
 patch.extend(difflib.unified_diff(path.read_text().splitlines(True),(root/rel).read_text().splitlines(True),fromfile='a/'+str(rel),tofile='b/'+str(rel)))
new = Path('crates/horizon-client-render/src/gfx/pack_overlay.rs')
patch.extend(difflib.unified_diff([], (root/new).read_text().splitlines(True), fromfile='/dev/null', tofile='b/'+str(new)))
(out/'renderer.patch').write_text(''.join(patch))
metadata={'base_horizon':'1da354b67a509812c4141b67de2affd6de65c342','build':'experimental-private-0.2.0','platform':'macOS arm64','files':{str(p.relative_to(out)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [mac/'horizon-client',res/'BSL_v10.1.8.zip',res/'launch.py',out/'renderer.patch']}}
(out/'BUILD.json').write_text(json.dumps(metadata,indent=2))
for name in ['HORIZON-LICENSE.txt','THIRD_PARTY_NOTICES.md','ORIGINAL-NOTICE.txt']:
 shutil.copy2(base/'outputs/upgrader-horizon'/name,out/name)
print(app)

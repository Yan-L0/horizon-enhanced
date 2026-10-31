from pathlib import Path
import shutil,plistlib,hashlib,json,difflib
base=Path.cwd();out=base/'outputs/horizon-render-upgrade';app=out/'Horizon Enhanced.app';res=app/'Contents/Resources';mac=app/'Contents/MacOS';res.mkdir(parents=True,exist_ok=True);mac.mkdir(parents=True,exist_ok=True)
binary=base/'work/upgrader-port/target/fast/horizon-client';shutil.copy2(binary,mac/'horizon-client')
shutil.copy2('<USER_HOME>/Downloads/BSL_v10.1.8.zip',res/'BSL_v10.1.8.zip')
shutil.copy2(base/'outputs/upgrader-horizon/upgrader-content-0.1.0.hmod',res/'upgrader-content.hmod')
launch=(base/'work/render-upgrade/launch_fixed.py').read_text()
(res/'launch.py').write_text(launch)
(mac/'launch').write_text('#!/bin/sh\nexec /usr/bin/python3 "$(dirname "$0")/../Resources/launch.py" "$@"\n');(mac/'launch').chmod(0o755)
with (app/'Contents/Info.plist').open('wb') as f:plistlib.dump({'CFBundleIdentifier':'dev.private.horizon.enhanced','CFBundleName':'Horizon Enhanced','CFBundleDisplayName':'Horizon Enhanced','CFBundleExecutable':'launch','CFBundlePackageType':'APPL','CFBundleVersion':'0.1.1','CFBundleShortVersionString':'0.1.1','NSHighResolutionCapable':True},f)
root=base/'work/upgrader-port/horizon-1da354b67a509812c4141b67de2affd6de65c342';before=base/'work/render-upgrade/before';patch=[]
for path in sorted(before.rglob('*.rs')):
 rel=path.relative_to(before);patch.extend(difflib.unified_diff(path.read_text().splitlines(True),(root/rel).read_text().splitlines(True),fromfile='a/'+str(rel),tofile='b/'+str(rel)))
(out/'renderer.patch').write_text(''.join(patch))
metadata={'base_horizon':'1da354b67a509812c4141b67de2affd6de65c342','build':'experimental-private-0.1.1','platform':'macOS arm64','files':{str(p.relative_to(out)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [mac/'horizon-client',res/'BSL_v10.1.8.zip',res/'launch.py',out/'renderer.patch']}}
(out/'BUILD.json').write_text(json.dumps(metadata,indent=2))
for name in ['HORIZON-LICENSE.txt','THIRD_PARTY_NOTICES.md','ORIGINAL-NOTICE.txt']:
 shutil.copy2(base/'outputs/upgrader-horizon'/name,out/name)
print(app)

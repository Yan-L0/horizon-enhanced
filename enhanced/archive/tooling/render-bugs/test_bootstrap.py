import os,tempfile,subprocess,json
from pathlib import Path
base=Path.cwd();app=base/'outputs/horizon-render-fixes/Horizon Enhanced.app';root=Path(tempfile.mkdtemp(prefix='horizon-bootstrap-',dir='/tmp'));home=root/'game';home.mkdir();(home/'saves').mkdir();(home/'saves/keep.txt').write_text('preserve me')
graphics=home/'horizon-graphics.txt';graphics.write_text('pickedShaderPack:BSL_v10.1.8.zip\nresolutionScale:87\nhdr:false\n')
options=home/'options.txt';options.write_text('renderDistance:10\nmaxFps:75\n');before=options.read_bytes()
env=os.environ.copy();env.update(HORIZON_HOME=str(home),HORIZON_LAUNCHER_HOME=str(root/'launcher'),XDG_DATA_HOME=str(root/'data'),XDG_CONFIG_HOME=str(root/'config'),HORIZON_LAUNCHER_NO_SELF_UPDATE='1',HORIZON_NO_AUDIO='1',HORIZON_NO_ERROR_DIALOG='1')
def run():subprocess.run([str(app/'Contents/MacOS/launch'),'--launch-api'],env=env,check=True,capture_output=True)
run();assert 'pickedShaderPack:BSL_Horizon_Quality_v10.1.8.zip' in graphics.read_text();assert 'resolutionScale:87' in graphics.read_text();assert options.read_bytes()==before
assert (home/'saves/keep.txt').read_text()=='preserve me';graphics.write_text(graphics.read_text().replace('BSL_Horizon_Quality_v10.1.8.zip','BSL_Horizon_Balanced_v10.1.8.zip'))
run();assert 'pickedShaderPack:BSL_Horizon_Balanced_v10.1.8.zip' in graphics.read_text();assert options.read_bytes()==before
result={'migration_once':True,'world_sentinel_preserved':True,'render_scale_preserved':True,'other_options_preserved':True,'quality_choice_preserved_on_second_launch':True}
(base/'outputs/horizon-render-fixes/bootstrap-test.json').write_text(json.dumps(result,indent=2));print(json.dumps(result))

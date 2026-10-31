import os,subprocess,tempfile,json,time
from pathlib import Path
root=Path(tempfile.mkdtemp(prefix='horizon-bsl-',dir='/tmp'))
base=Path.cwd();pack=Path('<USER_HOME>/Downloads/BSL_v10.1.8.zip')
env=os.environ.copy()
for key,sub in [('HORIZON_HOME','game'),('HORIZON_LAUNCHER_HOME','launcher'),('XDG_DATA_HOME','data'),('XDG_CONFIG_HOME','config')]:
 (root/sub).mkdir();env[key]=str(root/sub)
env.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1',HORIZON_NO_AUDIO='1',HORIZON_SHADER_PACK=str(pack))
exe=base/'work/upgrader-port/target/fast/horizon-client'
resources=Path.home()/'Library/Application Support/horizon-launcher/versions/nightly-2026-10-30-1da354b/.local/horizon-client/26.3'
sock=str(root/'ctl.sock');log=open(root/'game.log','w')
proc=subprocess.Popen([str(exe),'--resources',str(resources),'--game-dir',str(root/'game'),'--home',str(root/'game'),'--world','ShaderTest','--creative','--test-terrain','--seed','123','--render-distance','4','--offscreen','--width','640','--height','360','--control-socket',sock,'--name','ShaderTester'],env=env,cwd=root,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
state={'root':str(root),'pid':proc.pid,'socket':sock}
(base/'work/render-upgrade/test-state.json').write_text(json.dumps(state));print(json.dumps(state))

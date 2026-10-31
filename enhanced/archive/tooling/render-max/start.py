import os,subprocess,tempfile,json,sys
from pathlib import Path
base=Path.cwd();root=Path(tempfile.mkdtemp(prefix='horizon-max-',dir='/tmp'))
env=os.environ.copy()
for k,v in [('HORIZON_HOME','game'),('HORIZON_LAUNCHER_HOME','launcher'),('XDG_DATA_HOME','data'),('XDG_CONFIG_HOME','config')]:
 (root/v).mkdir();env[k]=str(root/v)
env.update(HORIZON_NO_AUDIO='1',HORIZON_LAUNCHER_NO_SELF_UPDATE='1',HORIZON_SHADER_PACK_TIMING='1')
env['HORIZON_SHADER_PACK']=str(Path(sys.argv[1] if len(sys.argv)>1 else '<USER_HOME>/Downloads/BSL_v10.1.8.zip').resolve())
import shutil
world=Path(json.loads(Path('work/render-max/world-snapshot.json').read_text())['path'])
shutil.copytree(world,root/'game/saves/RenderBugs')
(root/'game/options.txt').write_text('maxFps:260\ninactivityFpsLimit:minimized\nenableVsync:false\npauseOnLostFocus:false\nchatVisibility:2\n')
(root/'game/horizon-graphics.txt').write_text('resolutionScale:75\n')
exe=Path(sys.argv[2]).resolve() if len(sys.argv)>2 else base/'outputs/horizon-render-upgrade/Horizon Enhanced.app/Contents/MacOS/horizon-client'
resources=Path.home()/'Library/Application Support/horizon-launcher/versions/nightly-2026-10-30-1da354b/.local/horizon-client/26.3'
sock=root/'ctl.sock';log=open(root/'process.log','w')
args=[str(exe),'--resources',str(resources),'--game-dir',str(root/'game'),'--home',str(root/'game'),'--world','RenderBugs','--creative','--render-distance','6','--offscreen','--frames','100000','--timeout-seconds','600','--width','1710','--height','977','--control-socket',str(sock),'--name','RendererTester']
proc=subprocess.Popen(args,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
c={'root':str(root),'socket':str(sock),'pid':proc.pid,'pack':env['HORIZON_SHADER_PACK'],'exe':str(exe)};Path('work/render-max/test-state.json').write_text(json.dumps(c));print(json.dumps(c))

#!/usr/bin/env python3
"""Measure fixed cave/surface cameras on copies of a local fixture world.

Run before/after binaries sequentially; do not run builds or GPU tests concurrently.
Requires a fixture containing the chosen coordinates. Same inputs/settings for both.
Raw output contains machine/world information and belongs outside the Git checkout.
"""
from pathlib import Path
import json,subprocess,os,shutil,tempfile,time,sys
import argparse
ap=argparse.ArgumentParser(description="Isolated BSL renderer comparison; never edits the input world.")
ap.add_argument('--client',type=Path,required=True)
ap.add_argument('--ctl',type=Path,required=True)
ap.add_argument('--resources',type=Path,required=True)
ap.add_argument('--world',type=Path,required=True,help='Existing disposable fixture world; copied before use')
ap.add_argument('--pack',type=Path,required=True)
ap.add_argument('--output',type=Path,required=True,help='Keep raw captures/reports outside Git')
ap.add_argument('--scale',type=int,default=75)
ap.add_argument('--width',type=int,default=1710)
ap.add_argument('--height',type=int,default=977)
a=ap.parse_args()
exe=a.client.resolve();ctl=a.ctl.resolve()
for path in [exe,ctl,a.resources,a.world,a.pack]:
 if not path.exists(): ap.error('Missing input: '+str(path))
output=a.output.resolve()
repo=Path(__file__).resolve().parents[2]
if output==repo or repo in output.parents:
 ap.error('Raw output must be outside this source checkout')
for source in [a.world.resolve(),a.resources.resolve()]:
 if output==source or source in output.parents or output in source.parents:
  ap.error('Output must not overlap an input world/resources directory')
if min(a.scale,a.width,a.height)<=0: ap.error('Scale and dimensions must be positive')
a.output=output
a.output.mkdir(parents=True,exist_ok=True)
label=a.output.name
root=Path(tempfile.mkdtemp(prefix='horizon-pack-bench-'))
for name in ['game','launcher','data','config']: (root/name).mkdir()
source=a.world.resolve()
shutil.copytree(source,root/'game/saves/RenderBugs')
(root/'game/options.txt').write_text('maxFps:260\ninactivityFpsLimit:minimized\nenableVsync:false\npauseOnLostFocus:false\nchatVisibility:2\n')
(root/'game/horizon-graphics.txt').write_text(f'resolutionScale:{a.scale}\n')
env=os.environ.copy()
for k,n in [('HORIZON_HOME','game'),('HORIZON_LAUNCHER_HOME','launcher'),('XDG_DATA_HOME','data'),('XDG_CONFIG_HOME','config')]: env[k]=str(root/n)
env.update(HORIZON_NO_AUDIO='1',HORIZON_LAUNCHER_NO_SELF_UPDATE='1',HORIZON_SHADER_PACK_TIMING='1',HORIZON_SHADER_PACK=str(a.pack.resolve()))
sock=root/('ctl.json' if os.name=='nt' else 'ctl.sock')
log=(root/'process.log').open('w')
args=[str(exe),'--resources',str(a.resources.resolve()),'--game-dir',str(root/'game'),'--home',str(root/'game'),'--world','RenderBugs','--creative','--render-distance','6','--offscreen','--frames','100000','--timeout-seconds','600','--width',str(a.width),'--height',str(a.height),'--control-socket',str(sock),'--name','RendererTester']
p=subprocess.Popen(args,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT)
def call(*a):
 r=subprocess.run([str(ctl),'--socket',str(sock),'--timeout','60',*a],capture_output=True,text=True,timeout=65)
 if r.returncode: raise RuntimeError(r.stderr+r.stdout)
 v=json.loads(r.stdout)
 if 'error' in v: raise RuntimeError(v)
 return v
out=a.output.resolve()
try:
 deadline=time.monotonic()+90
 while not sock.exists():
  if p.poll() is not None or time.monotonic()>deadline: raise RuntimeError('startup failed '+str(root))
  time.sleep(.3)
 call('wait-ready');call('options-set','pauseOnLostFocus','false');call('options-set','chatVisibility','2')
 if call('ui')['screen']=='paused': call('click','back-to-game')
 for cmd in ['gamerule minecraft:advance_time false','weather clear','time set 8000']:call('command',cmd)
 results={}
 for scene,tp in [('cave','tp @s -155.083 -11.80064 -165.083 98.3 -0.8'),('outdoor','tp @s -10 100 -20 0 10')]:
  call('command',tp);time.sleep(30)
  call('look-at',*(['-165','-10','-165'] if scene=='cave' else ['-10','100','0']));time.sleep(2)
  results[scene]=[]
  for _ in range(3):
   call('look-at',*(['-165','-10','-165'] if scene=='cave' else ['-10','100','0']));time.sleep(1)
   before=call('state')['player']
   sample=call('bench','8')
   after=call('state')['player']
   sample['camera_before']={k:before[k] for k in ['position','yaw','pitch']}
   sample['camera_after']={k:after[k] for k in ['position','yaw','pitch']}
   sample['camera_stable']=sample['camera_before']==sample['camera_after']
   results[scene].append(sample)
  results[scene+'_state']=call('state')
  call('screenshot',str(out/(scene+'.png')))
 results['status']=call('state');results['runtime']=call('status');results['root']=str(root)
 results['conditions']={'scale':a.scale,'logical_width':a.width,'logical_height':a.height,'chunks':6,'warmup_seconds':30,'samples':3,'sample_seconds':8,'vsync':False,'max_fps':260}
 (out/'measurements.json').write_text(json.dumps(results,indent=2))
 print(str(out/'measurements.json'),flush=True)
finally:
 try:call('quit')
 except Exception:p.terminate()
 try:
  try: p.wait(timeout=30)
  except subprocess.TimeoutExpired:
   p.terminate()
   try: p.wait(timeout=10)
   except subprocess.TimeoutExpired: p.kill();p.wait()
 finally:
  log.close()
  shutil.copy2(root/'process.log',out/'process.log')

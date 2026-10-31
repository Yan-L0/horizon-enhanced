exec(open('work/render-upgrade/scene_test.py').read().split("deadline=time.monotonic()")[0])
result=[]
for dim in ['the_nether','the_end','overworld']:
 command(f'execute in minecraft:{dim} run tp @s 0 80 0')
 deadline=time.monotonic()+40
 while True:
  state=call('wait-ready')
  if state.get('dimension')=='minecraft:'+dim:break
  if time.monotonic()>deadline:raise RuntimeError(('dimension did not change',dim,state))
  time.sleep(.2)
 command('fill -3 78 -3 3 85 3 minecraft:air')
 time.sleep(2)
 shot(dim)
 result.append({'dimension':dim,'backend':state.get('renderer',{}).get('backend'),'screen':state.get('screen')})
log=(Path(c['root'])/'game.log').read_text()
assert 'folder `world-1`' in log and 'folder `world1`' in log, log
assert 'turned off:' not in log and 'panicked at' not in log,log
(out/'validation-dimensions.json').write_text(json.dumps(result,indent=2))
(out/'runtime.log').write_text(log)
print(json.dumps(result),flush=True)

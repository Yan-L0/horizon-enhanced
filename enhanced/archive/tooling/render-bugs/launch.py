#!/usr/bin/env python3
"""Horizon Enhanced bootstrap. Keep runtime data outside macOS protected Documents."""
import datetime
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import traceback


def main():
    resources = Path(__file__).resolve().parent
    app = resources.parent.parent
    override = os.environ.get('HORIZON_HOME')
    home = Path(override).expanduser().resolve() if override else Path.home() / 'Library/Application Support/Horizon Enhanced'
    home.mkdir(parents=True, exist_ok=True)
    logs = home / 'logs'
    logs.mkdir(exist_ok=True)
    log_path = logs / 'launcher.log'
    with log_path.open('a', buffering=1) as log:
        os.dup2(log.fileno(), 1)
        os.dup2(log.fileno(), 2)
        print('\n=== Horizon Enhanced launch', datetime.datetime.now().isoformat(), '===', flush=True)
        print('Enhanced build 0.2.0; game folder:', home, flush=True)
        try:
            # Preserve worlds and preferences from the original portable release.
            # Copies only missing destinations; never deletes or overwrites old data.
            legacy = app.parent / 'EnhancedUserData'
            if not override and legacy.is_dir() and legacy.resolve() != home.resolve():
                for name in ('saves', 'shaderpacks', 'resourcepacks', 'horizon-mods', 'options.txt',
                             'horizon-graphics.txt', 'horizon-extras.txt', 'horizon-mods.txt',
                             'horizon-player.txt', 'servers.dat'):
                    src, dst = legacy / name, home / name
                    if src.exists() and not dst.exists():
                        print('Preserving legacy data:', name, flush=True)
                        if src.is_dir():
                            shutil.copytree(src, dst)
                        else:
                            shutil.copy2(src, dst)
            prepared = Path.home() / 'Library/Application Support/horizon-launcher/versions/nightly-2026-10-30-1da354b/.local/horizon-client/26.3'
            if not (prepared / 'items.json').is_file():
                raise RuntimeError('Prepared Horizon 26.3 resources were not found: ' + str(prepared))
            packs = home / 'shaderpacks'
            packs.mkdir(exist_ok=True)
            pack = packs / 'BSL_v10.1.8.zip'
            if not pack.exists():
                shutil.copy2(resources / pack.name, pack)
            for name in ('BSL_Horizon_Balanced_v10.1.8.zip', 'BSL_Horizon_Quality_v10.1.8.zip'):
                target = packs / name
                if not target.exists():
                    shutil.copy2(resources / name, target)
            graphics = home / 'horizon-graphics.txt'
            if not graphics.exists():
                graphics.write_text('pickedShaderPack:BSL_Horizon_Quality_v10.1.8.zip\nresolutionScale:75\n')
            marker = home / 'horizon-enhanced-render-version.txt'
            if not marker.exists() or marker.read_text().strip() != '0.2.0':
                text = graphics.read_text()
                old_choice = 'pickedShaderPack:BSL_v10.1.8.zip'
                if old_choice in text.splitlines():
                    lines = ['pickedShaderPack:BSL_Horizon_Quality_v10.1.8.zip' if line == old_choice else line for line in text.splitlines()]
                    graphics.write_text('\n'.join(lines) + '\n')
                marker.write_text('0.2.0\n')
            options = home / 'options.txt'
            if not options.exists():
                options.write_text('renderDistance:6\nmaxFps:60\nenableVsync:true\n')
            os.environ['HORIZON_HOME'] = str(home)
            os.environ['HORIZON_LAUNCHER_NO_SELF_UPDATE'] = '1'
            os.chdir(home)
            cmd = [str(app / 'Contents/MacOS/horizon-client'), '--resources', str(prepared),
                   '--game-dir', str(home), '--home', str(home), '--dev-mod',
                   str(resources / 'upgrader-content.hmod'), '--name', 'lol']
            print('Starting renderer', flush=True)
            os.execv(cmd[0], cmd + sys.argv[1:])
        except Exception as error:
            traceback.print_exc()
            log.flush()
            if not os.environ.get('HORIZON_NO_ERROR_DIALOG'):
                message = 'Не удалось запустить Horizon Enhanced.\n' + str(error) + '\n\nЖурнал: ' + str(log_path)
                subprocess.run(['/usr/bin/osascript', '-e', 'display alert "Horizon Enhanced" message ' + json.dumps(message, ensure_ascii=False) + ' as critical'], check=False)
            return 1


if __name__ == '__main__':
    raise SystemExit(main())

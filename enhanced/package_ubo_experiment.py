#!/usr/bin/env python3
"""Package explicitly supplied private UBO-experiment binaries and shader inputs."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import shutil
import subprocess
import zipfile

VERSION = '0.4.2-ubo-experimental.1'
SOURCE = 'ccaed52874cbb57d23455d8c731c2964ee7bf818'
REPO = Path(__file__).resolve().parents[1]
PACKS = ('BSL_v10.1.8.zip', 'BSL_Horizon_Quality_r2_v10.1.8.zip',
         'BSL_Horizon_Balanced_r2_v10.1.8.zip')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ('mac-client', 'mac-ctl', 'windows-client', 'windows-ctl', 'packs', 'output'):
        parser.add_argument('--' + flag, type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    if output == REPO or REPO in output.parents:
        parser.error('Keep generated releases outside the source checkout')
    if output.exists():
        parser.error('Output directory already exists; choose a new directory')
    for client in (args.mac_client, args.windows_client):
        if b'inactive host fields retained as zero' not in client.read_bytes():
            parser.error('Client does not contain the final UBO experiment: ' + str(client))
    for name in PACKS:
        if not (args.packs / name).is_file():
            parser.error('Missing private shader input: ' + name)
    output.mkdir(parents=True)
    windows = output / 'windows' / 'Horizon Enhanced Experimental'
    mac = output / 'macos'
    windows.mkdir(parents=True)
    mac.mkdir()
    app = mac / 'Horizon Enhanced Experimental.app'
    executables = app / 'Contents/MacOS'
    resources = app / 'Contents/Resources'
    executables.mkdir(parents=True)
    resources.mkdir()
    for binary, target in ((args.mac_client, executables / 'horizon-client'),
                           (args.mac_ctl, executables / 'horizon-ctl'),
                           (args.windows_client, windows / 'horizon-client.exe'),
                           (args.windows_ctl, windows / 'horizon-ctl.exe')):
        shutil.copy2(binary, target)
        target.chmod(0o755)
    bootstrap = (REPO / 'enhanced/platform/macos/launch.py').read_text()
    bootstrap = bootstrap.replace('Enhanced build 0.4.0;', 'Enhanced build ' + VERSION + ';')
    (resources / 'launch.py').write_text(bootstrap)
    (executables / 'launch').write_text('#!/bin/sh\nexec /usr/bin/python3 "$(dirname "$0")/../Resources/launch.py" "$@"\n')
    (executables / 'launch').chmod(0o755)
    info = dict(CFBundleDisplayName='Horizon Enhanced Experimental', CFBundleName='Horizon Enhanced Experimental',
                CFBundleExecutable='launch', CFBundleIdentifier='dev.private.horizon.enhanced.uboexperiment',
                CFBundlePackageType='APPL', CFBundleShortVersionString='0.4.2', CFBundleVersion='0.4.2',
                NSHighResolutionCapable=True)
    (app / 'Contents/Info.plist').write_bytes(plistlib.dumps(info))
    for name in ('Launch-Horizon.cmd', 'Launch-Horizon.ps1'):
        text = (REPO / 'enhanced/platform/windows' / name).read_text()
        # The existing migration marker remains unchanged; only identify this package in logs.
        text = text.replace("'Horizon Enhanced ' + $BuildVersion", "'Horizon Enhanced " + VERSION + "'")
        (windows / name).write_bytes(text.replace('\r\n', '\n').replace('\n', '\r\n').encode('utf-8'))
    for directory in (resources, windows / 'resources'):
        directory.mkdir(exist_ok=True)
        for name in PACKS:
            shutil.copy2(args.packs / name, directory / name)
    readme = '''Horizon Enhanced 0.4.2 — UBO experiment

Это отдельный эксперимент, не подтверждённое ускорение FPS. Пройдены 84 теста.
Windows x64 и macOS Apple Silicon. Исходники: perf/ubo-upload-experiment, ccaed52.

1. Полностью распакуйте архив в новую папку.
2. Закройте обычный Horizon Enhanced.
3. Windows: запустите Launch-Horizon.cmd. macOS: откройте Horizon Enhanced Experimental.app.

Сборка использует существующие миры и настройки Enhanced:
Windows: %APPDATA%\\Horizon Enhanced
macOS: ~/Library/Application Support/Horizon Enhanced
Игровые ресурсы Minecraft 26.3 должны быть подготовлены ранее через Horizon launcher.
Архив не содержит миров, аккаунтов или ресурсов Mojang. Upgrader отключён.

Сравнивайте одну сцену, ракурс, профиль шейдеров, разрешение, масштаб и дальность прорисовки.
Дайте сцене прогрузиться 30–60 секунд. Лимит FPS/VSync должен совпадать в обеих сборках.
При упоре в лимит 60 FPS сравнение не покажет возможную разницу выше этого лимита.

Изменение: обработка только используемых параметров шейдера и пропуск одинаковых записей.
Эффекты и качество профиля не уменьшены. Прирост FPS в локальных замерах не подтверждён.
Windows-видеокарта отдельно не проверена. Для возврата закройте эксперимент и откройте прежнюю сборку.
'''
    patch = subprocess.check_output(['git', 'diff', 'fb91630', SOURCE, '--', 'crates'], cwd=REPO)
    validation = dict(version=VERSION, source_commit=SOURCE, source_branch='perf/ubo-upload-experiment',
                      shader_tests=51, renderer_tests=33, exact_gpu_equivalence_test=True,
                      macos_compiled=True, windows_x64_compiled=True, windows_gpu_tested=False,
                      demonstrated_fps_gain=False, upgrader=False)
    for directory, target in ((mac, 'aarch64-apple-darwin'), (windows, 'x86_64-pc-windows-msvc')):
        (directory / 'START-HERE-RU.txt').write_text(readme, encoding='utf-8-sig')
        shutil.copy2(REPO / 'LICENSE', directory / 'HORIZON-LICENSE.txt')
        shutil.copy2(REPO / 'THIRD_PARTY_NOTICES.md', directory / 'THIRD_PARTY_NOTICES.md')
        shutil.copy2(REPO / 'enhanced/validation/gemini-render-audit-2026-11-02.md', directory / 'EXPERIMENT-REPORT.md')
        (directory / 'renderer.patch').write_bytes(patch)
        (directory / 'VALIDATION.json').write_text(json.dumps(validation, indent=2) + '\n')
        if target.startswith('aarch64'):
            subprocess.run(['codesign', '--force', '--deep', '--sign', '-', str(app)], check=True)
            subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
        manifest = dict(validation, target=target, files={str(p.relative_to(directory)):sha(p)
                        for p in sorted(directory.rglob('*')) if p.is_file()})
        (directory / 'BUILD.json').write_text(json.dumps(manifest, indent=2) + '\n')
    archives = []
    for directory, platform in ((windows, 'Windows-x64'), (mac, 'macOS-arm64')):
        archive = output / ('Horizon-Enhanced-' + platform + '-v' + VERSION + '.zip')
        prefix = 'Horizon Enhanced Experimental'
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as z:
            for path in sorted(directory.rglob('*')):
                if path.is_file():
                    z.write(path, prefix + '/' + str(path.relative_to(directory)))
        with zipfile.ZipFile(archive) as z:
            assert z.testzip() is None
        archives.append((archive, sha(archive)))
    (output / 'SHA256SUMS.txt').write_text(''.join(digest + '  ' + path.name + '\n' for path,digest in archives))
    for path, digest in archives:
        print(path)
        print(digest)


if __name__ == '__main__':
    main()

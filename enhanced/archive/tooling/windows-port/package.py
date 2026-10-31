# -*- coding: utf-8 -*-
from pathlib import Path
import shutil, json, hashlib, zipfile

base = Path(__file__).resolve().parents[2]
out = base / 'outputs/Horizon-Enhanced-Windows-x64-0.4.0'
src = base / 'outputs/horizon-render-max'
for name in ['renderer.patch', 'HORIZON-LICENSE.txt', 'THIRD_PARTY_NOTICES.md', 'BSL_Horizon_Quality_r2_v10.1.8.json', 'BSL_Horizon_Balanced_r2_v10.1.8.json']:
    shutil.copy2(src / name, out / name)
shutil.copy2(base / 'work/windows-port/WINDOWS-LAUNCH-README.txt', out / 'README-English.txt')
for name in ['Launch-Horizon.cmd', 'Launch-Horizon.ps1']:
    text = (base / 'work/windows-port' / name).read_text()
    (out / name).write_bytes(text.replace('\r\n', '\n').replace('\n', '\r\n').encode('utf-8'))
(out / 'START-HERE-RU.txt').write_text('''Horizon Enhanced 0.4.0 — Windows x64

1. Распакуйте весь архив в папку на Windows.
2. Подготовьте Minecraft 26.3 через официальный Horizon launcher на этом ПК. Если ресурсы уже готовы, повторять не нужно.
3. Запустите Launch-Horizon.cmd. Он задаёт ресурсы, шейдеры и папку данных.

Данные сборки: %APPDATA%\\Horizon Enhanced. Миры обычного Minecraft/Horizon не изменяются и автоматически не копируются.
Ресурсы находятся автоматически в %APPDATA%\\horizon-launcher\\versions. Игровые файлы Mojang в архив не включены.
Шейдеры: Quality r2 по умолчанию, Balanced r2 доступен в настройках. Upgrader отсутствует.
Python, Rust и Visual C++ Redistributable для запуска не нужны: CRT включён статически.

Перенесены исправления растений, трещин и обводки, камеры теней, лишних mip-цепочек и расчёта скрытого неба. r2 подавляет ложные подземные лучи через настройку BSL; это приближение по высоте и небесному освещению.

Проверено в Windows 11 ARM VM: настоящий x64 EXE запускается, --launch-api работает, PowerShell 5.1 разбирает запускатель, пути с пробелами и миграция профиля работают, настройки и контрольное сохранение сохранены. Ресурсы nightly 9d979ca загрузились, DirectX 12 инициализировался и временный сервер принял игрока. Полная визуальная проверка шейдеров не выполнена: программный Microsoft Basic Render Driver, загрузка ресурсов 171 секунду, лимит теста. Производительность и картинка на настоящей видеокарте Windows пока не подтверждены. На Mac изменения прошли 28 тестов рендерера.

При ошибке пришлите свежий журнал из %APPDATA%\\Horizon Enhanced\\logs: launcher.log и client-...stderr.log.
Для своего пути ресурсов задайте HORIZON_RESOURCES, для отдельной папки данных — HORIZON_HOME. Подробности в README-English.txt.

Agent Control:
Launch-Horizon.cmd --control-socket "%TEMP%\\horizon-enhanced-control.json"
horizon-ctl.exe --socket "%TEMP%\\horizon-enhanced-control.json" state

База 1da354b67a509812c4141b67de2affd6de65c342 плюс наши изменения 0.4.0. Официальный launcher не заменяется.
''', encoding='utf-8-sig')
report_path = base / 'work/windows-port/vm-test.json'
report = json.loads(report_path.read_text(encoding='utf-8-sig'))
report.update(target='x86_64-pc-windows-msvc', resources_loaded=True, dx12_initialized=True,
              vm_gpu='Microsoft Basic Render Driver (CPU)', temporary_world_server_join=True,
              full_shader_visual_test=False, real_gpu_performance_test=False,
              limitation='VM startup exceeded test time budget; resources loaded in 171.5s.')
(out / 'VALIDATION.json').write_text(json.dumps(report, indent=2))
manifest = dict(name='Horizon Enhanced', version='0.4.0-windows.1-private',
                target='x86_64-pc-windows-msvc', base_commit='1da354b67a509812c4141b67de2affd6de65c342',
                rust='1.99.1', compiler='cargo-xwin 0.23.1, LLVM 23.1.4, MSVC SDK',
                profile='fast: release optimization, no LTO', crt='static', upgrader=False,
                files={str(p.relative_to(out)): hashlib.sha256(p.read_bytes()).hexdigest()
                       for p in out.rglob('*') if p.is_file() and p.name != 'BUILD.json'})
(out / 'BUILD.json').write_text(json.dumps(manifest, indent=2))
archive = base / 'outputs/Horizon-Enhanced-Windows-x64-v0.4.0.zip'
with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as z:
    for p in out.rglob('*'):
        if p.is_file(): z.write(p, 'Horizon Enhanced/' + str(p.relative_to(out)))
with zipfile.ZipFile(archive) as z:
    assert z.testzip() is None
    assert not any('upgrader' in n.lower() for n in z.namelist())
print(archive)
print(hashlib.sha256(archive.read_bytes()).hexdigest())

#!/usr/bin/env python3
"""Build private BSL profiles with its supported underground-sky option.

This is an explicit coverage approximation: outside the renderer's loaded shadow
casters BSL assumes sun visibility. Its SKY_UNDERGROUND option suppresses that
fallback where the camera has no skylight below y=48; it also changes underground
sky/fog behavior. It cannot recover sunlight occlusion in unloaded geometry.
The additional early return is exact under those option conditions, and skips
neither block-light fog nor any sample where the original multiplier is nonzero.
"""
from pathlib import Path
import hashlib
import json
import zipfile

BASE = Path(__file__).resolve().parents[2]
SOURCE = BASE / 'outputs/horizon-render-optimized'
OUTPUT = BASE / 'outputs/horizon-render-max'
SETTINGS = 'shaders/lib/settings.glsl'
SHAFTS = 'shaders/lib/atmospherics/lightShafts.glsl'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def make(kind):
    source = SOURCE / f'BSL_Horizon_{kind}_v10.1.8.zip'
    target = OUTPUT / f'BSL_Horizon_{kind}_r2_v10.1.8.zip'
    changes = []
    with zipfile.ZipFile(source) as src, zipfile.ZipFile(target, 'w', zipfile.ZIP_DEFLATED) as dst:
        for info in src.infolist():
            before = src.read(info.filename)
            after = before
            if info.filename == SETTINGS:
                text = before.decode('utf-8')
                old = '//#define SKY_UNDERGROUND'
                assert text.count(old) == 1, 'Expected one disabled supported underground option'
                after = text.replace(old, '#define SKY_UNDERGROUND').encode('utf-8')
            elif info.filename == SHAFTS:
                text = before.decode('utf-8')
                marker = 'vec3 GetLightShafts(vec3 viewPos, float linZ0, float linZ1, float hand, float dither, vec3 translucent) {'
                assert text.count(marker) == 1
                newline = '\r\n' if '\r\n' in text else '\n'
                guard = newline.join([
                    '',
                    '\t// Horizon private profile: the existing underground multiplier is exactly zero.',
                    '\t// Keep block-light fog and all sky-lit samples on the original ray-march path.',
                    '#if defined OVERWORLD && defined SKY_UNDERGROUND && !defined MCBL_FOG',
                    '\tif (eBS == 0.0 && cameraPosition.y <= 48.0) return vec3(0.0);',
                    '#endif',
                    '',
                ])
                after = text.replace(marker, marker + guard).encode('utf-8')
            if after != before:
                changes.append({'path': info.filename, 'before_sha256': sha(before), 'after_sha256': sha(after)})
            dst.writestr(info, after)
    assert len(changes) == 2
    with zipfile.ZipFile(target) as check:
        assert check.testzip() is None
        assert len(check.infolist()) > 0
    metadata = {
        'source': str(source.relative_to(BASE)),
        'source_sha256': sha(source.read_bytes()),
        'target': target.name,
        'target_sha256': sha(target.read_bytes()),
        'changes': changes,
        'supported_setting': {'SKY_UNDERGROUND': True},
        'visual_change': 'Supported BSL underground-sky option gates its existing shaft/sky/fog fallback by local skylight and height. This suppresses false sunlight from missing distant shadow casters; it is a coverage approximation, not reconstruction of unloaded terrain.',
        'early_return': 'Exact when OVERWORLD and SKY_UNDERGROUND and not MCBL_FOG and eBS == 0 and cameraPosition.y <= 48: original ApplyLightShaftMultiplier underground multiplier is zero; original final vl remains exactly zero. Block-light fog cases retain the original loop.',
        'unchanged': 'All prior quality/shadow/bloom/water/AA settings, texture assets, author notices and copyrights. No source from another renderer was used.',
    }
    target.with_suffix('.json').write_text(json.dumps(metadata, indent=2) + '\n')
    print(json.dumps({'profile': target.name, 'sha256': metadata['target_sha256']}))


if __name__ == '__main__':
    OUTPUT.mkdir(parents=True, exist_ok=True)
    for profile in ('Quality', 'Balanced'):
        make(profile)

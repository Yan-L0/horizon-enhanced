"""Rebuild private BSL r2 profiles from the user's own BSL 10.1.8 ZIP."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import zipfile

PRESETS = {
    'Quality': {'FOG_DENSITY_END': '0.37', 'LIGHT_SHAFT_END_SMOKE': '0.25'},
    'Balanced': {'shadowMapResolution': '1024', 'shadowDistance': '128.0',
                 'CLOUD_THICKNESS': '4', 'BLOOM_RADIUS': '2', 'FOG_DENSITY_END': '0.37',
                 'LIGHT_SHAFT_END_SMOKE': '0.25', 'FXAA': False, 'AO': False},
}

def build(source, output):
    output.mkdir(parents=True, exist_ok=True)
    for kind, settings in PRESETS.items():
        target = output / f'BSL_Horizon_{kind}_r2_v10.1.8.zip'
        with zipfile.ZipFile(source) as src, zipfile.ZipFile(target, 'w', zipfile.ZIP_DEFLATED) as dst:
            for info in src.infolist():
                data = src.read(info)
                if info.filename == 'shaders/lib/settings.glsl':
                    text = data.decode()
                    for key, value in settings.items():
                        if value is False:
                            text, n = re.subn(r'(?m)^(\s*)#define '+re.escape(key)+r'\s*(?=\r?$)', r'\1//#define '+key, text, count=1)
                        elif key in ('shadowMapResolution', 'shadowDistance'):
                            text, n = re.subn(r'(const (?:int|float) '+key+r'\s*=\s*)[^;]+;', r'\g<1>'+value+';', text, count=1)
                        else:
                            text, n = re.subn(r'(?m)^(\s*#define '+key+r'\s+)\S+', r'\g<1>'+value, text, count=1)
                        if n != 1:
                            raise ValueError(f'Unexpected BSL source for {kind}/{key}')
                    if text.count('//#define SKY_UNDERGROUND') != 1:
                        raise ValueError('Expected original BSL SKY_UNDERGROUND option')
                    data = text.replace('//#define SKY_UNDERGROUND', '#define SKY_UNDERGROUND').encode()
                elif info.filename == 'shaders/lib/atmospherics/lightShafts.glsl':
                    text = data.decode()
                    marker = 'vec3 GetLightShafts(vec3 viewPos, float linZ0, float linZ1, float hand, float dither, vec3 translucent) {'
                    if text.count(marker) != 1:
                        raise ValueError('Unexpected BSL light-shaft program')
                    newline = '\r\n' if '\r\n' in text else '\n'
                    guard = newline.join(['',
                        '\t// Horizon private profile: the existing underground multiplier is exactly zero.',
                        '\t// Keep block-light fog and all sky-lit samples on the original ray-march path.',
                        '#if defined OVERWORLD && defined SKY_UNDERGROUND && !defined MCBL_FOG',
                        '\tif (eBS == 0.0 && cameraPosition.y <= 48.0) return vec3(0.0);', '#endif', ''])
                    data = text.replace(marker, marker + guard).encode()
                dst.writestr(info, data)
        with zipfile.ZipFile(target) as check:
            if check.testzip() is not None:
                raise ValueError('Invalid generated profile ZIP')
        target.with_suffix('.json').write_text(json.dumps({
            'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
            'output_sha256': hashlib.sha256(target.read_bytes()).hexdigest(),
            'preset': kind, 'settings': settings, 'SKY_UNDERGROUND': True,
            'note': 'Private derivative; original author rights retained. Underground sky is a coverage approximation.'
        }, indent=2))
        print(target)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bsl', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    build(args.bsl, args.output)

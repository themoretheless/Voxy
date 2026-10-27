"""Hidden deterministic frame-120 HDR A/A and A/B; no performance qualification."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

before, after, output, materials = map(Path, sys.argv[1:])
images = {}
for name, binary in [('weight-A0', before), ('weight-A1', before), ('weight-B', after)]:
    env = {k: v for k, v in os.environ.items() if not k.startswith(('VOXY_STREET_', 'VOXY_FX_'))}
    env.update(VOXY_STREET_MATERIALS=str(materials.resolve()),
               VOXY_STREET_GPU_TIMING='1',
               VOXY_STREET_BENCHMARK_REPORT=str((output / f'{name}.json').resolve()))
    with (output / f'{name}.log').open('w') as log:
        subprocess.run([str(binary.resolve()), '--offscreen-benchmark',
                        '--offscreen-render-every-step', '--benchmark', '.1',
                        '--snapshot', str((output / f'{name}.png').resolve())],
                       env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    images[name] = (output / f'{name}.pfm').read_bytes()
    print(name, hashlib.sha256(images[name]).hexdigest(), flush=True)
result = {'scope': 'frame-120 full linear Float32 RGB PFM, inline zero-weight GI admission',
          'sha256': {k: hashlib.sha256(v).hexdigest() for k, v in images.items()},
          'aa_byte_equal': images['weight-A0'] == images['weight-A1'],
          'ab_byte_equal': images['weight-A0'] == images['weight-B']}
(output / 'weight-image-comparison.json').write_text(json.dumps(result, indent=2) + '\n')
if not result['aa_byte_equal'] or not result['ab_byte_equal']:
    raise SystemExit('Raw HDR image parity failed; investigate before claiming unchanged output')

#!/usr/bin/env python3
"""Offline 44-byte render-ray schema prototype; never changes the engine ABI."""
import argparse
import hashlib
import json
import struct
from pathlib import Path

COEFFICIENTS = [0, 1, 2, 8, 9, 10, 12, 13, 14]
HEADER = struct.Struct('<10I')  # grid metadata (6), reserved (2), queue header (2)
ROW = struct.Struct('<11I')
LEGACY = struct.Struct('<20I')


def pack(raw, metadata):
    x, y, width, height, frame_width, kind = metadata
    rows = list(LEGACY.iter_unpack(raw))
    assert len(rows) == width * height and rows
    encoded = bytearray(HEADER.pack(*metadata, 0, 0, rows[0][7], rows[0][11]))
    for index, row in enumerate(rows):
        assert all(row[k] == 0 for k in [4, 5, 6, 16, 17]), 'unsupported nonzero implicit word'
        assert index == 0 or (row[7] == row[11] == 0), 'non-header diagnostic padding not eligible'
        assert row[3] in [0, 1, 2], 'not an archive-producer admission'
        identity = (y + index // width) * frame_width + x + index % width
        assert row[18:20] == (identity, kind), 'grid identity mismatch'
        encoded += ROW.pack(*(row[k] for k in COEFFICIENTS), row[3], row[15])
    return bytes(encoded)


def unpack(encoded):
    x, y, width, height, frame_width, kind, zero0, zero1, count, cursor = HEADER.unpack_from(encoded)
    assert zero0 == zero1 == 0 and len(encoded) == HEADER.size + width * height * ROW.size
    result = bytearray()
    for index, data in enumerate(ROW.iter_unpack(encoded[HEADER.size:])):
        row = [0] * 20
        for key, value in zip(COEFFICIENTS, data[:9]):
            row[key] = value
        row[3], row[15] = data[9:]
        if index == 0:
            row[7], row[11] = count, cursor
        row[18] = (y + index // width) * frame_width + x + index % width
        row[19] = kind
        result += LEGACY.pack(*row)
    return bytes(result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    frame = json.loads((args.capture / 'world-frame.json').read_text())
    records = []
    total_raw = total_packed = total_rays = 0
    for kind in range(2):
        for index, (x, y, width, height) in enumerate(frame['tiles']):
            path = args.capture / f'frame-{kind}-tile-{index}-input.bin'
            raw = path.read_bytes()
            encoded = pack(raw, (x, y, width, height, frame['size'][0], kind))
            assert unpack(encoded) == raw, path.name
            rows = list(LEGACY.iter_unpack(raw))
            # A single nonzero low-origin bit must reject this schema, even if
            # its numeric value is tiny or signed zero. Keep the general ABI.
            for bad_value in [1, 0x80000000]:
                bad = list(rows[0]); bad[4] = bad_value
                changed = LEGACY.pack(*bad) + raw[LEGACY.size:]
                try:
                    pack(changed, (x, y, width, height, frame['size'][0], kind))
                except AssertionError:
                    pass
                else:
                    raise AssertionError('nonzero low-origin bit was silently discarded')
            total_raw += len(raw); total_packed += len(encoded); total_rays += len(rows)
            records.append({'file': path.name, 'rays': len(rows), 'legacy_bytes': len(raw),
                            'prototype_bytes': len(encoded), 'roundtrip_sha256': hashlib.sha256(raw).hexdigest()})
    # Bit-domain cases include signed zero, subnormals, infinities and NaN payloads.
    # This is a codec test, not a claim these operands are valid geometric rays.
    corners = [0, 0x80000000, 1, 0x007fffff, 0x00800000,
               0x7f7fffff, 0xff7fffff, 0x7f800000, 0xff800000, 0x7fc00001, 0x7fa00001]
    for shift in range(len(corners)):
        rows = []
        for index in range(4):
            row = [0] * 20
            for k, word in enumerate(COEFFICIENTS):
                row[word] = corners[(shift + index + k) % len(corners)]
            row[3] = index % 3; row[15] = 3-index
            row[18] = index; row[19] = 1
            if index == 0:
                row[7] = 0xffffffff; row[11] = 0xfffffffe
            rows.append(row)
        raw = b''.join(LEGACY.pack(*row) for row in rows)
        assert unpack(pack(raw, (0, 0, 4, 1, 4, 1))) == raw
    result = {'scope': 'offline captured render-producer words, including mutable queue padding',
              'rays': total_rays, 'legacy_bytes': total_raw, 'prototype_bytes': total_packed,
              'all_twenty_words_bit_exact': True, 'nonzero_origin_low_rejected': True,
              'synthetic_bit_domain_cases': len(corners), 'row_bytes': ROW.size, 'per_region_header_bytes': HEADER.size,
              'runtime_format_changed': False, 'gpu_codec_implemented': False,
              'full_street_qualified': False, 'speedup_claimed': False, 'records': records}
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k: v for k, v in result.items() if k != 'records'}))


if __name__ == '__main__':
    main()

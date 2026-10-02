"""Plot bind-space globe exposure samples exported by the lid diagnostic."""
import argparse
import csv
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt


def plot(source, output):
    with source.open(newline='') as stream:
        rows = list(csv.DictReader(stream))
    rows = [r for r in rows if float(r['closure']) == 1.]
    if not rows:
        raise ValueError('No full-closure samples')
    fig, axes = plt.subplots(2, 2, figsize=(12, 6), layout='constrained')
    for column, side in enumerate([1., -1.]):
        samples = [r for r in rows if float(r['side']) == side]
        gaps = [r for r in samples if int(r['globe_visible'])]
        x = [abs(float(r['x'])) * 1000 for r in samples]
        y = [(float(r['y']) - 0.7104) * 1000 for r in samples]
        gx = [abs(float(r['x'])) * 1000 for r in gaps]
        gy = [(float(r['y']) - 0.7104) * 1000 for r in gaps]
        for row in range(2):
            axis = axes[row, column]
            axis.scatter(x, y, s=2, c='#b6c8d8', marker='s', linewidths=0, rasterized=True)
            axis.scatter(gx, gy, s=10 if row else 3, c='#d63232', marker='s', linewidths=0, label='Exposed globe')
            axis.axhline(0., color='#374453', linewidth=0.7, linestyle='--', label='Target contact line')
            axis.set_xlim(19, 47)
            axis.set_xlabel('Absolute bind X (mm)')
            axis.set_ylabel('Y from target contact line (mm)')
            axis.grid(alpha=0.18)
        axes[0, column].set_ylim(-9, 12)
        axes[0, column].set_title(f'Eye {side:+.0f}X: {len(gaps)} / {len(samples)} samples exposed')
        axes[1, column].set_ylim(-0.8, 1.0)
        axes[1, column].set_title('Contact strip, enlarged vertically')
    axes[1, 1].legend(loc='upper right', fontsize=8)
    tissue = "skin + wet rim" if any(r.get("rim_z") for r in rows) else "skin"
    fig.suptitle(f"Full closure: actual globe versus {tissue} depth, 0.125 mm sampling\nOrthographic bind-space diagnostic; coarse-grid zero exposure is not full contact proof", fontsize=11)
    fig.savefig(output, dpi=160)
    plt.close(fig)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    plot(args.source, args.output)

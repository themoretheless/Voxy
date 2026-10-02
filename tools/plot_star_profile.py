"""Plot the final SI shell profile exported by the reactive_star example."""
import argparse
import csv
import math
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('profile', type=Path)
    parser.add_argument('output', type=Path, help='PNG, PDF or SVG output')
    args = parser.parse_args()
    with args.profile.open(newline='') as source:
        rows = [{key: float(value) for key, value in row.items()}
                for row in csv.DictReader(source)]
    if not rows:
        parser.error('profile contains no shells')
    required = ['time_s', 'inner_radius_m', 'outer_radius_m', 'density_kg_m3',
                'temperature_K', 'gas_pressure_Pa', 'radiation_pressure_Pa',
                'carbon_fraction', 'enclosed_mass_kg']
    edge = 0.0
    for row in rows:
        if any(key not in row or not math.isfinite(row[key]) for key in required):
            parser.error('missing or nonfinite profile values')
        if not math.isclose(row['inner_radius_m'], edge, abs_tol=1e-10) or row['outer_radius_m'] <= edge:
            parser.error('shell radii must be contiguous and start at zero')
        if row['time_s'] != rows[0]['time_s']:
            parser.error('profile must contain one time snapshot')
        if any(row[key] <= 0 for key in ['density_kg_m3', 'temperature_K', 'gas_pressure_Pa', 'radiation_pressure_Pa']):
            parser.error('density, temperature and pressures must be positive')
        edge = row['outer_radius_m']
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    radius = [(r['inner_radius_m'] + r['outer_radius_m']) / 2e6 for r in rows]
    fig, axes = plt.subplots(2, 2, figsize=(10, 7), layout='constrained')
    axes[0, 0].plot(radius, [r['density_kg_m3'] for r in rows], marker='.')
    axes[0, 0].set_ylabel('Density (kg/m³)')
    axes[0, 1].plot(radius, [r['temperature_K'] / 1e6 for r in rows], marker='.', color='tab:red')
    axes[0, 1].set_ylabel('Temperature (million K)')
    for key, label in [('gas_pressure_Pa', 'Gas'), ('radiation_pressure_Pa', 'Trapped radiation')]:
        axes[1, 0].semilogy(radius, [r[key] for r in rows], label=label, marker='.')
    axes[1, 0].set_ylabel('Pressure (Pa)')
    axes[1, 0].legend()
    axes[1, 1].plot(radius, [r['carbon_fraction'] for r in rows], marker='.', color='tab:green')
    axes[1, 1].set_ylabel('Carbon mass fraction')
    for ax in axes.flat:
        ax.set_xlabel('Radius (million m)')
        ax.grid(alpha=.25)
        ax.ticklabel_format(axis='x', style='plain')
    fig.suptitle(f"Synthetic stellar sphere — t = {rows[0]['time_s']:g} s\nShell-average states; not a calibrated stellar model")
    fig.savefig(args.output, dpi=180)
    plt.close(fig)
    print(args.output.resolve())


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Plot the measured CSV reports written by the surface-film contact tests."""
import csv
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

root = Path(__file__).resolve().parents[1] / 'docs'
with (root / 'film-self-contact-time-convergence.csv').open() as source:
    temporal = list(csv.DictReader(source))
with (root / 'film-self-contact-convergence.csv').open() as source:
    spatial = list(csv.DictReader(source))
fig, axes = plt.subplots(1, 2, figsize=(11, 4.5), constrained_layout=True)
axes[0].plot([float(r['dt_seconds']) * 1000 for r in temporal],
             [float(r['error_over_initial_thickness']) * 100 for r in temporal],
             'o-', color='#247ba0')
axes[0].set(xlabel='Шаг времени, мс', ylabel='Ошибка / исходная толщина, %',
            title='Сравнение с аналитическим решением')
counts = [int(r['triangles']) for r in spatial]
axes[1].plot(counts, [float(r['receiver_volume_m3']) * 1e6 for r in spatial],
             'o-', color='#247ba0')
axes[1].set(xlabel='Количество треугольников', ylabel='Объём на принимающем участке, мл',
            title='Равномерный контакт: сгущение сетки', ylim=(0, 0.01), xscale='log')
axes[1].set_xticks(counts, [str(n) for n in counts])
for ax in axes:
    ax.grid(alpha=0.2)
fig.suptitle('Два плоских участка: инженерная модель контактного обмена', fontsize=12)
fig.savefig(root / 'film-self-contact-validation.png', dpi=160)

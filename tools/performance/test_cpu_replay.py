import csv
import json
from pathlib import Path
import tempfile
import unittest
import paired_cpu_replay as replay


class ReplayReportTests(unittest.TestCase):
    def fixture(self, folder):
        for scenario in replay.SCENARIOS:
            for number in range(1, 6):
                for variant in ('before', 'after'):
                    path = folder / f'{scenario}-{number}-{variant}.csv'
                    value = 2. if variant == 'before' else 1.
                    with path.open('w') as output:
                        writer = csv.writer(output)
                        writer.writerow(['frame', 'fixed_ticks', 'total_cpu_us'])
                        writer.writerows((i, i % 2, value) for i in range(240))
                    report = {'gpu_created': False, 'measured_frames': 240, 'scenario': scenario,
                              'seed': 42, 'vertices': 3, 'geometry_blake3': 'g', 'indices_blake3': 'i',
                              'state_blake3': scenario, 'warmup_frames': 120, 'fixed_dt': 1/60,
                              'fixed_ticks_total': 180,
                              'stages': {'total_cpu': {'p50_us': value, 'p95_us': value, 'p99_us': value,
                                                       'max_us': value, 'sum_us': value*240}}}
                    path.with_suffix('.json').write_text(json.dumps(report))

    def test_complete_pairs(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            self.fixture(folder)
            report = replay.summarize(folder, {})
            self.assertTrue(report['cpu_gate'])
            self.assertEqual(len(report['summary']), 3)
            self.assertEqual(len((folder / 'pairs.csv').read_text().splitlines()), 31)

    def test_pooled_tail_cannot_hide_behind_median_run(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            self.fixture(folder)
            for number in (1, 2):
                path = folder / f'catchup-{number}-after.csv'
                with path.open('w') as output:
                    writer = csv.writer(output)
                    writer.writerow(['frame', 'fixed_ticks', 'total_cpu_us'])
                    writer.writerows((i, i % 2, 100. if i < 7 else 1.) for i in range(240))
                report = json.loads(path.with_suffix('.json').read_text())
                report['stages']['total_cpu'].update(p99_us=100., max_us=100., sum_us=933.)
                path.with_suffix('.json').write_text(json.dumps(report))
            report = replay.summarize(folder, {})
            stress = report['summary'][2]
            self.assertEqual(stress['ratio'], .5)
            self.assertEqual(stress['pooled_p99_ratio'], 50.)
            self.assertFalse(report['cpu_gate'])

    def test_rejects_missing_or_nonfinite_pose_clock_samples(self):
        for mutation in ('missing', 'nan', 'inactive'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                folder = Path(directory)
                self.fixture(folder)
                path = folder / 'motion-1-before.csv'
                report = json.loads(path.with_suffix('.json').read_text())
                report['pose_profiled'] = True
                path.with_suffix('.json').write_text(json.dumps(report))
                poses = [[i, 'true' if i % 2 else 'false', 1.] for i in range(240)]
                if mutation == 'missing':
                    poses.pop()
                elif mutation == 'nan':
                    poses[1][2] = 'nan'
                else:
                    poses[1][1] = 'false'
                with path.with_suffix('.pose.csv').open('w') as output:
                    writer = csv.writer(output)
                    writer.writerow(['frame', 'active', 'caller_cpu_us'])
                    writer.writerows(poses)
                with self.assertRaises(ValueError):
                    replay.read_phase(path)

    def test_rejects_invalidated_process_even_with_complete_csv(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            self.fixture(folder)
            path = folder / 'motion-1-before.csv'
            path.with_suffix('.run.json').write_text(json.dumps({'timing_valid': False}))
            with self.assertRaisesRegex(ValueError, 'lifecycle'):
                replay.read_phase(path)

    def test_rejects_changed_state_or_geometry(self):
        for field in ('state_blake3', 'geometry_blake3', 'fixed_dt'):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                folder = Path(directory)
                self.fixture(folder)
                path = folder / 'motion-1-after.json'
                report = json.loads(path.read_text())
                report[field] = 'changed'
                path.write_text(json.dumps(report))
                with self.assertRaises(ValueError):
                    replay.summarize(folder, {})

    def test_rejects_missing_rows_nonfinite_and_wrong_percentile(self):
        for mutation in ('missing', 'nan', 'percentile'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                folder = Path(directory)
                self.fixture(folder)
                path = folder / 'idle-1-before.csv'
                lines = path.read_text().splitlines()
                if mutation == 'missing':
                    path.write_text('\n'.join(lines[:-1]))
                elif mutation == 'nan':
                    lines[1] = '0,0,nan'
                    path.write_text('\n'.join(lines))
                else:
                    report = json.loads(path.with_suffix('.json').read_text())
                    report['stages']['total_cpu']['p99_us'] = 100
                    path.with_suffix('.json').write_text(json.dumps(report))
                with self.assertRaises(ValueError):
                    replay.read_phase(path)


if __name__ == '__main__':
    unittest.main()

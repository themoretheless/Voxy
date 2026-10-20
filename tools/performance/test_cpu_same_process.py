import tempfile
from pathlib import Path
import unittest
from paired_cpu_same_process import summarize


class CPUReportTests(unittest.TestCase):
    def fixture(self, folder):
        binary = folder / 'fixture-binary'
        binary.write_bytes(b'not executed')
        ticks, poses = [], []
        for round_number in range(1, 6):
            for variant in ('before', 'after'):
                before = variant == 'before'
                for species in ('false', 'true'):
                    ticks.append(f'TICK_PAIR,{round_number},{variant},{species},{10 if before else 8},{12 if before else 9},{species}-state\n')
                for layout in ('coherent', 'alternating', 'mixed_quarter'):
                    value = 8 if not before and layout == 'mixed_quarter' else 10
                    poses.append(f'KERNEL_PAIR,{round_number},{variant},{layout},{value}\n')
        (folder / 'ticks-same-process.log').write_text(''.join(ticks))
        (folder / 'pose-same-process.log').write_text(''.join(poses))
        return {'fixture': binary}

    def test_summary_retains_complete_pairs_and_tick_tails(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            binaries = self.fixture(folder)
            report = summarize(folder, binaries)
            self.assertTrue(report['controls_stable_within_3_percent'])
            dog = next(row for row in report['summary'] if row['case'] == 'tick_false')
            self.assertEqual(dog['ratio'], .8)
            self.assertEqual(dog['run_p99_us'], {'before': 12, 'after': 9})

    def test_state_mismatch_and_truncated_series_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            binaries = self.fixture(folder)
            path = folder / 'ticks-same-process.log'
            path.write_text(path.read_text().replace('false-state', 'corrupted-state', 1))
            with self.assertRaisesRegex(RuntimeError, 'State mismatch'):
                summarize(folder, binaries)
            self.fixture(folder)
            path = folder / 'pose-same-process.log'
            path.write_text(''.join(path.read_text().splitlines(keepends=True)[:-1]))
            with self.assertRaisesRegex(RuntimeError, 'five complete pairs'):
                summarize(folder, binaries)


if __name__ == '__main__':
    unittest.main()

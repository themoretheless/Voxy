import tempfile
import unittest
from pathlib import Path
import paired_palette_history_cpu as collector


class ReportTests(unittest.TestCase):
    def make_logs(self, folder):
        for filename, marker, cases in [('palette.log', 'PALETTE_PAIR', ['1', '2']),
                                        ('history.log', 'HISTORY_PAIR', ['repeat_large', 'change_large', 'change_small'])]:
            lines = [f'{marker},{n},{variant},{case},{1 if variant == "before" else .99}'
                     for case in cases for n in range(1, 6) for variant in ['before', 'after']]
            (folder / filename).write_text('\n'.join(lines) + '\n')

    def test_complete_report_and_control_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            self.make_logs(folder)
            report = collector.summarize(folder, {})
            self.assertEqual(len(report['summary']), 5)
            self.assertTrue(report['controls_within_3_percent'])
            self.assertEqual(len((folder / 'pairs.csv').read_text().splitlines()), 51)

    def test_rejects_duplicate_missing_and_nonfinite_rows(self):
        for mutation in ['duplicate', 'missing', 'nan']:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                folder = Path(directory)
                self.make_logs(folder)
                path = folder / 'palette.log'
                lines = path.read_text().splitlines()
                if mutation == 'duplicate':
                    lines.append(lines[0])
                elif mutation == 'missing':
                    lines.pop()
                else:
                    lines[0] = lines[0].rsplit(',', 1)[0] + ',nan'
                path.write_text('\n'.join(lines))
                with self.assertRaises(ValueError):
                    collector.summarize(folder, {})


if __name__ == '__main__':
    unittest.main()

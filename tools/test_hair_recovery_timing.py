import unittest
from analyze_hair_recovery_timing import analyze


def event(kind, phase, duration=None):
    return f'HAIR RECOVERY TIMING event={kind} phase={phase} iteration=None backtrack=None' + (f' elapsed_ms={duration} admission=unclassified' if duration is not None else '')


class RecoveryTimingTests(unittest.TestCase):
    def test_nested_time_is_counted_once_and_unrelated_logs_are_ignored(self):
        result = analyze('\n'.join([event('begin', 'recovery'), 'HAIR CONTACT SOC admitted=false', event('begin', 'refresh'), event('end', 'refresh', 7), event('end', 'recovery', 10)]))
        self.assertTrue(result['all_observed_scopes_closed'])
        self.assertEqual(result['completed_root_ms'], 10)
        self.assertEqual(result['phases']['recovery']['exclusive_ms'], 3)
        self.assertEqual(sum(p['exclusive_ms'] for p in result['phases'].values()), 10)

    def test_partial_run_reports_open_scopes_without_inventing_duration(self):
        result = analyze('\n'.join([event('begin', 'recovery'), event('begin', 'refresh'), event('end', 'refresh', 7), event('begin', 'projection')]))
        self.assertFalse(result['all_observed_scopes_closed'])
        self.assertEqual(result['completed_root_ms'], 0)
        self.assertEqual([p['phase'] for p in result['open_scopes']], ['recovery', 'projection'])
        self.assertNotIn('recovery', result['phases'])
        self.assertEqual(result['completed_root_phases'], {})

    def test_invalid_or_interleaved_traces_fail(self):
        traces = [event('end', 'recovery', 1), '\n'.join([event('begin', 'recovery'), event('begin', 'refresh'), event('end', 'recovery', 1)]), '\n'.join([event('begin', 'recovery'), event('end', 'recovery', -1)]), '\n'.join([event('begin', 'recovery'), event('begin', 'refresh'), event('end', 'refresh', 7), event('end', 'recovery', 6)]), 'HAIR RECOVERY TIMING garbage']
        for trace in traces:
            with self.subTest(trace=trace), self.assertRaises(ValueError):
                analyze(trace)


if __name__ == '__main__':
    unittest.main()

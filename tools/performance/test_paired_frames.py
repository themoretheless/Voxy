import unittest
from paired_frames import compare


def reports(before=10.0, after=9.0):
    return {s: {v: [{'p99_ms': t, 'skipped_frames': 0,
                     'adapter': 'device_type: IntegratedGpu, backend: Metal'} for _ in range(5)]
                for v, t in [('before', before), ('after', after)]}
            for s in ('idle', 'motion', 'water', 'street')}


class ComparisonTests(unittest.TestCase):
    def test_thresholds(self):
        self.assertTrue(compare(reports(), 'water')['candidate_accepted'])
        self.assertFalse(compare(reports(after=9.6), 'water')['candidate_accepted'])
        r = reports()
        for row in r['street']['after']:
            row['p99_ms'] = 10.31
        self.assertFalse(compare(r, 'water')['candidate_accepted'])

    def test_skipped_software_and_changed_adapter_are_invalid(self):
        for adapter in ['device_type: Cpu', 'unknown', 'device_type: DiscreteGpu, backend: Vulkan']:
            r = reports()
            r['idle']['after'][0]['adapter'] = adapter
            self.assertFalse(compare(r, 'water')['valid'])
        r = reports()
        r['idle']['before'][0]['skipped_frames'] = 1
        self.assertFalse(compare(r, 'water')['valid'])

    def test_budget(self):
        self.assertTrue(compare(reports(after=8), 'water')['target_met'])
        self.assertFalse(compare(reports(after=8.34), 'water')['target_met'])


if __name__ == '__main__':
    unittest.main()

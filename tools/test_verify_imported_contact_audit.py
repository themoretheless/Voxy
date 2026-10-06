#!/usr/bin/env python3
"""Qualification evidence must not accept partial or contaminated reports."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from verify_imported_contact_audit import read_audit, verify

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'artifacts/production-contact-candidate-2026-10-06/independent-energy-audit-480.json'


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.audit = json.loads(FIXTURE.read_text())

    def test_existing_full_audit(self):
        result = verify(self.audit)
        self.assertEqual(result['steps'], 480)
        self.assertEqual(len(result['bodies']), 4)
        # Raw balance remains visible; no subtraction hides it in that field.
        self.assertNotEqual(result['bodies'][0]['final_unadjusted_balance_j'], 0.)

    def test_contaminated_reports_rejected(self):
        cases = [
            ('steps', 479), ('steps', True),
            ('reported_defect_subtracted', True),
            ('reported_defect_subtracted', 'false'),
            ('requested_budget_per_body_j', float('inf')),
            ('requested_budget_per_body_j', 10**400),
            ('requested_budget_per_body_j', 0.048),
            ('scope', 'reported ledger only'),
            ('absolute_frame_error_j', [0., 0., 0.]),
            ('absolute_frame_error_j', [0.01, 0., 0., 0.]),
            ('rounding_allowance_j', [-1., 0., 0., 0.]),
            ('final_energy_receipts', [[float('nan')]*4]*4),
            ('final_energy_receipts', [[0., 0., 0., 1.]]*4),
            ('final_energy_receipts', [[1e308, -1e308, 0., 0.]]*4),
            ('final_energy_receipts', [[1e308, 0., 0., -1e308]]*4),
        ]
        for key, value in cases:
            with self.subTest(key=key, value=value):
                audit = copy.deepcopy(self.audit)
                audit[key] = value
                with self.assertRaises(ValueError):
                    verify(audit)

    def test_log_requires_terminal_success_after_audit(self):
        record = 'WIDE_IMPORTED_ENERGY_AUDIT ' + json.dumps(self.audit) + '\n'
        success = 'test result: ok. 1 passed; 0 failed; 0 ignored;\n'
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'tests.log'
            for bad in ['running 1 test\n', record, success+record,
                        record+'test result: FAILED. 0 passed; 1 failed;\n',
                        record+record+success]:
                path.write_text(bad)
                with self.assertRaises(ValueError):
                    read_audit(path)
            path.write_text(record+success)
            self.assertEqual(verify(read_audit(path))['steps'], 480)


if __name__ == '__main__':
    unittest.main()

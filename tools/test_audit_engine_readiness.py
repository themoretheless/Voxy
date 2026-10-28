"""Prevent ENG-00 source catalog checks from becoming false qualification claims."""
import copy
import json
import unittest
from pathlib import Path
import tempfile
from audit_engine_readiness import INVENTORY, validate


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.data = json.loads(INVENTORY.read_text())

    def test_current_inventory(self):
        validate(self.data)

    def test_duplicate_domain_rejected(self):
        self.data['domains'].append(copy.deepcopy(self.data['domains'][0]))
        with self.assertRaises(ValueError):
            validate(self.data)

    def test_missing_source_rejected(self):
        self.data['domains'][0]['evidence']['api'] = 'missing-eng00-file.rs'
        with self.assertRaises(ValueError):
            validate(self.data)

    def test_escape_and_absolute_source_rejected(self):
        for path in ('../Cargo.toml', '/etc/hosts'):
            with self.subTest(path=path):
                self.data['domains'][0]['evidence']['api'] = path
                with self.assertRaises(ValueError):
                    validate(self.data)

    def test_symlink_escape_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'external').symlink_to(INVENTORY)
            self.data['domains'][0]['evidence']['api'] = 'external'
            with self.assertRaises(ValueError):
                validate(self.data, root)

    def test_source_catalog_cannot_claim_runtime_qualification(self):
        self.data['domains'][0]['gates']['standalone'] = 'verified'
        with self.assertRaises(ValueError):
            validate(self.data)

    def test_missing_gate_or_acceptance_rejected(self):
        for key in ('gates', 'acceptance', 'owner', 'gap'):
            with self.subTest(key=key):
                invalid = copy.deepcopy(self.data)
                del invalid['domains'][0][key]
                with self.assertRaises(ValueError):
                    validate(invalid)


if __name__ == '__main__':
    unittest.main()

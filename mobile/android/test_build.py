"""Artifact gate tests; these do not substitute for an SDK build or device run."""
import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('android_build', Path(__file__).with_name('build.py'))
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


class LibraryValidation(unittest.TestCase):
    def validate(self, machine=183, alignment=16384, address=0, count=1):
        data = bytearray(120)
        data[:6] = b'\x7fELF\x02\x01'
        struct.pack_into('<HH', data, 16, 3, machine)
        struct.pack_into('<Q', data, 32, 64)
        struct.pack_into('<HH', data, 54, 56, count)
        struct.pack_into('<IIQQQQQQ', data, 64, 1, 5, 0, address, 0, 120, 120, alignment)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'library.so'
            path.write_bytes(data)
            build.validate_library(path)

    def test_valid_layout(self):
        self.validate()

    def test_wrong_architecture(self):
        with self.assertRaisesRegex(ValueError, 'AArch64'):
            self.validate(machine=62)

    def test_4k_alignment_rejected(self):
        with self.assertRaisesRegex(ValueError, '16 KiB'):
            self.validate(alignment=4096)

    def test_noncongruent_segment_rejected(self):
        with self.assertRaisesRegex(ValueError, '16 KiB'):
            self.validate(address=4096)

    def test_truncated_header_table_rejected(self):
        with self.assertRaisesRegex(ValueError, 'header table'):
            self.validate(count=2)


class DeviceSelection(unittest.TestCase):
    def select(self, output, requested=None):
        with patch.object(build.subprocess, 'check_output', return_value=output) as command:
            result = build.select_device(Path('/sdk/adb'), requested)
            command.assert_called_once_with(['/sdk/adb', 'devices'], text=True)
            return result

    def test_single_online_device(self):
        self.assertEqual(self.select('List of devices attached\none\tdevice\nother\toffline\n'), 'one')

    def test_explicit_device_with_multiple_online(self):
        self.assertEqual(self.select('one\tdevice\ntwo\tdevice\n', 'two'), 'two')

    def test_ambiguous_or_missing_default_fails(self):
        for output in ['', 'one\tdevice\ntwo\tdevice\n', 'one\tunauthorized\n']:
            with self.subTest(output=output), self.assertRaisesRegex(ValueError, 'one online'):
                self.select(output)

    def test_explicit_offline_unauthorized_or_missing_fails(self):
        for state in ['offline', 'unauthorized', 'missing']:
            with self.subTest(state=state), self.assertRaisesRegex(ValueError, 'offline or unauthorized'):
                self.select(f'one\t{state}\ntwo\tdevice\n', 'one')


if __name__ == '__main__':
    unittest.main()

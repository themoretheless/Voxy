import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import unittest
import cpu_replay_process


@unittest.skipUnless(os.name == 'posix', 'requires native wait statuses')
class ProcessTests(unittest.TestCase):
    def test_observes_normal_exit_and_actual_cpu(self):
        with tempfile.TemporaryDirectory() as folder:
            log = Path(folder)/'normal.log'
            result = cpu_replay_process.run([sys.executable, '-c', 'print(sum(range(100000)))'], log)
            self.assertTrue(result['timing_valid'])
            self.assertGreater(result['child_user_cpu_seconds'], 0)
            self.assertEqual(result['events'], [])

    def test_stopped_owned_sample_is_invalidated_and_killed_without_resume(self):
        with tempfile.TemporaryDirectory() as folder:
            log = Path(folder)/'stopped.log'
            with self.assertRaisesRegex(RuntimeError, 'stopped externally'):
                cpu_replay_process.run([sys.executable, '-c',
                    'import os,signal;os.kill(os.getpid(),signal.SIGSTOP)'], log)
            result = json.loads(log.with_suffix('.run.json').read_text())
            self.assertFalse(result['timing_valid'])
            self.assertEqual(result['returncode'], -signal.SIGKILL)
            self.assertEqual([e['kind'] for e in result['events']], ['stopped'])
            with self.assertRaises(ProcessLookupError):
                os.kill(result['pid'], 0)

"""No-manager regressions for noexec staging and bounded startup diagnostics."""
import json
import os
from pathlib import Path
import runpy
import stat
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

native = runpy.run_path(str(Path(__file__).with_name('native-proof.py')))


class NativeStartup(unittest.TestCase):
    def test_staging_is_separate_and_noexec_is_rejected(self):
        plan = native['plan']()
        self.assertEqual(plan['staging'], '/var/lib/buzz-service-proof-stage-DRYRUN')
        self.assertNotIn(plan['staging'], plan['state'])
        self.assertIn('--property=RootDirectory=' + plan['staging'] + '/rootfs', plan['start'])
        with patch.object(os, 'statvfs', return_value=SimpleNamespace(f_flag=os.ST_NOEXEC)):
            with self.assertRaisesRegex(RuntimeError, 'noexec'):
                native['require_executable_filesystem'](Path(plan['staging']))
        with patch.object(os, 'statvfs', return_value=SimpleNamespace(f_flag=0)):
            native['require_executable_filesystem'](Path(plan['staging']))

    def test_failed_command_keeps_bounded_diagnostics_before_cleanup(self):
        unit = 'buzz-service-proof-0123456789abcdef.service'
        proof = SimpleNamespace(bounded=Mock(return_value=(1, b'out' * 3000, b'Permission denied\n' * 1000)))
        with self.assertRaises(RuntimeError) as caught:
            native['checked_command'](proof, ['/usr/bin/systemd-run', '--unit=' + unit])
        error = json.loads(str(caught.exception).split(': ', 1)[1])
        self.assertEqual(error['exit_code'], 1)
        self.assertEqual(len(error['stdout']), 4096)
        self.assertEqual(len(error['stderr']), 4096)
        self.assertIn('Permission denied', error['stderr'])
        proof.bounded.return_value = (0, b'status=203/EXEC\n' * 2000, b'')
        with tempfile.TemporaryDirectory(prefix='buzz-native-startup.') as temporary:
            base = Path(temporary)
            native['write_failure'](proof, base, unit, caught.exception)
            path = base / 'failure.json'
            receipt = json.loads(path.read_text())
            self.assertEqual(receipt['unit'], unit)
            self.assertEqual(len(receipt['journal']), 16384)
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
            self.assertEqual(proof.bounded.call_args.args[0], ['/usr/bin/journalctl', '--unit=' + unit,
                             '--lines=40', '--no-pager', '--output=short-precise'])
            proof.bounded.side_effect = TimeoutError('synthetic timeout')
            native['write_failure'](proof, base, unit, caught.exception)
            self.assertEqual(json.loads(path.read_text())['journal_error'], 'synthetic timeout')


if __name__ == '__main__':
    unittest.main()

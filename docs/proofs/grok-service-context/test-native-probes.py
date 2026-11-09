"""M-1 regression: exercise the actual native descriptor writer without root."""
import json
import os
from pathlib import Path
import runpy
import stat
import tempfile
import unittest
from unittest.mock import patch

write_probes = runpy.run_path(str(Path(__file__).with_name('native-proof.py')))['write_probes']


class ProbePermissions(unittest.TestCase):
    def test_descriptor_under_private_umask(self):
        self.assertNotEqual(os.geteuid(), 0, 'run this regression without root')
        payload = {'uid_policy': 'service-owner', 'denied_files': ['/synthetic/operator']}
        previous = os.umask(0o077)
        try:
            with tempfile.TemporaryDirectory(prefix='buzz-native-permission.') as temporary:
                root = Path(temporary)
                (root / 'proof').mkdir()
                descriptor = root / 'proof/probes.json'
                write_probes(root, payload)
                self.assertEqual(json.loads(descriptor.read_text()), payload)
                self.assertEqual(stat.S_IMODE(descriptor.stat().st_mode), 0o444)
                # Falsification: suppress the real helper's chmod, then require
                # the exact permission assertion above to detect the original bug.
                descriptor.unlink()
                with patch.object(Path, 'chmod', return_value=None):
                    write_probes(root, payload)
                with self.assertRaises(AssertionError):
                    self.assertEqual(stat.S_IMODE(descriptor.stat().st_mode), 0o444)
        finally:
            os.umask(previous)


if __name__ == '__main__':
    unittest.main()

"""Run only as hash-verified bytes in the separately approved root command."""
import hashlib
import io
import os
from pathlib import Path
import secrets
import signal
import shutil
import subprocess
import sys
import tarfile

# argv: verified bootstrap path, prepared archive path, approved archive SHA-256.
if os.geteuid() != 0:
    raise SystemExit('root required for approved native service rehearsal')
if Path('/proc/1/comm').read_text().strip() != 'systemd':
    raise SystemExit('PID 1 is not systemd: native execution remains NOT RUN')
if len(sys.argv) != 4:
    raise SystemExit('expected bootstrap path, archive and pinned archive SHA')
with open(sys.argv[2], 'rb') as source:
    archive_bytes = source.read(256 * 1024 * 1024 + 1)
if len(archive_bytes) > 256 * 1024 * 1024 or hashlib.sha256(archive_bytes).hexdigest() != sys.argv[3]:
    raise SystemExit('archive size/hash mismatch')
if shutil.disk_usage('/run').free < len(archive_bytes) * 2 + 32 * 1024 * 1024:
    raise SystemExit('insufficient /run space')
def interrupted(signum, frame):
    raise KeyboardInterrupt('native bootstrap interrupted')

signal.signal(signal.SIGTERM, interrupted)
os.umask(0o077)
stage = Path('/run') / ('buzz-service-proof-' + secrets.token_hex(8))
stage.mkdir(mode=0o700)  # Exclusive create; never overwrite a pre-existing path.
try:
    with tarfile.open(fileobj=io.BytesIO(archive_bytes)) as archive:
        members = archive.getmembers()
        if any(not (entry.isfile() or entry.isdir()) for entry in members):
            raise RuntimeError('bundle contains non-file entries')
        archive.extractall(stage, filter='data')
    for path in stage.rglob('*'):
        path.chmod(0o755 if path.is_dir() or path.stat().st_mode & 0o111 else 0o444)
    child = subprocess.Popen(['/usr/bin/python3', '-I', '-S', str(stage / 'native-proof.py'),
                              '--run', 'isolated-native-offline-proof'],
                             env={'PATH': '/usr/bin:/bin'})
    try:
        status = child.wait()
    except BaseException:
        child.terminate()
        child.wait(timeout=120)  # Allow native finally to stop the unit and verify cleanup.
        raise
    if status:
        raise RuntimeError('native candidate failed; inspect preserved staging')
except BaseException:
    print('Preserved owned staging for inspection: ' + str(stage), file=sys.stderr)
    raise
else:
    shutil.rmtree(stage)
    print('Removed owned staging: ' + str(stage))

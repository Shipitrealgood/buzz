#!/usr/bin/python3
"""Prepare a pinned offline bundle; native execution requires separate approval."""
import argparse
import configparser
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import stat
import sys
import tarfile
import tempfile

HERE = Path(__file__).resolve().parent
PREFIX = 'buzz-service-proof-'
STAGE_PREFIX = 'buzz-service-proof-stage-'
CONFIRM = 'isolated-native-offline-proof'


def fixture():
    spec = importlib.util.spec_from_file_location('proof_fixture', HERE / 'run-proof.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def invocation(name, base):
    text = (HERE / 'buzz-service-proof.service').read_text().replace('buzz-service-proof', name)
    unit = configparser.ConfigParser(interpolation=None)
    unit.optionxform = str
    unit.read_string(text)
    properties = dict(unit['Service'])
    properties['RootDirectory'] = str(base / 'rootfs')
    command = properties.pop('ExecStart').split()
    argv = ['/usr/bin/systemd-run', '--unit=' + name + '.service',
            '--description=' + name, '--service-type=oneshot']
    argv += ['--property=' + key + '=' + value for key, value in properties.items() if key != 'Type']
    return argv + ['--'] + command


def plan(name=PREFIX + 'DRYRUN', base=None):
    base = base or Path('/var/lib') / (STAGE_PREFIX + name.removeprefix(PREFIX))
    unit = name + '.service'
    return {'status': 'PLAN ONLY; actual native service NOT RUN',
            'staging': str(base), 'state': ['/var/lib/' + name, '/var/lib/private/' + name],
            'unit': unit, 'start': invocation(name, base),
            'restart': ['/usr/bin/systemctl', 'restart', unit],
            'stop': ['/usr/bin/systemctl', 'stop', unit],
            'cleanup': 'stop owned unit; verify cgroup empty; reset-failed owned unit; remove only listed state and staging',
            'tests': ['first-start write/read/models/failure', 'restart read/models/owner mapping',
                      'forced unit exit 70', 'stopped cgroup empty'],
            'provider_modes': 'disabled', 'network': 'PrivateNetwork=yes'}


def bundle():
    if os.geteuid() == 0:
        raise RuntimeError('bundle preparation must run unprivileged')
    proof = fixture()
    with tempfile.TemporaryDirectory(prefix='buzz-proof-build.') as temporary:
        stage = Path(temporary)
        root = stage / 'rootfs'
        root.mkdir()
        for source, pin in proof.PINS.items():
            if proof.digest(source) != pin:
                raise RuntimeError('runtime pin mismatch')
        proof.build_rootfs(root)
        for source, destination in [(proof.ACP, 'runtime/buzz-acp'), (proof.GROK, 'runtime/grok')]:
            if proof.digest(root / destination) != proof.PINS[source]:
                raise RuntimeError('copied runtime pin mismatch')
        for name in ['native-proof.py', 'run-proof.py', 'context.py', 'buzz-service-proof.service']:
            shutil.copyfile(HERE / name, stage / name)
        manifest = {str(p.relative_to(stage)): proof.digest(p)
                    for p in sorted(stage.rglob('*')) if p.is_file()}
        (stage / 'bundle-manifest.json').write_text(json.dumps(manifest, indent=2))
        # Unique exclusive file; no caller-supplied output path or overwrite.
        fd, filename = tempfile.mkstemp(prefix='buzz-native-proof-', suffix='.tar', dir='/tmp')
        os.close(fd)
        with tarfile.open(filename, 'w') as archive:
            for path in sorted(stage.rglob('*')):
                archive.add(path, arcname=str(path.relative_to(stage)), recursive=False)
        print(json.dumps({'bundle': filename, 'sha256': proof.digest(Path(filename)),
                          'files': len(manifest), 'archive_bytes': Path(filename).stat().st_size,
                          'unpacked_bytes': sum(p.stat().st_size for p in stage.rglob('*') if p.is_file()),
                          'native': 'NOT RUN'}, indent=2))


def write_probes(root, probes):
    """Publish trusted synthetic metadata readable, but not writable, by DynamicUser."""
    descriptor = root / 'proof/probes.json'
    descriptor.write_text(json.dumps(probes))
    descriptor.chmod(0o444)


def require_executable_filesystem(path):
    if os.statvfs(path).f_flag & os.ST_NOEXEC:
        raise RuntimeError('proof staging filesystem is noexec: ' + str(path))


def checked_command(proof, args, expected=0):
    code, out, err = proof.bounded(args, timeout=30)
    if code != expected:
        raise RuntimeError('native command failed: ' + json.dumps({
            'command': args[:2], 'exit_code': code,
            'stdout': out[:4096].decode(errors='replace'),
            'stderr': err[:4096].decode(errors='replace')}))
    return out.decode().strip()


def write_failure(proof, base, unit, error):
    receipt = {'unit': unit, 'error': str(error)[:12288]}
    try:
        code, out, err = proof.bounded(['/usr/bin/journalctl', '--unit=' + unit,
                                       '--lines=40', '--no-pager', '--output=short-precise'], timeout=10)
        receipt.update(journal_exit_code=code, journal=out[:16384].decode(errors='replace'),
                       journal_error=err[:4096].decode(errors='replace'))
    except Exception as diagnostic_error:
        receipt['journal_error'] = str(diagnostic_error)[:4096]
    path = base / 'failure.json'
    path.write_text(json.dumps(receipt, indent=2))
    path.chmod(0o600)
    print('Bounded failure diagnostics: ' + str(path), file=sys.stderr)


def native():
    base = HERE
    if os.geteuid() != 0 or not re.fullmatch(STAGE_PREFIX + '[a-f0-9]{16}', base.name) or base.parent != Path('/var/lib'):
        raise RuntimeError('requires verified root-owned /var/lib staging bootstrap')
    name = PREFIX + base.name.removeprefix(STAGE_PREFIX)
    for path in [base, *base.parents, *base.rglob('*')]:
        info = path.lstat()
        if info.st_uid != 0 or info.st_mode & 0o022 or stat.S_ISLNK(info.st_mode):
            raise RuntimeError('untrusted staged path')
    require_executable_filesystem(base / 'rootfs')
    if shutil.disk_usage('/var/lib').free < 32 * 1024 * 1024:
        raise RuntimeError('insufficient private state space')
    if Path('/proc/1/comm').read_text().strip() != 'systemd':
        raise RuntimeError('native PID 1 must be systemd')
    proof = fixture()  # Only after root-owned immutable input validation.
    manifest = json.loads((base / 'bundle-manifest.json').read_text())
    for relative, wanted in manifest.items():
        if proof.digest(base / relative) != wanted:
            raise RuntimeError('staged bundle hash mismatch')
    def interrupted(signum, frame):
        raise KeyboardInterrupt('native proof interrupted')

    signal.signal(signal.SIGTERM, interrupted)
    unit = name + '.service'
    state = Path('/var/lib') / name
    private = Path('/var/lib/private') / name
    for path in [state, private]:
        if path.exists() or path.is_symlink():
            raise RuntimeError('state collision')

    def command(args, expected=0):
        return checked_command(proof, args, expected)

    def show(key):
        return command(['/usr/bin/systemctl', 'show', unit, '--property=' + key, '--value'])

    if show('LoadState') != 'not-found':
        raise RuntimeError('unit collision')
    result = plan(name, base)
    result['status'] = 'native test started'
    listeners = []
    attempted = False
    clean = False
    try:
        probes, listeners = proof.prepare_probes(base)
        probes['uid_policy'] = 'service-owner'
        write_probes(base / 'rootfs', probes)
        # Synthetic public canaries are readable to the service UID outside its rootfs.
        base.chmod(0o711)
        for directory in (base / 'rootfs').rglob('*'):
            if directory.is_dir():
                directory.chmod(0o755)
        (base / 'rootfs').chmod(0o755)
        attempted = True
        command(invocation(name, base))
        if show('Description') != name or show('ExecMainStatus') != '0' or show('ActiveState') != 'active':
            raise RuntimeError('first start did not complete')
        # Only the standard StateDirectory mapping is accepted; no arbitrary symlink traversal.
        if state.is_symlink() and state.resolve() != private:
            raise RuntimeError('unexpected StateDirectory mapping')
        if state.resolve() not in {state, private}:
            raise RuntimeError('unexpected StateDirectory target')
        receipt = state / 'home/native-receipt.json'
        first = json.loads(receipt.read_text())
        if len(first['starts']) != 1:
            raise RuntimeError('unexpected first-start receipt')
        command(['/usr/bin/systemctl', 'restart', unit])
        second = json.loads(receipt.read_text())
        if len(second['starts']) != 2:
            raise RuntimeError('restart did not preserve own state')
        result['service_receipt'] = second
        (state / 'fail-next').touch(exist_ok=False)
        code, _, _ = proof.bounded(['/usr/bin/systemctl', 'restart', unit], timeout=30)
        if code == 0 or show('ExecMainStatus') != '70' or show('Result') != 'exit-code':
            raise RuntimeError('forced unit failure was not observed')
        result['forced_failure'] = 70
    except BaseException as error:
        write_failure(proof, base, unit, error)
        raise
    finally:
        for listener in listeners:
            listener.close()
        if attempted:
            if show('Description') != name:
                raise RuntimeError('unit ownership uncertain; preserve resources')
            control_group = show('ControlGroup')
            command(['/usr/bin/systemctl', 'stop', unit])
            if control_group:
                if control_group != '/system.slice/' + unit:
                    raise RuntimeError('unexpected cgroup; preserve resources')
                events = Path('/sys/fs/cgroup' + control_group) / 'cgroup.events'
                if events.exists() and 'populated 1' in events.read_text():
                    raise RuntimeError('cgroup still populated; preserve resources')
            # Reset only our verified name; transient unit may already have unloaded.
            proof.bounded(['/usr/bin/systemctl', 'reset-failed', unit], timeout=10)
            if state.is_symlink():
                if state.resolve() != private:
                    raise RuntimeError('state mapping changed; preserve resources')
                state.unlink()
            elif state.exists():
                shutil.rmtree(state)
            if private.exists():
                if private.is_symlink():
                    raise RuntimeError('private state is a symlink; preserve resources')
                shutil.rmtree(private)
        clean = True
    result.update(status='PASS: native offline fixture only', cleanup='owned unit stopped, cgroup empty, state removed')
    if clean:
        print(json.dumps(result, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group()
    group.add_argument('--bundle', action='store_true')
    group.add_argument('--run', choices=[CONFIRM])
    args = parser.parse_args()
    if args.bundle:
        bundle()
    elif args.run:
        native()
    else:
        print(json.dumps(plan(), indent=2))


if __name__ == '__main__':
    main()

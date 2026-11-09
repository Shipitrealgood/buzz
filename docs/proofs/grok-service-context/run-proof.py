#!/usr/bin/python3
"""Finite synthetic proof; emits sanitized JSON and cleans its own temp tree."""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import selectors
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2] if len(HERE.parents) > 2 else HERE
ACP = Path('/home/bryce/services/buzz-agents/releases/paired-hosts-canary-723c7e849/netcup/bin/buzz-acp')
GROK = Path('/home/bryce/.grok/downloads/grok-1.0.70-linux-x86_64')
PINS = {ACP: 'fa3f1a36919d3d1c34fd523f40695495bb58cd00e7f6b52b62e59edc140341fb',
        GROK: '22d281c5964c5444147328096ba2a221f3f0874c6858e27c825656a80f6e58e9'}


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def copy_file(root, source, destination=None):
    dest = root / str(destination or source).lstrip('/')
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, dest)
    dest.chmod(source.stat().st_mode & 0o777)


def elf(root, source, destination=None):
    copy_file(root, source, destination)
    output = subprocess.check_output(['/usr/bin/ldd', str(source)], env={'PATH': '/usr/bin:/bin'}, text=True)
    for name in re.findall(r'(/[^\s()]+)', output):
        copy_file(root, Path(name))


def bounded(argv, timeout=15):
    """Bound both streams, time and whole namespace lifetime, including cancel."""
    def limits():
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        resource.setrlimit(resource.RLIMIT_FSIZE, (1048576, 1048576))
        resource.setrlimit(resource.RLIMIT_NOFILE, (128, 128))
    process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, env={'PATH': '/usr/bin:/bin', 'PROOF_POISON': 'synthetic'},
                               start_new_session=True, preexec_fn=limits)
    chunks = {process.stdout: bytearray(), process.stderr: bytearray()}
    selector = selectors.DefaultSelector()
    for stream in chunks:
        selector.register(stream, selectors.EVENT_READ)
    deadline = time.monotonic() + timeout
    try:
        while selector.get_map():
            if time.monotonic() >= deadline:
                raise TimeoutError('fixture deadline')
            for key, _ in selector.select(0.1):
                data = os.read(key.fd, 4096)
                if not data:
                    selector.unregister(key.fileobj)
                else:
                    chunks[key.fileobj].extend(data)
                    if sum(map(len, chunks.values())) > 65536:
                        raise RuntimeError('fixture output limit')
        return process.wait(timeout=1), bytes(chunks[process.stdout]), bytes(chunks[process.stderr])
    finally:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=3)
        selector.close()
        for stream in chunks:
            stream.close()


def build_rootfs(root):
    for path, dest in [(Path('/usr/bin/python3'), None), (Path('/usr/bin/git'), None),
                       (Path('/bin/sh'), '/bin/sh'), (Path('/usr/bin/env'), None), (ACP, '/runtime/buzz-acp')]:
        elf(root, path, dest)
    # A copied, pinned interpreter stdlib; no site customization, site packages or host config.
    stdlib = Path('/usr/lib/python3.13')
    (root / 'usr/lib/python3.13/lib-dynload').mkdir(parents=True)
    for path in stdlib.glob('*.py'):
        if path.name != 'sitecustomize.py':
            copy_file(root, path)
    for name in ['encodings', 'json', 're', 'collections', 'importlib', 'pathlib']:
        shutil.copytree(stdlib / name, root / str(stdlib).lstrip('/') / name,
                        ignore=shutil.ignore_patterns('__pycache__'))
    for name in ['_socket', 'select', 'fcntl', '_posixsubprocess', 'math', 'array', 'binascii', '_struct']:
        matches = list((stdlib / 'lib-dynload').glob(name + '.*.so'))
        for path in matches:
            elf(root, path)
    copy_file(root, GROK, '/runtime/grok')
    copy_file(root, HERE / 'context.py', '/proof/context.py')
    copy_file(root, REPO / 'crates/buzz-acp/tests/fixtures/task_agent.py', '/proof/task_agent.py')
    runtime = {'synthetic': {'harness': '/runtime/buzz-acp', 'command': '/usr/bin/python3',
                            'args': ['-S', '/proof/task_agent.py']},
               'grok': {'harness': '/runtime/buzz-acp', 'command': '/runtime/grok',
                        'args': ['agent', '--always-approve', 'stdio'], 'login': ['login', '--device-auth']}}
    (root / 'proof/runtime.json').write_text(json.dumps(runtime))
    for name in ['state/home', 'state/work', 'run', 'tmp', 'proc', 'dev']:
        (root / name).mkdir(parents=True, exist_ok=True)


def prepare_probes(base):
    listeners = []
    # Only synthetic files. Their world readability makes the mount boundary meaningful.
    denied = []
    for name in ['operator', 'sibling', 'non-home', 'machine-config']:
        path = base / name
        path.write_text('SYNTHETIC-CANARY')
        path.chmod(0o644)
        assert path.read_text() == 'SYNTHETIC-CANARY'
        denied.append(str(path))
    for family, address in [(socket.AF_INET, ('127.0.0.1', 0)),
                            (socket.AF_INET6, ('::1', 0)),
                            (socket.AF_UNIX, str(base / 'shared.sock')),
                            (socket.AF_UNIX, '\0buzz-service-proof-' + base.name)]:
        server = socket.socket(family, socket.SOCK_STREAM)
        listeners.append(server)
        server.bind(address)
        if family == socket.AF_UNIX and not str(address).startswith('\0'):
            Path(address).chmod(0o666)
        server.listen(16)
        server.settimeout(1)
        with socket.socket(family, socket.SOCK_STREAM) as client:
            client.connect(server.getsockname())
            accepted, _ = server.accept()
            accepted.close()
    probes = {'denied_files': denied, 'ipv4': listeners[0].getsockname(),
              'ipv6': listeners[1].getsockname(), 'unix': listeners[2].getsockname(),
              'abstract': listeners[3].getsockname()[1:].decode()}
    return probes, listeners


def main():
    os.umask(0o077)
    for path, expected in PINS.items():
        assert digest(path) == expected, 'runtime pin mismatch'
    result = {'scope': 'offline namespace proof; not native systemd or provider readiness',
              'pins': {str(p): h for p, h in PINS.items()},
              'source_inputs': {str(p): digest(p) for p in [HERE / 'run-proof.py', HERE / 'context.py',
                  REPO / 'crates/buzz-acp/tests/fixtures/task_agent.py', Path('/usr/bin/bwrap')]}, 'checks': []}
    listeners = []
    with tempfile.TemporaryDirectory(prefix='buzz-service-proof.') as temp:
        base = Path(temp)
        root = base / 'rootfs'
        root.mkdir()
        home = base / 'home'
        home.mkdir()
        (home / '.grok').mkdir()
        work = base / 'work'
        work.mkdir()
        probes, listeners = prepare_probes(base)
        denied = probes['denied_files']
        result['checks'].append('positive controls: four readable canaries and IPv4/IPv6/path/abstract listeners')
        build_rootfs(root)
        (root / 'proof/probes.json').write_text(json.dumps(probes))
        result['rootfs_manifest'] = {str(p.relative_to(root)): digest(p)
                                    for p in sorted(root.rglob('*')) if p.is_file()}
        def command(mode, ref='service-proof-agent', expose=False, share_network=False):
            args = ['/usr/bin/bwrap', '--unshare-all', '--die-with-parent', '--new-session',
                    '--clearenv', '--ro-bind', str(root), '/', '--proc', '/proc', '--dev', '/dev',
                    '--tmpfs', '/tmp', '--tmpfs', '/run', '--bind', str(home), '/state/home',
                    '--bind', str(work), '/state/work', '--chdir', '/state/work']
            if share_network:
                args += ['--share-net']
            if expose:
                args += ['--ro-bind', denied[0], denied[0]]
            for key, value in {'HOME': '/state/home', 'GROK_HOME': '/state/home/.grok',
                               'PATH': '/usr/bin:/bin:/runtime', 'XDG_CONFIG_HOME': '/state/home/.config',
                               'XDG_RUNTIME_DIR': '/run', 'LANG': 'C.UTF-8', 'LC_CTYPE': 'C.UTF-8'}.items():
                args += ['--setenv', key, value]
            return args + ['/usr/bin/python3', '-S', '/proof/context.py', ref, mode]
        for mode in ['write', 'read']:
            code, out, err = bounded(command(mode))
            assert code == 0, (mode, code, err.decode())
            result['checks'].append(json.loads(out))
        code, _, err = bounded(command('read', expose=True))
        assert code != 0 and b'canary readable' in err
        result['checks'].append('negative control: exposing operator canary fails the real boundary probe')
        code, _, err = bounded(command('read', share_network=True))
        assert code != 0 and b'outside listener reachable' in err
        result['checks'].append('negative control: sharing network fails the real boundary probe')
        code, out, err = bounded(command('models'))
        assert code == 0, ('models', code, err.decode())
        payload = json.loads(out)
        assert payload['agent']['name'] == 'task-test'
        assert payload['stable']['configOptions'][0]['currentValue'] == 'old'
        result['model_payload'] = payload
        result['checks'].append('real ACP models --json delegates to unchanged source fixture')
        for mode, ref, expected in [('fail', 'service-proof-agent', 70), ('read', 'wrong-agent', 64),
                                    ('shell', 'service-proof-agent', 64), ('login', 'service-proof-agent', 78),
                                    ('native-models', 'service-proof-agent', 78), ('conversation', 'service-proof-agent', 78)]:
            assert bounded(command(mode, ref))[0] == expected
        result['checks'].append('failure, unknown reference/mode and provider gates fail closed')
        hold = subprocess.Popen(command('hold'), env={'PATH': '/usr/bin:/bin'},
                                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                stderr=subprocess.DEVNULL, start_new_session=True)
        try:
            for _ in range(100):
                if (home / 'holding').exists():
                    break
                if hold.poll() is not None:
                    raise AssertionError('hold died')
                time.sleep(0.03)
            assert (home / 'holding').exists()
            # Namespace descendants are identified before kill without dumping unrelated command lines.
            descendants = []
            pending = [hold.pid]
            while pending:
                parent = pending.pop()
                children = Path(f'/proc/{parent}/task/{parent}/children')
                if children.exists():
                    found = [int(pid) for pid in children.read_text().split()]
                    descendants.extend(found)
                    pending.extend(found)
            assert len(descendants) >= 3, 'expected context and detached descendant'
            assert bounded(command('models'))[0] == 75
        finally:
            if hold.poll() is None:
                os.killpg(hold.pid, signal.SIGKILL)
            hold.wait(timeout=3)
        assert bounded(command('read'))[0] == 0
        for pid in descendants:
            status = Path('/proc') / str(pid) / 'stat'
            if status.exists():
                assert status.read_text().split(') ')[1].startswith('Z'), 'live descendant after cancellation'
        result['checks'].append('exclusive lock; cancellation kills namespace including setsid child; lock reusable')
        try:
            bounded(command('hold'), timeout=0.5)
        except TimeoutError:
            pass
        else:
            raise AssertionError('deadline did not terminate hold')
        assert bounded(command('read'))[0] == 0
        result['checks'].append('deadline terminates namespace and releases lock')
        for server in listeners:
            server.close()
        result['cleanup'] = 'processes reaped or dead; sockets closed; temporary root removed on return'
    assert not base.exists()
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()

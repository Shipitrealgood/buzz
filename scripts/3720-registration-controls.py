#!/usr/bin/env python3
"""Exactly five authorized disposable negative controls, with byte restoration."""
import hashlib
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

root = Path(__file__).resolve().parent.parent
os.chdir(root)
receipt = root / 'docs/verification/3720/controls'
receipt.mkdir(parents=True, exist_ok=True)
base = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
if subprocess.run(['git', 'diff', '--quiet', 'HEAD', '--', 'Cargo.lock', 'crates', 'schema']).returncode:
    raise SystemExit('commit the proof before destructive negative controls')

def replace(files, path, before, after):
    source = files.setdefault(path, (root / path).read_text())
    if source.count(before) != 1:
        raise RuntimeError(f'control anchor must occur once: {path}: {before}')
    files[path] = source.replace(before, after, 1)

def remove_scope_guards(files):
    for name in ['event', 'req', 'count', 'artifact_sync']:
        path = f'crates/buzz-relay/src/handlers/{name}.rs'
        source = (root / path).read_text()
        anchor = 'if conn.registry.generation().is_some() {'
        if source.count(anchor) != 1:
            raise RuntimeError(f'expected one handler scope guard: {path}')
        start = source.index(anchor)
        end = start + len(anchor)
        depth = 1
        while depth:
            if source[end] == '{': depth += 1
            if source[end] == '}': depth -= 1
            end += 1
        files[path] = source[:start] + source[end:]
    replace(files, 'crates/buzz-relay/src/handlers/host_registry.rs',
            'if !registry_req && !registry_event && c.registry.generation().is_none() {',
            'if !registry_req && !registry_event {')
    replace(files, 'crates/buzz-relay/src/handlers/host_registry.rs',
            'ctx.scopes.clear();',
            '/* negative control: retain ordinary NIP-42 scopes */')

def mutate(name):
    files = {}
    writer = 'crates/buzz-db/src/store/paired_hosts.rs'
    if name == 'scope':
        remove_scope_guards(files)
    elif name == 'membership':
        replace(files, writer, 'require(member(tx, owner).await?)?;', '/* negative control: owner membership omitted */')
    elif name == 'parent-generation':
        replace(files, writer, 'require(parent.as_slice() == b.get::<Vec<u8>, _>("ack_id"))?;', '/* negative control: accepted-ack parent omitted */')
    elif name == 'unique-live-binding':
        replace(files, writer, "AND count(*) FILTER (WHERE host_key = $3 AND state<>'dead') = 0", 'AND true /* negative control: host occupancy omitted */')
        replace(files, 'schema/tables/public/paired_host_bindings.sql',
                "CREATE UNIQUE INDEX idx_paired_host_bindings_live ON paired_host_bindings\n    (community_id, host_key) WHERE state IN ('active', 'fenced');",
                '-- negative control: unique live binding backstop omitted')
    elif name == 'live-reauthorization':
        replace(files, 'crates/buzz-db/src/store/paired_hosts/registry.rs',
                'owner_access(&mut tx, reader, allow).await?;',
                'if r.query.view != "live" { owner_access(&mut tx, reader, allow).await?; }')
    return files

cases = [
    ('scope', 'scoped_auth_rejects_wrong_scope_challenge_relay_and_delegation'),
    ('membership', 'owner_membership_wrong_parties_and_tenants'),
    ('parent-generation', 'parent_generation_and_operation_identity_are_enforced'),
    ('unique-live-binding', 'unique_live_binding_survives_competing_signed_host_acks'),
    ('live-reauthorization', 'live_delivery_reauthorizes_on_second_relay_node'),
]
report = {'source': base, 'controls': [], 'terminal_green': 'pending final restored suite'}

def digest(data): return hashlib.sha256(data).hexdigest()
def interrupted(*_): raise KeyboardInterrupt()
signal.signal(signal.SIGTERM, interrupted)

for name, test in cases:
    changed = mutate(name)
    originals = {path: (root / path).read_bytes() for path in changed}
    entry = {'name': name, 'test': test, 'source': base, 'files': {}, 'result': 'in progress'}
    report['controls'].append(entry)
    try:
        for path, source in changed.items():
            (root / path).write_text(source)
            entry['files'][path] = {'before': digest(originals[path]), 'mutated': digest(source.encode())}
        (receipt / 'index.json').write_text(json.dumps(report, indent=2) + '\n')
        started = time.monotonic()
        command = ['cargo', 'test', '-p', 'buzz-relay', '--lib', '--no-run', '--locked', '--offline', '--message-format=json']
        built = subprocess.run(command, capture_output=True, text=True)
        entry['build_exit'] = built.returncode
        if built.returncode:
            (receipt / f'{name}-build-failed.txt').write_text(built.stderr + built.stdout)
            raise RuntimeError(f'{name}: mutation failed compilation (not a valid negative receipt)')
        executables = [item['executable'] for line in built.stdout.splitlines()
                       if (item := json.loads(line)).get('reason') == 'compiler-artifact'
                       and item.get('target', {}).get('name') == 'buzz_relay'
                       and item.get('profile', {}).get('test') and item.get('executable')]
        if len(executables) != 1:
            raise RuntimeError(f'{name}: expected one actual relay lib test artifact')
        executable = str(Path(executables[0]).relative_to(root))
        full_test = f'handlers::host_registry::external_infra_registration_tests::external_infra_tests::{test}'
        command = ['scripts/3720-registration-proof.sh', 'run', executable, full_test,
                   '--exact', '--ignored', '--nocapture', '--test-threads=1']
        ran = subprocess.run(command, capture_output=True, text=True)
        output = ran.stdout + ran.stderr
        (receipt / f'{name}.txt').write_text(output)
        entry.update({'test_exit': ran.returncode, 'seconds': round(time.monotonic()-started, 2),
                      'command': command, 'build_command': ['cargo', 'test', '-p', 'buzz-relay', '--lib', '--no-run', '--locked', '--offline'],
                      'result': 'guard detected' if ran.returncode and 'running 1 test' in output
                      and 'test result: FAILED. 0 passed; 1 failed;' in output and 'panicked' in output else 'INVALID or survived'})
        if entry['result'] != 'guard detected':
            raise RuntimeError(f'{name}: expected one functional test failure; inspect receipt')
        print(f'{name}: guard removal detected by exactly one failing production-entry test', flush=True)
    finally:
        for path, original in originals.items():
            (root / path).write_bytes(original)
            entry['files'][path]['restored'] = digest((root / path).read_bytes())
            if entry['files'][path]['restored'] != entry['files'][path]['before']:
                raise RuntimeError(f'byte restoration failed: {path}')
        (receipt / 'index.json').write_text(json.dumps(report, indent=2) + '\n')
print('All five controls detected; all source bytes restored. Run one terminal green suite.', flush=True)

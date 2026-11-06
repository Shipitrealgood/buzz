#!/usr/bin/env python3
"""Supplement strict clippy with an auditable changed-line diagnostic gate."""
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
base = '723c7e849a65af766a7db8ae8c2b62821f7550b4'
raw = Path(sys.argv[1])
output = Path(sys.argv[2])

def git(*args):
    return subprocess.check_output(['git', *args], cwd=root, text=True)

changes = git('diff', '--unified=0', base, '--', '*.rs')
touched = {}
current = None
for line in changes.splitlines():
    if line.startswith('+++ b/'):
        current = line[6:]
    match = re.match(r'@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@', line)
    if match and current:
        start, count = int(match[1]), int(match[2] or 1)
        touched.setdefault(current, []).append((start, start + max(count, 1) - 1))
for path in git('ls-files', '--others', '--exclude-standard').splitlines():
    if path.endswith('.rs'):
        touched[path] = [(1, len((root / path).read_text().splitlines()))]

inventory = []
unexplained = []
for line in raw.read_text().splitlines():
    try:
        item = json.loads(line)
    except json.JSONDecodeError:
        unexplained.append(line)
        continue
    if item.get('reason') != 'compiler-message':
        continue
    msg = item['message']
    if msg['level'] not in ('warning', 'error'):
        continue
    spans = []
    intersects = False
    for span in msg['spans']:
        if not span.get('is_primary'):
            continue
        path = Path(span['file_name'])
        if path.is_absolute():
            try:
                name = str(path.relative_to(root))
            except ValueError:
                name = str(path)
        else:
            name = str(path)
        changed = any(span['line_start'] <= end and span['line_end'] >= start
                      for start, end in touched.get(name, []))
        intersects |= changed
        spans.append({'path': name, 'start': span['line_start'],
                      'end': span['line_end'], 'changed': changed})
    inventory.append({'level': msg['level'], 'code': (msg.get('code') or {}).get('code'),
                      'message': msg['message'], 'primary_spans': spans,
                      'classification': 'new-code' if intersects else 'baseline-or-dependency'})
failures = [d for d in inventory if d['level'] == 'error' or d['classification'] == 'new-code']
manifest = {path: hashlib.sha256((root / path).read_bytes()).hexdigest()
            for path in sorted(touched) if (root / path).is_file()}
report = {'base': base, 'head': git('rev-parse', 'HEAD').strip(),
          'scope': 'all primary spans; every new Rust file and all changed-line ranges',
          'strict_clippy': 'baseline-blocked; this is supplemental evidence only',
          'command': 'cargo clippy -p buzz-relay -p buzz-core -p buzz-auth -p buzz-hostd -p buzz-paired-owner -p buzz-db -p buzz-conformance -p buzz-ws-client --all-targets --locked --offline --message-format=json',
          'diff_sha256': hashlib.sha256(changes.encode()).hexdigest(),
          'changed_file_sha256': manifest, 'diagnostics': inventory,
          'unexplained_output': unexplained, 'new_or_error_count': len(failures),
          'result': 'PASS' if not failures and not unexplained else 'FAIL'}
output.write_text(json.dumps(report, indent=2) + '\n')
print(f"{report['result']}: {len(inventory)} diagnostics, {len(failures)} new/error, {len(unexplained)} unexplained output lines")
for diagnostic in failures:
    print(json.dumps(diagnostic))
sys.exit(bool(failures or unexplained))

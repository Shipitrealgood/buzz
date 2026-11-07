#!/usr/bin/env python3
"""Measure the complete staged proof, including new files, against exact pins."""
import json
import subprocess
from collections import defaultdict
from pathlib import Path

root = Path(__file__).resolve().parent.parent
base = '723c7e849a65af766a7db8ae8c2b62821f7550b4'
upstream = '88697f1c597cda5790d4c702520d0651c48496c0'

def diff(*args):
    output = subprocess.check_output(['git', 'diff', '--numstat', *args], cwd=root, text=True)
    return [{'path': p, 'additions': int(a), 'deletions': int(d)}
            for line in output.splitlines() for a, d, p in [line.split('\t', 2)]]

def totals(rows):
    a = sum(r['additions'] for r in rows)
    d = sum(r['deletions'] for r in rows)
    return {'additions': a, 'deletions': d, 'net': a-d}

def category(path):
    if path.startswith('docs/verification/'):
        if path.endswith(('.rs', '.py')):
            return 'verification probe source'
        return 'verification receipts/logs'
    if path.endswith('Cargo.lock'):
        return 'lockfile'
    if path.endswith('Cargo.toml'):
        return 'production manifests'
    if path.endswith('.rs'):
        if '/tests/' in path or path.endswith('_tests.rs'):
            return 'standalone tests/fixtures'
        if '#[cfg(test)]' in (root/path).read_text():
            return 'mixed production/inline tests'
        return 'production-only Rust'
    return 'docs/tooling'

rows = diff('--cached', base)
by_category = defaultdict(list)
by_package = defaultdict(list)
for row in rows:
    row['category'] = category(row['path'])
    by_category[row['category']].append(row)
    parts = row['path'].split('/')
    package = parts[1] if parts[0] == 'crates' else parts[0]
    by_package[package].append(row)
report = {
    'proof_base': base,
    'production_source_pin': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
    'measurement': 'complete staged tree, including new files; final commit identifies artifact pin',
    'command': 'git diff --cached --numstat 723c7e849a65af766a7db8ae8c2b62821f7550b4',
    'unit': 'physical text lines, not executable SLOC',
    'totals': totals(rows),
    'by_category': {k: totals(v) for k, v in sorted(by_category.items())},
    'by_package': {k: totals(v) for k, v in sorted(by_package.items())},
    'files': rows,
    'retired_paths': subprocess.check_output(['git', 'diff', '--cached', '--diff-filter=D', '--name-only', base], cwd=root, text=True).splitlines(),
    'upstream_context_only': {
        'upstream_base': upstream,
        'preexisting_difference_to_proof_base': totals(diff(upstream, base)),
        'combined_upstream_to_staged': totals(diff('--cached', upstream)),
        'meaning': 'includes pre-existing fork/catch-up work; not attributed to this registration proof'
    }
}
repair_base = 'b7576dc4ae1400d9688ddcc8b36b44d350703b96'
repair_rows = diff('--cached', repair_base)
repair_categories = defaultdict(list)
for row in repair_rows:
    row['category'] = category(row['path'])
    repair_categories[row['category']].append(row)
report['round1'] = {
    'base': repair_base,
    'totals': totals(repair_rows),
    'by_category': {k: totals(v) for k, v in sorted(repair_categories.items())},
    'files': repair_rows,
}
path = root/'docs/verification/3720/diff-measurement.json'
path.write_text(json.dumps(report, indent=2)+'\n')
for name, value in report['by_category'].items():
    print(name, value)

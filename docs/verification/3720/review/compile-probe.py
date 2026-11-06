import json, subprocess
from pathlib import Path
names={'buzz_core','buzz_db','nostr','tokio','sqlx','serde_json'}
libs={}
for line in Path('target/3720-review-build.json').read_text().splitlines():
    d=json.loads(line)
    n=d.get('target',{}).get('name')
    if n in names:
        for f in d.get('filenames',[]):
            if f.endswith('.rlib'): libs.setdefault(n,f)
assert set(libs)==names, libs
cmd=['rustc','--edition=2021','docs/verification/3720/review/pruning-probe.rs','-L','dependency=target/debug/deps','-o','target/3720-review-pruning-probe']
for n,f in libs.items(): cmd+=['--extern',n+'='+f]
subprocess.run(cmd,check=True)

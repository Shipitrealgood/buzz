"""Read-only continuity, exact diff accounting and receipt checks at the review pin."""
import ast, hashlib, json, re, subprocess
from pathlib import Path
base='723c7e849a65af766a7db8ae8c2b62821f7550b4'
head='f54cf2d8d6b1d3ca1bc52f591f655d867309ce69'
root=Path.cwd()
def git(*a): return subprocess.check_output(['git',*a])
def sha(b): return hashlib.sha256(b).hexdigest()
controls=json.loads(Path('docs/verification/3720/controls/index.json').read_text())
# Reconstruct the executor's exact transformations in memory against its source.
class Source:
    def __init__(self,p=''): self.p=p
    def __truediv__(self,p): return Source(p)
    def read_text(self): return git('show',controls['source']+':'+self.p).decode()
module=ast.parse(Path('scripts/3720-registration-controls.py').read_text())
functions=ast.Module(body=[n for n in module.body if isinstance(n,ast.FunctionDef) and n.name in {'replace','remove_scope_guards','mutate'}],type_ignores=[])
ns={'root':Source()};exec(compile(functions,'controls-functions-only','exec'),ns)
checks=[]
for control in controls['controls']:
    reconstructed=ns['mutate'](control['name'])
    paths=[]
    for p,h in control['files'].items():
        original=git('show',controls['source']+':'+p)
        assert sha(original)==h['before']==h['restored']
        assert sha(reconstructed[p].encode())==h['mutated']
        paths.append({'path':p,'original_and_restored_match':True,'exact_mutation_reconstructed':True,'unchanged_at_review_pin':git('show',head+':'+p)==original})
    log=Path('docs/verification/3720/controls/'+control['name']+'.txt').read_text()
    assert 'test result: FAILED. 0 passed; 1 failed;' in log
    checks.append({'name':control['name'],'paths':paths,'assertion_failure':re.findall(r'(?m)^assertion[^\n]*|^stale subscription leaked[^\n]*',log)})
raw=git('diff','--numstat',base,head).decode()
rows={p:(int(a),int(d)) for line in raw.splitlines() for a,d,p in [line.split('\t',2)]}
measurement=json.loads(Path('docs/verification/3720/diff-measurement.json').read_text())
assert set(rows)=={r['path'] for r in measurement['files']}
assert all(rows[r['path']]==(r['additions'],r['deletions']) for r in measurement['files'])
assert sum(a for a,d in rows.values())==measurement['totals']['additions']
assert sum(d for a,d in rows.values())==measurement['totals']['deletions']
clippy=json.loads(Path('docs/verification/3720/clippy-inventory.json').read_text())
assert all(sha(git('show',head+':'+p))==h for p,h in clippy['changed_file_sha256'].items())
spans={s['path'] for d in clippy['diagnostics'] for s in d['primary_spans']}
assert all(d['primary_spans'] for d in clippy['diagnostics'])
assert all(git('show',base+':'+p)==git('show',head+':'+p) for p in spans)
assert clippy['new_or_error_count']==0 and not clippy['unexplained_output']
out={'base':base,'head':head,'source_mutation':False,'controls_reconstructed':checks,
     'control_continuity':'Only paired_hosts.rs differs among mutated files: late fresh-unpair state check; membership, parent and occupancy anchors unchanged. Complete control assertion receipts inspected; scope kill precedes independent-member insertion. Two partial scope survivors get no kill credit.',
     'exact_numstat_matches_every_reported_file':True,'totals':measurement['totals'],'categories':measurement['by_category'],
     'rust_totals':{'added':sum(a for p,(a,d) in rows.items() if p.endswith('.rs')),'deleted':sum(d for p,(a,d) in rows.items() if p.endswith('.rs'))},
     'clippy':{'source_manifest_matches':True,'diagnostics':len(clippy['diagnostics']),'all_primary_spans_in_byte_identical_baseline_files':True,'new_or_error':0,'strict':'baseline-blocked, not green'},
     'negative_receipts':'Executor controls read/reconstructed, not rerun; reviewer pruning and subscription race separately measured.'}
Path('docs/verification/3720/review/evidence-audit.json').write_text(json.dumps(out,indent=2)+'\n')
print('Exact diff/classification, control transformations/restoration and supplemental clippy continuity verified.')

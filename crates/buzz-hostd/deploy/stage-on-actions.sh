#!/usr/bin/env bash
# One disposable Ubuntu VM. Run with sudo --preserve-env=GITHUB_ACTIONS,RUNNER_ENVIRONMENT
# after building the five indicated binaries and generating the test relay CA.
set -Eeuo pipefail
trap 'printf "Staging setup failed at line %s.\n" "$LINENO" >&2' ERR
if [[ ${GITHUB_ACTIONS:-} != true || ${RUNNER_ENVIRONMENT:-} != github-hosted || $EUID != 0 || $# -lt 3 || $# -gt 4 ]]; then
  echo 'requires root on a fresh GitHub-hosted runner: stage-on-actions.sh BIN_DIR CA_CRT RELAY_KEY_HEX' >&2
  exit 2
fi
bin_dir=$(realpath -- "$1")
ca=$(realpath -- "$2")
relay_key=$3
[[ $relay_key =~ ^[0-9a-f]{64}$ ]]
assets=$(cd -- "$(dirname -- "$0")" && pwd)
repo=$(cd -- "$assets/../../.." && pwd)
for name in buzz-hostd buzz-host-activate buzz-host-runtime buzz-acp buzz; do
  [[ -x "$bin_dir/$name" && ! -e "/usr/local/libexec/paired-$name" ]]
done
getent passwd buzz-hostd >/dev/null && { echo 'host service user already exists' >&2; exit 1; }
for path in /etc/buzz-hostd /var/lib/buzz-hostd /var/lib/buzz-paired-units /usr/local/share/ca-certificates/buzz-paired-staging.crt \
    /usr/local/libexec/buzz-hostd /usr/local/libexec/buzz-host-activate /usr/local/libexec/buzz-host-runtime \
    /usr/local/libexec/paired-fixture.mjs /etc/systemd/system/buzz-hostd.service \
    /etc/systemd/system/buzz-hostd-activate.socket /etc/systemd/system/buzz-hostd-activate@.service /etc/systemd/system/buzz-paired-agent@.service; do
  [[ ! -e "$path" ]] || { echo "existing asset: $path" >&2; exit 1; }
done
useradd --system --user-group --home-dir /var/lib/buzz-hostd --shell /usr/sbin/nologin buzz-hostd
install -d -o buzz-hostd -g buzz-hostd -m 0700 /var/lib/buzz-hostd
install -d -m 0755 /etc/buzz-hostd /usr/local/libexec
for name in buzz-hostd buzz-host-activate buzz-host-runtime; do
  install -m 0755 "$bin_dir/$name" "/usr/local/libexec/$name"
done
install -m 0755 "$bin_dir/buzz-acp" /usr/local/libexec/paired-buzz-acp
install -m 0755 "$bin_dir/buzz" /usr/local/libexec/paired-buzz
# The fixture directly exercises authenticated chat reply. Its executable and
# absolute CLI reference are installed by the test operator and covered by the digest.
sed -e '1c#!/usr/bin/node' -e 's|const cli = process.env.BUZZ_E2E_CLI_BIN;|const cli = "/usr/local/libexec/paired-buzz";|' \
  "$repo/desktop/tests/e2e/fixtures/fake-acp-agent.mjs" > /usr/local/libexec/paired-fixture.mjs
chmod 0755 /usr/local/libexec/paired-fixture.mjs
test -x /usr/bin/node || { echo "Fixture needs the system Node runtime" >&2; exit 1; }
install -m 0644 "$ca" /usr/local/share/ca-certificates/buzz-paired-staging.crt
update-ca-certificates >/dev/null
python3 - "$relay_key" "${4:-https://localhost:39553}" <<'PY'
import hashlib, json, pathlib, sys
def exe(name):
    path = pathlib.Path('/usr/local/libexec', name)
    return {'path':str(path), 'sha256':hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()}
catalog = {'community':sys.argv[2], 'label':'Staging Linux', 'capacity':4, 'relay_key':sys.argv[1], 'runtimes':{
    'local/fixture':{'harness':exe('paired-buzz-acp'), 'adapter':exe('paired-fixture.mjs'), 'args':[], 'credential_profile':'none'}}}
pathlib.Path('/etc/buzz-hostd/catalog.json').write_text(json.dumps(catalog)+'\n')
PY
for unit in buzz-hostd.service buzz-hostd-activate.socket buzz-hostd-activate@.service buzz-paired-agent@.service; do
  install -m 0644 "$assets/$unit" "/etc/systemd/system/$unit"
done
systemd-analyze verify /etc/systemd/system/buzz-hostd.service /etc/systemd/system/buzz-hostd-activate.socket \
  /etc/systemd/system/buzz-hostd-activate@.service /etc/systemd/system/buzz-paired-agent@.service
runuser -u buzz-hostd -- /usr/local/libexec/buzz-hostd --state-dir /var/lib/buzz-hostd init
systemctl daemon-reload
systemctl enable --now buzz-hostd-activate.socket buzz-hostd.service
systemctl is-active --quiet buzz-hostd-activate.socket buzz-hostd.service
runuser -u buzz-hostd -- /usr/local/libexec/buzz-hostd --state-dir /var/lib/buzz-hostd status

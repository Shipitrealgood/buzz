#!/usr/bin/env bash
# Narrow disposable fixture only; not a distributable installer.
set -euo pipefail
[[ ${GITHUB_ACTIONS:-} == true && ${RUNNER_ENVIRONMENT:-} == github-hosted && $EUID == 0 && $# == 3 ]] || exit 2
bin_dir=$(realpath -- "$1")
ca=$(realpath -- "$2")
relay_key=$3
[[ $relay_key =~ ^[0-9a-f]{64}$ ]]
assets=$(cd -- "$(dirname -- "$0")" && pwd)
for path in /etc/buzz-host-registration /var/lib/buzz-host-registration /var/lib/buzz-host-registration-root \
  /usr/local/libexec/buzz-hostd /usr/local/libexec/buzz-host-activate /usr/local/libexec/buzz-registration-probe \
  /home/buzz-existing-agent /var/lib/buzz-existing-agent /etc/buzz-hostd /etc/systemd/system/buzz-host-registration.service; do
  [[ ! -e "$path" ]] || { echo "refusing existing fixture path: $path" >&2; exit 1; }
done
! getent passwd buzz-host-registration >/dev/null
useradd --system --user-group --home-dir /var/lib/buzz-host-registration --shell /usr/sbin/nologin buzz-host-registration
install -d -o buzz-host-registration -g buzz-host-registration -m 0700 /var/lib/buzz-host-registration
install -d -m 0755 /var/lib/buzz-host-registration-root /etc/buzz-host-registration /usr/local/libexec
install -m 0755 "$bin_dir/buzz-hostd" /usr/local/libexec/buzz-hostd
install -m 0755 "$bin_dir/examples/registration_isolation_probe" /usr/local/libexec/buzz-registration-probe
install -m 0644 "$ca" /etc/buzz-host-registration/relay-ca.pem
printf 'BUZZ_REGISTRY_COMMUNITY=https://localhost:39553\nBUZZ_REGISTRY_RELAY_KEY=%s\n' "$relay_key" > /etc/buzz-host-registration/controller.conf
chmod 0644 /etc/buzz-host-registration/controller.conf
# World-readable/executable canaries prove the mount boundary, not just DAC.
install -d -m 0755 /home/buzz-existing-agent /var/lib/buzz-existing-agent /etc/buzz-hostd/providers
for path in /home/buzz-existing-agent/credential /var/lib/buzz-existing-agent/token /etc/buzz-hostd/providers/synthetic.json; do
  printf 'synthetic fixture canary\n' > "$path"; chmod 0644 "$path"
done
printf '#!/bin/sh\nexit 0\n' > /usr/local/libexec/buzz-host-activate
chmod 0755 /usr/local/libexec/buzz-host-activate
# Positive controls: the service UID can access these on the ordinary fixture
# filesystem. The actual unit must remove that access through its sandbox.
runuser -u buzz-host-registration -- test -r /home/buzz-existing-agent/credential
runuser -u buzz-host-registration -- test -r /var/lib/buzz-existing-agent/token
runuser -u buzz-host-registration -- test -r /etc/buzz-hostd/providers/synthetic.json
runuser -u buzz-host-registration -- /usr/local/libexec/buzz-host-activate
install -m 0644 "$assets/buzz-host-registration.service" /etc/systemd/system/buzz-host-registration.service
install -d -m 0755 /etc/systemd/system/buzz-host-registration.service.d
cat > /etc/systemd/system/buzz-host-registration.service.d/probe.conf <<'DROPIN'
[Service]
BindReadOnlyPaths=/usr/local/libexec/buzz-registration-probe
ExecPaths=+/usr/local/libexec/buzz-registration-probe
ExecStartPre=/usr/local/libexec/buzz-registration-probe
DROPIN
systemd-analyze verify /etc/systemd/system/buzz-host-registration.service
runuser -u buzz-host-registration -- /usr/local/libexec/buzz-hostd --state-dir /var/lib/buzz-host-registration registration init
systemctl daemon-reload
systemctl start buzz-host-registration.service
systemctl is-active --quiet buzz-host-registration.service

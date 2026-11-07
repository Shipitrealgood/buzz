"""Run on a disposable GitHub-hosted Linux VM with PostgreSQL and Redis services."""
import errno
import json
import os
from pathlib import Path
import pty
import secrets
import select
import ssl
import subprocess
import tempfile
import time
from urllib.request import Request, urlopen

from cryptography.hazmat.primitives.asymmetric import ec

ROOT = Path(__file__).resolve().parents[2]
BIN = ROOT / "target/debug"
ORIGIN = "https://localhost:39553"


def output(args, **kwargs):
    return subprocess.check_output(argv(args), text=True, timeout=220, **kwargs)


def argv(args):
    return [str(item) for item in args]


def run(args, **kwargs):
    subprocess.run(argv(args), check=True, timeout=240, **kwargs)


def eventually(probe, timeout=90):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            result = probe()
            if result:
                return result
        except (ConnectionError, OSError):
            pass
        time.sleep(1)
    raise RuntimeError("staging condition timed out")


def capture_offer():
    master, slave = pty.openpty()
    try:
        command = ["sudo", "-u", "buzz-host-registration", "/usr/local/libexec/buzz-hostd", "--state-dir",
                   "/var/lib/buzz-host-registration", "registration", "offer", "--community", ORIGIN]
        child = subprocess.Popen(command, stdout=slave)
        os.close(slave)
        try:
            data = bytearray()
            deadline = time.monotonic() + 15
            while True:
                remaining = deadline - time.monotonic()
                if remaining <= 0 or not select.select([master], [], [], remaining)[0]:
                    raise RuntimeError("private offer timed out")
                try:
                    chunk = os.read(master, 4096)
                except OSError as error:
                    if error.errno == errno.EIO:
                        break
                    raise
                if not chunk:
                    break
                data.extend(chunk)
                if len(data) > 4096:
                    raise RuntimeError("offer exceeded size limit")
            if child.wait(timeout=5):
                raise RuntimeError("host did not issue a private offer")
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=5)
        return json.loads(data.decode().strip())
    finally:
        os.close(master)


def main():
    if os.environ.get("GITHUB_ACTIONS") != "true" or os.environ.get("RUNNER_ENVIRONMENT") != "github-hosted":
        raise SystemExit("requires a disposable GitHub-hosted runner")
    # Refuse any pre-existing fixture resource before entering cleanup ownership.
    for path in ["/etc/buzz-host-registration", "/var/lib/buzz-host-registration",
                 "/var/lib/buzz-host-registration-root", "/usr/local/libexec/buzz-hostd",
                 "/usr/local/libexec/buzz-host-activate", "/usr/local/libexec/buzz-registration-probe",
                 "/home/buzz-existing-agent", "/var/lib/buzz-existing-agent", "/etc/buzz-hostd",
                 "/etc/systemd/system/buzz-host-registration.service",
                 "/etc/systemd/system/buzz-host-registration.service.d"]:
        if subprocess.run(["sudo", "test", "-e", path]).returncode == 0:
            raise SystemExit(f"refusing pre-existing fixture resource: {path}")
    if subprocess.run(["getent", "passwd", "buzz-host-registration"], stdout=subprocess.DEVNULL).returncode == 0:
        raise SystemExit("refusing pre-existing registration account")
    os.umask(0o077)
    with tempfile.TemporaryDirectory(prefix="buzz-host-verify-") as temp:
        stage = Path(temp)
        device = stage / "owner"
        device.mkdir(mode=0o700)
        driver = BIN / "examples/registration"
        owner = output([driver, device, ORIGIN, "init"]).strip()
        if len(owner) != 64:
            raise RuntimeError("unexpected owner identity")
        relay_keys = ec.generate_private_key(ec.SECP256K1()).private_numbers()
        cert = stage / "ca.crt"
        run(["openssl","req","-x509","-newkey","rsa:2048","-nodes","-keyout",stage/"ca.key",
             "-out",cert,"-subj","/CN=Disposable Buzz Pairing Test","-days","1",
             "-addext","basicConstraints=critical,CA:TRUE"], stderr=subprocess.DEVNULL)
        run(["openssl","req","-new","-newkey","rsa:2048","-nodes","-keyout",stage/"server.key",
             "-out",stage/"server.csr","-subj","/CN=localhost"], stderr=subprocess.DEVNULL)
        ext = stage / "server.ext"
        ext.write_text("subjectAltName=DNS:localhost\nbasicConstraints=critical,CA:FALSE\nextendedKeyUsage=serverAuth\n")
        run(["openssl","x509","-req","-in",stage/"server.csr","-CA",cert,"-CAkey",stage/"ca.key",
             "-CAcreateserial","-out",stage/"server.crt","-days","1","-extfile",ext],stderr=subprocess.DEVNULL)
        env = dict(os.environ, RELAY_URL="wss://localhost:39553", RELAY_OWNER_PUBKEY=owner,
                   BUZZ_RELAY_PRIVATE_KEY=f"{relay_keys.private_value:064x}",
                   BUZZ_REQUIRE_AUTH_TOKEN="true", BUZZ_REQUIRE_RELAY_MEMBERSHIP="true",
                   BUZZ_ALLOW_NIP_OA_AUTH="true", BUZZ_PAIRED_HOSTS_V1="false", BUZZ_HOST_REGISTRY_V2="true",
                   BUZZ_BIND_ADDR="127.0.0.1:39552", BUZZ_HEALTH_PORT="39554", BUZZ_METRICS_PORT="39555",
                   RUST_LOG="warn", BUZZ_AUTO_MIGRATE="true",
                   BUZZ_GIT_CONFORMANCE_PROBE="false", BUZZ_GIT_REPO_PATH=str(stage/"repos"),
                   BUZZ_GIT_PACK_CACHE_PATH=str(stage/"pack-cache"),
                   BUZZ_GIT_HOOK_HMAC_SECRET=secrets.token_hex(32))
        processes = []
        try:
            processes.append(subprocess.Popen([BIN/"buzz-relay"],env=env,cwd=stage,
                                              stdout=subprocess.DEVNULL,stderr=(stage/"relay.log").open("w")))
            processes.append(subprocess.Popen(argv(["python3",ROOT/"scripts/paired-hosts/loopback_tls.py",
                "--cert",stage/"server.crt","--key",stage/"server.key","--https-port",39553,"--relay-port",39552])))
            tls = ssl.create_default_context(cafile=str(cert))
            def capability():
                with urlopen(Request(ORIGIN+"/",headers={"Accept":"application/nostr+json"}),context=tls,timeout=4) as response:
                    return json.load(response).get("host_registry", {}).get("versions") == [2]
            eventually(capability)
            run(["sudo", "--preserve-env=GITHUB_ACTIONS,RUNNER_ENVIRONMENT", "bash",
                 ROOT/"crates/buzz-hostd/deploy/stage-registration-on-actions.sh", BIN, cert, f"{relay_keys.public_numbers.x:064x}"])
            owner_env = dict(os.environ, BUZZ_REGISTRATION_CA=str(cert))
            def drive(op, *args, payload=None):
                return json.loads(output([driver, device, ORIGIN, op, *args], env=owner_env,
                    input=json.dumps(payload) if payload is not None else None))
            if drive("hosts") != []:
                raise RuntimeError("fresh owner registry was not empty")
            offer = capture_offer()
            if offer["v"] != 2:
                raise RuntimeError("executable did not issue a v2 invitation")
            if drive("pair", payload=offer)["state"] != "pair-pending":
                raise RuntimeError("missing pending registration")
            def registered():
                cards = drive("hosts")
                return cards if len(cards) == 1 and cards[0]["state"] == "paired" else None
            cards = eventually(registered)
            if cards[0]["fresh"] or cards[0]["runtimes"]:
                raise RuntimeError("registration advertised live/runtime capability")
            print("PASS executable/service registered through isolated TLS relay", flush=True)
            run(["sudo", "systemctl", "restart", "buzz-host-registration.service"])
            (device/"owner.sqlite").unlink()
            recovered = eventually(registered)
            if recovered[0]["scope"] != cards[0]["scope"]:
                raise RuntimeError("restart/journal recovery changed binding")
            print("PASS service restart and independent journal recovery preserve binding", flush=True)
            second = subprocess.run(argv(["sudo", "-u", "buzz-host-registration", "/usr/local/libexec/buzz-hostd",
                "--state-dir", "/var/lib/buzz-host-registration", "registration", "run", "--community", ORIGIN,
                "--relay-key", f"{relay_keys.public_numbers.x:064x}", "--once"]), capture_output=True, text=True, timeout=15)
            if second.returncode == 0 or "another host controller" not in second.stderr:
                raise RuntimeError("second controller was not excluded")
            if drive("unpair", offer["generation"])["state"] != "unpair-pending":
                raise RuntimeError("revocation did not retain pending state")
            eventually(lambda: any(c["state"] == "unpaired" for c in drive("hosts")))
            run(["sudo", "systemctl", "restart", "buzz-host-registration.service"])
            if not any(c["state"] == "unpaired" for c in drive("hosts")):
                raise RuntimeError("restart revived revoked binding")
            print("PASS single controller, revoke and revoked restart", flush=True)
            probe = output(["sudo", "cat", "/var/lib/buzz-host-registration/isolation-probe"]).strip()
            if probe != "dedicated unprivileged service; credential/home/helper denied":
                raise RuntimeError("missing actual service isolation receipt")
            run(["systemctl", "is-active", "--quiet", "buzz-host-registration.service"])
            pid = output(["systemctl", "show", "-p", "MainPID", "--value", "buzz-host-registration.service"]).strip()
            if int(output(["ps", "-o", "uid=", "-p", pid]).strip()) == 0:
                raise RuntimeError("controller is privileged")
            print("PASS actual service identity and filesystem/helper isolation", flush=True)
        finally:
            # Hostd emits generic status/errors, never an invitation or private key.
            subprocess.run(["sudo", "journalctl", "-u", "buzz-host-registration.service", "--no-pager", "-n", "60"], check=False)
            # Everything belongs to this disposable VM; no existing-agent units are touched.
            subprocess.run(["sudo", "systemctl", "stop", "buzz-host-registration.service"], check=False)
            for process in reversed(processes):
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
            subprocess.run(["sudo", "rm", "-rf", "/var/lib/buzz-host-registration",
                "/var/lib/buzz-host-registration-root", "/etc/buzz-host-registration",
                "/etc/systemd/system/buzz-host-registration.service",
                "/etc/systemd/system/buzz-host-registration.service.d",
                "/home/buzz-existing-agent", "/var/lib/buzz-existing-agent", "/etc/buzz-hostd",
                "/usr/local/libexec/buzz-hostd", "/usr/local/libexec/buzz-host-activate",
                "/usr/local/libexec/buzz-registration-probe"], check=False)
            subprocess.run(["sudo", "systemctl", "daemon-reload"], check=False)
            subprocess.run(["sudo", "userdel", "buzz-host-registration"], check=False)
            for path in ["/var/lib/buzz-host-registration", "/var/lib/buzz-host-registration-root",
                         "/etc/buzz-host-registration", "/usr/local/libexec/buzz-hostd",
                         "/usr/local/libexec/buzz-host-activate", "/usr/local/libexec/buzz-registration-probe",
                         "/home/buzz-existing-agent", "/var/lib/buzz-existing-agent", "/etc/buzz-hostd"]:
                if subprocess.run(["sudo", "test", "-e", path]).returncode == 0:
                    raise RuntimeError(f"fixture cleanup incomplete: {path}")
            if subprocess.run(["getent", "passwd", "buzz-host-registration"], stdout=subprocess.DEVNULL).returncode == 0:
                raise RuntimeError("fixture service account remains")
            print("PASS fixture processes, account and credential/state canaries removed", flush=True)


if __name__ == "__main__":
    main()

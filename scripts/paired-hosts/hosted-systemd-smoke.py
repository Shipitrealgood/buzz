"""Run on a disposable GitHub-hosted Linux VM with PostgreSQL and Redis services."""
import errno
import json
import os
from pathlib import Path
import pty
import secrets
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
        command = ["sudo", "-u", "buzz-hostd", "/usr/local/libexec/buzz-hostd", "--state-dir",
                   "/var/lib/buzz-hostd", "offer", "--community", ORIGIN]
        with subprocess.Popen(command, stdout=slave) as child:
            os.close(slave)
            data = bytearray()
            try:
                while chunk := os.read(master, 4096):
                    data.extend(chunk)
                    if len(data) > 4096:
                        raise RuntimeError("offer exceeded size limit")
            except OSError as error:
                if error.errno != errno.EIO:
                    raise
            if child.wait(timeout=10):
                raise RuntimeError("host did not issue a private offer")
        return json.loads(data.decode().strip())
    finally:
        os.close(master)


def main():
    if os.environ.get("GITHUB_ACTIONS") != "true" or os.environ.get("RUNNER_ENVIRONMENT") != "github-hosted":
        raise SystemExit("requires a disposable GitHub-hosted runner")
    os.umask(0o077)
    with tempfile.TemporaryDirectory(prefix="buzz-host-verify-") as temp:
        stage = Path(temp)
        device = stage / "owner"
        device.mkdir(mode=0o700)
        driver = BIN / "examples/staging"
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
                   BUZZ_ALLOW_NIP_OA_AUTH="true", BUZZ_PAIRED_HOSTS_V1="true",
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
                    return json.load(response).get("paired_hosts_v1")
            eventually(capability)
            run(["sudo","--preserve-env=GITHUB_ACTIONS,RUNNER_ENVIRONMENT","bash",
                 ROOT/"crates/buzz-hostd/deploy/stage-on-actions.sh",BIN,cert,f"{relay_keys.public_numbers.x:064x}"])
            owner_env = dict(os.environ, BUZZ_PAIRED_STAGING_CA=str(cert))
            def drive(op, *args, payload=None):
                result=output([driver,device,ORIGIN,op,*args],env=owner_env,
                              input=json.dumps(payload) if payload is not None else None)
                return [json.loads(line) for line in result.splitlines() if line]
            offer = capture_offer()
            drive("pair", payload=offer)
            if len(drive("hosts")) != 1:
                raise RuntimeError("expected one paired host in owner inventory")
            print("paired and restored one host on the owner device", flush=True)
            chat_env = dict(os.environ, BUZZ_RELAY_URL=ORIGIN,
                            BUZZ_PRIVATE_KEY=(device/"disposable-owner.secret").read_text().strip())
            def buzz(*args):
                return json.loads(output([BIN/"buzz",*args],env=chat_env))
            channel=buzz("channels","create","--name","paired-host-smoke","--type","stream","--visibility","open")["channel_id"]
            draft=dict(name="Fixture", runtime="local/fixture", channels=[channel],
                       policy=dict(respond_to="owner-only",respond_to_allowlist=[],channel_add_policy="owner_only"))
            result=drive("create",offer["generation"],payload=draft)[-1]
            if result["state"] != "ready":
                raise RuntimeError("host did not report a ready runtime")
            agent=result["agent"]
            marker=secrets.token_hex(8)
            buzz("messages","send","--channel",channel,"--mention",agent,"--content","AE-ID:"+marker)
            def responses():
                messages=buzz("messages","get","--channel",channel,"--kinds","9")
                rows=messages if isinstance(messages,list) else messages["messages"]
                return [m for m in rows if m.get("pubkey")==agent and "AE-ACK:"+marker in m.get("content","")]
            eventually(responses)
            time.sleep(5)
            if len(responses()) != 1:
                raise RuntimeError("agent did not reply exactly once")
            if drive("unpair",offer["generation"])[-1]["state"] != "unpaired":
                raise RuntimeError("host did not return a signed unpair receipt")
            print("registered host created a working systemd agent, received one reply, and acknowledged unpair",flush=True)
        finally:
            for process in reversed(processes):
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()


if __name__ == "__main__":
    main()

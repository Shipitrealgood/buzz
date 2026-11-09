"""Disposable offline context. Fixed reference/modes; never a remote command API."""
import fcntl
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import time

REFERENCE = "service-proof-agent"
MODES = {"write", "read", "models", "hold", "fail", "login", "conversation", "native-models", "native-suite"}


def main():
    if len(sys.argv) != 3 or sys.argv[1] != REFERENCE or sys.argv[2] not in MODES:
        return 64
    mode = sys.argv[2]
    # The provider gates are intentionally not executable in this offline artifact.
    if mode in {"login", "conversation", "native-models"}:
        return 78
    home = Path("/state/home")
    if mode == "native-suite":
        assert os.getuid() != 0, "native service must use DynamicUser"
        home.mkdir(mode=0o700, exist_ok=True)
        (home / ".grok").mkdir(mode=0o700, exist_ok=True)
        Path("/state/work").mkdir(mode=0o700, exist_ok=True)
        os.chdir("/state/work")
        fail_next = Path("/state/fail-next")
        if fail_next.exists():
            fail_next.unlink()
            return 70
        receipt = home / "native-receipt.json"
        previous = json.loads(receipt.read_text()) if receipt.exists() else {"starts": []}
        results = {}
        for operation, expected in [("read" if receipt.exists() else "write", 0),
                                    ("models", 0), ("fail", 70), ("read", 0)]:
            child = subprocess.run([sys.executable, "-S", __file__, REFERENCE, operation],
                                   capture_output=True, timeout=15)
            assert child.returncode == expected, (operation, child.returncode)
            if operation == "models":
                payload = json.loads(child.stdout)
                assert payload["agent"]["name"] == "task-test"
                assert payload["stable"]["configOptions"][0]["currentValue"] == "old"
                results["model_payload"] = payload
        previous["starts"].append({"uid": os.getuid(), "state_owner": home.stat().st_uid,
                                    "persist_owner": (home / "persist.json").stat().st_uid})
        previous.update(results)
        receipt.write_text(json.dumps(previous))
        return 0
    with (home / "context.lock").open("a") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return 75
        if mode == "fail":
            return 70
        if mode == "hold":
            # Descendant in another session: namespace teardown must still kill it.
            subprocess.Popen([sys.executable, "-S", "-c", "import time; time.sleep(120)"], start_new_session=True)
            (home / "holding").write_text("ready")
            time.sleep(120)
            return 0
        if mode == "models":
            runtime = json.loads(Path("/proof/runtime.json").read_text())["synthetic"]
            env = dict(os.environ)
            env.update(BUZZ_ACP_AGENT_COMMAND=runtime["command"],
                       BUZZ_ACP_AGENT_ARGS=",".join(runtime["args"]),
                       TASK_AGENT_LOG="/state/home/adapter.jsonl",
                       GIT_CONFIG_COUNT="1", GIT_CONFIG_KEY_0="nostr.keyfile",
                       GIT_CONFIG_VALUE_0="/state/home/synthetic-no-key")
            return subprocess.call([runtime["harness"], "models", "--json"], env=env)
        spec = json.loads(Path("/proof/probes.json").read_text())
        assert os.environ["HOME"] == str(home)
        assert os.environ["GROK_HOME"] == str(home / ".grok")
        assert os.getcwd() == "/state/work"
        assert os.environ["PWD"] == "/state/work"
        assert set(os.environ) == {"HOME", "GROK_HOME", "PATH", "XDG_CONFIG_HOME", "XDG_RUNTIME_DIR", "LANG", "LC_CTYPE", "PWD"}, sorted(os.environ)
        assert not Path("/etc/grok").exists()
        assert not Path("/run/systemd/private").exists()
        for path in spec["denied_files"]:
            try:
                Path(path).read_bytes()
            except (FileNotFoundError, PermissionError):
                pass
            else:
                raise AssertionError("synthetic private canary readable")
        for family, address in [(socket.AF_INET, tuple(spec["ipv4"])),
                                (socket.AF_INET6, tuple(spec["ipv6"])),
                                (socket.AF_UNIX, spec["unix"]),
                                (socket.AF_UNIX, "\0" + spec["abstract"])]:
            with socket.socket(family, socket.SOCK_STREAM) as client:
                client.settimeout(0.4)
                try:
                    client.connect(address)
                except OSError:
                    pass
                else:
                    raise AssertionError("synthetic outside listener reachable")
        own_socket = home / "own.sock"
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
            server.bind(str(own_socket))
            server.listen()
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
                client.connect(str(own_socket))
                conn, _ = server.accept()
                conn.close()
        own_socket.unlink()
        marker = home / "persist.json"
        identity = {"uid": os.getuid(), "home": str(home), "grokHome": os.environ["GROK_HOME"]}
        if mode == "write":
            marker.write_text(json.dumps(identity))
            (home / ".grok" / "synthetic-state").write_text("not a credential")
        else:
            saved = json.loads(marker.read_text())
            if spec.get("uid_policy") == "service-owner":
                # systemd may remap DynamicUser between starts; own-state access is the contract.
                assert saved["home"] == identity["home"] and saved["grokHome"] == identity["grokHome"]
                assert all(path.stat().st_uid == os.getuid() for path in
                           [home, home / ".grok", marker, home / ".grok" / "synthetic-state"])
            else:
                assert saved == identity
            assert (home / ".grok" / "synthetic-state").read_text() == "not a credential"
        print(json.dumps({"mode": mode, "filesystem": "pass", "network_none": "pass", "state": "pass", **identity}))
        return 0


if __name__ == "__main__":
    sys.exit(main())

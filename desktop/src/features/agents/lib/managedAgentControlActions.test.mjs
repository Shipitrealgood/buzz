import assert from "node:assert/strict";
import test from "node:test";

import {
  getManagedAgentPrimaryAction,
  stopManagedAgentWithRules,
  startManagedAgentWithRules,
  respawnManagedAgentWithRules,
} from "./managedAgentControlActions.ts";

function agent(overrides = {}) {
  return {
    pubkey: "deadbeef".repeat(8),
    name: "Mesh Agent",
    personaId: null,
    relayUrl: "ws://localhost:3000",
    acpCommand: "buzz-acp",
    agentCommand: "goose",
    agentArgs: [],
    mcpCommand: "",
    turnTimeoutSeconds: 320,
    idleTimeoutSeconds: null,
    maxTurnDurationSeconds: null,
    parallelism: 1,
    systemPrompt: null,
    model: "hf://demo/model.gguf",
    envVars: {},
    status: "stopped",
    pid: null,
    createdAt: new Date(0).toISOString(),
    updatedAt: new Date(0).toISOString(),
    lastStartedAt: null,
    lastStoppedAt: null,
    lastExitCode: null,
    lastError: null,
    logPath: null,
    startOnAppLaunch: false,
    backend: { type: "local" },
    backendAgentId: null,
    respondTo: "owner-only",
    respondToAllowlist: [],
    ...overrides,
  };
}

test("relay-mesh agents delegate start to the backend preflight", async () => {
  const meshAgent = agent({
    envVars: {
      BUZZ_AGENT_PROVIDER: "openai",
      OPENAI_COMPAT_BASE_URL: "http://127.0.0.1:9337/v1/",
    },
  });

  let calledWith = null;
  await startManagedAgentWithRules({
    agent: meshAgent,
    startManagedAgent: async (pubkey) => {
      calledWith = pubkey;
    },
  });
  assert.equal(calledWith, meshAgent.pubkey);

  // Backend preflight failures (e.g. no live serve target) propagate as-is.
  await assert.rejects(
    startManagedAgentWithRules({
      agent: meshAgent,
      startManagedAgent: async () => {
        throw new Error("no live serve target is available for this model");
      },
    }),
    /no live serve target/,
  );
});

test("ordinary local agents still start normally", async () => {
  let calledWith = null;
  await startManagedAgentWithRules({
    agent: agent(),
    startManagedAgent: async (pubkey) => {
      calledWith = pubkey;
    },
  });
  assert.equal(calledWith, "deadbeef".repeat(8));
});

// --- respawnManagedAgentWithRules: stop→clear→start boundary tests -----------

test("test_respawn_stop_success_start_failure_onStopped_still_fires", async () => {
  // Prove: onStopped fires at the stop-success boundary even when start later
  // throws.  This is the key discriminator: on round-1 code the clear only
  // ran after the full respawn, so a failed start left the badge intact.
  const runningAgent = agent({ status: "running" });
  let onStoppedFired = false;

  await assert.rejects(
    respawnManagedAgentWithRules({
      agent: runningAgent,
      stopManagedAgent: async () => {
        /* stop succeeds */
      },
      startManagedAgent: async () => {
        throw new Error("start failed");
      },
      onStopped: () => {
        onStoppedFired = true;
      },
    }),
    /start failed/,
  );

  assert.ok(
    onStoppedFired,
    "onStopped must fire at stop-success boundary even when start subsequently fails",
  );
});

test("test_respawn_stop_failure_onStopped_not_called", async () => {
  // Prove: onStopped does NOT fire when stop itself throws.  Clearing on a
  // failed stop would remove a badge that is still legitimately active.
  const runningAgent = agent({ status: "running" });
  let onStoppedFired = false;

  await assert.rejects(
    respawnManagedAgentWithRules({
      agent: runningAgent,
      stopManagedAgent: async () => {
        throw new Error("stop failed");
      },
      startManagedAgent: async () => {
        /* should not be reached */
      },
      onStopped: () => {
        onStoppedFired = true;
      },
    }),
    /stop failed/,
  );

  assert.ok(
    !onStoppedFired,
    "onStopped must NOT fire when stop itself fails — badge is still active",
  );
});

test("test_respawn_onStopped_fires_before_start_resolves", async () => {
  // Prove: onStopped fires strictly between stop resolution and start
  // invocation.  A clear that fires after start begins can tombstone genuine
  // new turns from the freshly spawned process.
  const runningAgent = agent({ status: "running" });
  const events = [];

  await respawnManagedAgentWithRules({
    agent: runningAgent,
    stopManagedAgent: async () => {
      events.push("stop");
    },
    startManagedAgent: async () => {
      events.push("start");
    },
    onStopped: () => {
      events.push("onStopped");
    },
  });

  assert.deepEqual(
    events,
    ["stop", "onStopped", "start"],
    "onStopped must fire after stop resolves and before start is called",
  );
});

// --- respawn bound to the workspace relay -----------------------------------

const { markRelayRemoved } = await import("../managedAgentRelayCleanup.ts");

async function respawnAcrossStop(relayUrl, duringStop) {
  const started = [];
  const result = respawnManagedAgentWithRules({
    agent: agent({ status: "running" }),
    relayUrl,
    stopManagedAgent: async () => duringStop(),
    startManagedAgent: async (pubkey) => {
      started.push(pubkey);
    },
  });
  return { result, started };
}

test("a respawn whose own relay is removed and re-added during its stop starts nothing", async () => {
  const { result, started } = await respawnAcrossStop(
    "wss://respawn-own.example",
    () => markRelayRemoved("wss://respawn-own.example"),
  );
  await assert.rejects(result, /relay was removed from this device/);
  assert.deepEqual(started, []);
});

test("removing an unrelated relay during a respawn's stop still restarts it", async () => {
  const { result, started } = await respawnAcrossStop(
    "wss://respawn-kept.example",
    () => markRelayRemoved("wss://respawn-unrelated.example"),
  );
  await result;
  assert.equal(started.length, 1);
});

test("hosted lifecycle uses only owner/community presence; remote send pins its signer, relay and sole recipient", async (t) => {
  const target = agent({
    status: "deployed",
    backend: {
      type: "registered_host",
      scope: {
        community: "https://community.example",
        owner: "a".repeat(64),
      },
    },
  });
  const session = {
    expectedRelay: "wss://community.example",
    expectedOwner: target.backend.scope.owner,
  };
  const localStops = [];
  const ipc = [];
  const originalWindow = globalThis.window;
  t.after(() => {
    if (originalWindow === undefined) delete globalThis.window;
    else globalThis.window = originalWindow;
  });
  globalThis.window = {
    __TAURI_INTERNALS__: {
      invoke: async (command, args) => {
        ipc.push([command, args]);
        return { event_id: "event" };
      },
    },
  };
  const stop = (record, session, relayAgents) =>
    stopManagedAgentWithRules({
      agent: record,
      session,
      channels: [],
      relayAgents,
      stopManagedAgent: async (pubkey) => {
        localStops.push(pubkey);
      },
    });
  const directory = [{ pubkey: target.pubkey, channelIds: ["target-channel"] }];
  for (const wrong of [
    undefined,
    { ...session, expectedRelay: "wss://other.example" },
    { ...session, expectedOwner: "b".repeat(64) },
  ]) {
    assert.equal(
      getManagedAgentPrimaryAction(target, "online", wrong),
      undefined,
    );
    await assert.rejects(stop(target, wrong, directory), /community and owner/);
  }
  for (const presence of ["online", "away", "offline", undefined]) {
    const present = presence === "online" || presence === "away";
    assert.deepEqual(getManagedAgentPrimaryAction(target, presence, session), {
      stop: present,
      label: present ? "Stop" : "Start agent",
    });
  }
  await assert.rejects(
    stop(target, session, [
      { pubkey: "b".repeat(64), channelIds: ["wrong-channel"] },
    ]),
    /no channel could be resolved/,
  );
  assert.deepEqual(ipc, []);
  const request = await stop(target, session, directory);
  assert.match(request.noticeMessage, /^Shutdown requested/);
  assert.equal(ipc.length, 1);
  const [[command, args]] = ipc;
  assert.equal(command, "send_channel_message");
  assert.equal(args.channelId, "target-channel");
  assert.deepEqual(args.mentionPubkeys, [target.pubkey]);
  assert.equal(args.content, "!shutdown");
  assert.equal(args.expectedRelayUrl, session.expectedRelay);
  assert.equal(args.expectedSignerPubkey, session.expectedOwner);
  const provider = agent({ backend: { type: "provider" }, status: "deployed" });
  assert.match(
    (await stop(provider, undefined, directory)).noticeMessage,
    /^Shutdown requested/,
  );
  assert.equal(ipc[1][1].expectedSignerPubkey, null);
  assert.deepEqual(ipc[1][1].mentionPubkeys, [provider.pubkey]);
  assert.deepEqual(localStops, []);
  await stop(agent({ status: "running" }), undefined, []);
  assert.deepEqual(localStops, [target.pubkey]);
  assert.equal(ipc.length, 2);
});

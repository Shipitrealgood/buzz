import assert from "node:assert/strict";
import test from "node:test";

import {
  applyProbeResult,
  canSubmitWhereToRun,
  emptyWhereToRunDraft,
  providerConfigComplete,
  resolveBackendIntent,
} from "./whereToRunIntent.ts";

const probed = {
  ok: true,
  config_schema: {
    properties: { region: { type: "string" }, size: { type: "integer" } },
    required: ["region"],
  },
};

function providerDraft(overrides = {}) {
  return {
    ...emptyWhereToRunDraft,
    runOn: "blox",
    probedProvider: probed,
    providerConfig: { region: "us", size: "3" },
    ...overrides,
  };
}

test("provider selection blocks submit until the probe completes", () => {
  assert.equal(
    canSubmitWhereToRun(providerDraft({ probedProvider: null })),
    false,
  );
});

test("provider selection blocks submit while required config is missing", () => {
  const missing = providerDraft({ providerConfig: { size: "3" } });
  assert.equal(canSubmitWhereToRun(missing), false);
  assert.equal(providerConfigComplete(missing), false);
});

test("complete provider config allows submit", () => {
  assert.equal(canSubmitWhereToRun(providerDraft()), true);
});

test("local never gates submit", () => {
  assert.equal(canSubmitWhereToRun(emptyWhereToRunDraft), true);
});

test("local draft resolves to null intent", () => {
  assert.equal(resolveBackendIntent(emptyWhereToRunDraft), null);
});

test("provider draft resolves with coerced config values", () => {
  const intent = resolveBackendIntent(providerDraft());
  assert.deepEqual(intent, {
    type: "provider",
    id: "blox",
    config: { region: "us", size: 3 },
  });
});

// ── applyProbeResult: probe resolution must merge, not overwrite ─────────────
//
// Pins the seam that fixed the "Typewriter Eraser" (agent-create dialog's
// provider config fields losing keystrokes): a probe resolution prefills
// schema defaults *beneath* the user's in-flight config, never over it. The
// effect in WhereToRunSection keys probing on the provider's binary path, so
// the only probe writes that reach providerConfig are the ones pinned here.

const probeWithDefaults = {
  ok: true,
  config_schema: {
    properties: {
      context: { type: "string", title: "Kubeconfig context" },
      namespace: { type: "string", default: "buzz-agents-x1y2z3" },
      inactivity_seconds: { type: "number", default: 1800 },
    },
    required: ["namespace"],
  },
};

const unprobedDraft = {
  ...emptyWhereToRunDraft,
  runOn: "kubernetes",
};

test("probe resolution prefills schema defaults on a fresh draft", () => {
  const next = applyProbeResult(unprobedDraft, probeWithDefaults);
  assert.equal(next.probedProvider, probeWithDefaults);
  assert.deepEqual(next.providerConfig, {
    namespace: "buzz-agents-x1y2z3",
    inactivity_seconds: "1800",
  });
});

test("probe resolution keeps user-typed values over schema defaults", () => {
  const typed = {
    ...unprobedDraft,
    providerConfig: { context: "prod-us-west", namespace: "my-ns" },
  };
  const next = applyProbeResult(typed, probeWithDefaults);
  assert.deepEqual(next.providerConfig, {
    context: "prod-us-west",
    namespace: "my-ns",
    inactivity_seconds: "1800",
  });
});

test("probe resolution keeps a user-cleared field cleared", () => {
  // "" is a deliberate user state — coerceConfigValues drops empty numerics
  // and required-gating treats "" as incomplete; the probe must not undo it.
  const cleared = { ...unprobedDraft, providerConfig: { namespace: "" } };
  const next = applyProbeResult(cleared, probeWithDefaults);
  assert.equal(next.providerConfig.namespace, "");
});

test("a schema-less probe result records the probe without touching config", () => {
  const typed = { ...unprobedDraft, providerConfig: { context: "abc" } };
  const next = applyProbeResult(typed, { ok: true });
  assert.deepEqual(next.providerConfig, { context: "abc" });
  assert.deepEqual(next.probedProvider, { ok: true });
});

test("probe resolution preserves unrelated draft fields", () => {
  assert.equal(
    applyProbeResult(unprobedDraft, probeWithDefaults).runOn,
    "kubernetes",
  );
});

test("host destination always blocks even with complete provider/capability data", () => {
  const destination = {
    type: "registered-host",
    scope: {
      community: "https://fixture.test",
      owner: "a",
      host: "b",
      generation: "c",
    },
  };
  for (const runOn of ["local", "blox", "host:forged"]) {
    const draft = providerDraft({
      runOn,
      destination,
      deployment: { supported: true },
    });
    assert.equal(canSubmitWhereToRun(draft), false);
    assert.deepEqual(resolveBackendIntent(draft), destination);
    assert.equal(applyProbeResult(draft, probeWithDefaults), draft);
  }
  assert.equal(
    canSubmitWhereToRun(providerDraft({ runOn: "host:forged" })),
    false,
  );
  assert.throws(
    () => resolveBackendIntent(providerDraft({ runOn: "host:forged" })),
    /no longer available/,
  );
});

test("host submit requires both current registration and native readiness", () => {
  const destination = { type: "registered-host", scope: {} };
  for (const registered of [false, true])
    for (const ready of [false, true]) {
      assert.equal(
        canSubmitWhereToRun(
          { ...emptyWhereToRunDraft, destination, hostAvailable: registered },
          ready,
        ),
        registered && ready,
      );
    }
});

test("host catalog and model evidence remain exact-scope and freshness bound", async () => {
  const { currentHostedCatalog, currentHostedModels } = await import(
    "@/shared/api/hostedAgents.ts"
  );
  const scope = {
    community: "https://fixture.test",
    owner: "ab".repeat(32),
    host: "cd".repeat(32),
    generation: "ef".repeat(32),
  };
  const catalog = {
    status: {
      scope,
      availability: "current",
      expires_at: 2,
      capability: { state: "supported" },
    },
    runtimes: [],
  };
  assert.equal(currentHostedCatalog(catalog, scope, 1000), true);
  for (const availability of ["stale", "revoked", "unavailable"])
    assert.equal(
      currentHostedCatalog(
        { ...catalog, status: { ...catalog.status, availability } },
        scope,
        1000,
      ),
      false,
    );
  assert.equal(currentHostedCatalog(catalog, scope, 2000), false);
  assert.equal(
    currentHostedCatalog(
      {
        ...catalog,
        status: { ...catalog.status, capability: { state: "unsupported" } },
      },
      scope,
      1000,
    ),
    false,
  );
  for (const field of ["community", "owner", "host", "generation"])
    assert.equal(
      currentHostedCatalog(catalog, { ...scope, [field]: "other" }, 1000),
      false,
    );
  const models = { models: [{ id: "opaque/model", name: "Fixture" }] };
  const observation = {
    scope,
    agent: "agent",
    runtime_id: "native/grok",
    operation_id: "operation",
    revision: 2,
    expires_at: 2,
    login: "not-observed",
    readiness: "logged-in",
    availability: "available",
    models: { state: "available", catalog: models },
  };
  const setup = {
    scope,
    pubkey: "agent",
    runtime: "native/grok",
    operation: "operation",
    revision: 2,
    state: "needs-login",
    observation,
  };
  for (const runtime of ["native/grok", "approved/claude"]) {
    const mapped = {
      ...setup,
      runtime,
      observation: { ...observation, runtime_id: runtime },
    };
    assert.equal(currentHostedModels(mapped, 1000), models);
    assert.equal(
      currentHostedModels(
        {
          ...mapped,
          observation: { ...mapped.observation, readiness: "not-applicable" },
        },
        1000,
      ),
      models,
    );
    assert.equal(
      currentHostedModels(
        {
          ...mapped,
          observation: {
            ...mapped.observation,
            login: "succeeded",
            readiness: "logged-out",
          },
        },
        1000,
      ),
      null,
    );
  }
  for (const [field, value] of [
    ["agent", "other"],
    ["runtime_id", "other"],
    ["operation_id", "old"],
    ["revision", 1],
    ["expires_at", 1],
    ["readiness", "unknown"],
    ["readiness", undefined],
    ["availability", "cli_missing"],
    ["models", { state: "empty" }],
    ["scope", { ...scope, owner: "other" }],
  ])
    assert.equal(
      currentHostedModels(
        { ...setup, observation: { ...observation, [field]: value } },
        1000,
      ),
      null,
    );
  for (const state of ["failed", "pending", "conflict", "unverified"])
    assert.equal(currentHostedModels({ ...setup, state }, 1000), null);
});

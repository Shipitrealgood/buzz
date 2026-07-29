import assert from "node:assert/strict";
import test from "node:test";

import {
  canSubmitWhereToRun,
  emptyWhereToRunDraft,
  providerConfigComplete,
  resolveBackendIntent,
  seedProviderConfigDefaults,
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

const seedSchema = {
  properties: {
    host: { type: "string" },
    region: { type: "string", default: "us-east" },
    tier: { type: "string", default: "standard" },
  },
};

test("seeding fills only empty fields from schema defaults", () => {
  const seeded = seedProviderConfigDefaults(seedSchema, {
    region: "eu-west",
    tier: "",
  });
  // Typed value wins; empty and absent fields take the default.
  assert.deepEqual(seeded, { region: "eu-west", tier: "standard" });
});

test("seeding never clobbers typed config on re-probe", () => {
  // Regression: the probe effect re-ran per keystroke and replaced the whole
  // config with defaults, wiping the user's host as they typed it.
  const typed = { host: "user@backend", region: "us-east" };
  assert.deepEqual(seedProviderConfigDefaults(seedSchema, typed), {
    host: "user@backend",
    region: "us-east",
    tier: "standard",
  });
});

test("seeding tolerates a missing or default-free schema", () => {
  assert.deepEqual(seedProviderConfigDefaults(undefined, { a: "1" }), {
    a: "1",
  });
  assert.deepEqual(
    seedProviderConfigDefaults(
      { properties: { host: { type: "string" } } },
      {},
    ),
    {},
  );
});

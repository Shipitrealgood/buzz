/**
 * Exercises the real discovery hook: runtime switches relabel the same cached
 * response, and a failed discovery always settles instead of loading forever.
 */
import assert from "node:assert/strict";
import { afterEach, test } from "node:test";

const discovered = {
  agentName: "@agentclientprotocol/claude-agent-acp",
  agentVersion: "0.36.1",
  agentDefaultModel: "claude-sonnet-5",
  selectedModel: null,
  supportsSwitching: true,
  models: [{ id: "claude-sonnet-5", name: "Sonnet", description: null }],
};
const calls = [];
let discover = () => Promise.resolve(discovered);
let hostDiscover = () =>
  Promise.resolve({
    availability: "available",
    readiness: "logged-in",
    models: { state: "available", catalog: discovered },
  });
const tauriMock = {
  invoke(command, input) {
    calls.push(command);
    if (command === "hosted_models") return hostDiscover(input);
    if (command === "discover_agent_models") return discover();
    return Promise.reject(new Error(`unmocked Tauri command: ${command}`));
  },
  transformCallback() {
    return Math.random();
  },
};
globalThis.__TAURI_INTERNALS__ = tauriMock;
globalThis.window.__TAURI_INTERNALS__ = tauriMock;

const React = (await import("react")).default;
const { act } = await import("react");
const { createRoot } = await import("react-dom/client");
const { usePersonaModelDiscovery } = await import(
  "./usePersonaModelDiscovery.ts"
);

const runtime = (id) => ({
  id,
  label: id,
  command: "shared-acp",
  availability: "available",
});
const envVars = [];

let root;
let container;
afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
  discover = () => Promise.resolve(discovered);
});

function mountRoot() {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
}

let labels = null;
function Probe({ selectedRuntime }) {
  const { discoveredModelOptions } = usePersonaModelDiscovery({
    envVars,
    isCustomProviderEditing: false,
    modelFieldVisible: true,
    open: true,
    provider: "",
    selectedRuntime,
  });
  labels = discoveredModelOptions?.map((option) => option.label) ?? null;
  return null;
}

async function render(id) {
  await act(async () => {
    root.render(React.createElement(Probe, { selectedRuntime: runtime(id) }));
  });
}

test("hook relabels discovered models when the runtime switches", async () => {
  mountRoot();

  await render("claude");
  assert.deepEqual(labels, [
    "Default model (Claude Sonnet 5)",
    "Claude Sonnet 5",
  ]);
  await render("codex");
  assert.deepEqual(labels, ["Default model (Sonnet)", "Sonnet"]);
  await render("claude");
  assert.deepEqual(labels, [
    "Default model (Claude Sonnet 5)",
    "Claude Sonnet 5",
  ]);
  assert.equal(
    calls.filter((command) => command === "discover_agent_models").length,
    1,
  );
});

test("a missing Databricks host settles discovery instead of loading forever", async () => {
  discover = () =>
    Promise.reject(
      "buzz-acp models failed (exit 1): config: DATABRICKS_MODEL required",
    );
  let state = null;
  function DatabricksProbe() {
    state = usePersonaModelDiscovery({
      envVars,
      isCustomProviderEditing: false,
      modelFieldVisible: true,
      open: true,
      provider: "databricks_v2",
      selectedRuntime: runtime("buzz-agent"),
    });
    return null;
  }
  mountRoot();

  await act(async () => {
    root.render(React.createElement(DatabricksProbe));
  });
  // databricks_v2 discovery is debounced before the IPC call.
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 400));
  });

  assert.equal(state.modelDiscoveryLoading, false);
  assert.match(state.modelDiscoveryStatus?.message ?? "", /DATABRICKS_HOST/);
});

test("host discovery shares picker/cache, scopes runtime and ignores late replies without creating agents", async () => {
  mountRoot();
  const pending = [];
  const requests = [];
  hostDiscover = (input) => {
    requests.push(input);
    return new Promise((resolve) => pending.push(resolve));
  };
  let result;
  function HostedProbe({
    host,
    runtimeId = "main/claude",
    revision = "revision-a",
  }) {
    result = usePersonaModelDiscovery({
      source: {
        type: "hosted",
        key: host,
        scope: {
          community: "https://fixture.test",
          owner: "owner",
          host,
          generation: "g",
        },
        catalogRevision: revision,
      },
      envVars,
      isCustomProviderEditing: false,
      modelFieldVisible: true,
      open: true,
      provider: "",
      selectedRuntime: runtime(runtimeId),
    });
    return null;
  }
  const renderHost = (host, runtimeId, revision) =>
    act(async () => {
      root.render(
        React.createElement(HostedProbe, { host, runtimeId, revision }),
      );
    });
  const response = (id) => ({
    availability: "available",
    readiness: "logged-in",
    models: {
      state: "available",
      catalog: { ...discovered, models: [{ id, name: id, description: null }] },
    },
  });
  const start = calls.length;
  await renderHost("one");
  assert.equal(result.modelDiscoveryLoading, true);
  await renderHost("two");
  await act(async () => pending[0](response("obsolete")));
  assert.deepEqual(result.availableModelIds, []);
  await act(async () => pending[1](response("default")));
  assert.deepEqual(result.availableModelIds, ["default"]);
  assert.equal(result.explicitDefaultModel, "default");
  assert.equal(result.discoveredModelOptions[0].id, "");
  await renderHost("two");
  assert.equal(requests.length, 2, "same scoped input reuses cache");
  await renderHost("two", "other-installed-id");
  assert.deepEqual(result.availableModelIds, []);
  await act(async () => pending[2](response("explicit")));
  assert.equal(result.explicitDefaultModel, null);
  await renderHost("two", "other-installed-id", "revision-b");
  assert.deepEqual(result.availableModelIds, []);
  await act(async () =>
    pending[3]({
      availability: "available",
      readiness: "logged-in",
      models: { state: "unavailable", reason: "timeout" },
    }),
  );
  assert.equal(result.modelDiscoveryLoading, false);
  assert.equal(result.modelDiscoverySuccessfulEmpty, false);
  assert.deepEqual(result.availableModelIds, []);
  assert.match(result.modelDiscoveryStatus.message, /timeout|unavailable/i);
  assert.deepEqual(
    calls.slice(start),
    Array(4).fill("hosted_models"),
    "no local discovery, key, profile or create IPC",
  );
  assert.equal(requests[1].scope.host, "two");
  assert.equal(requests[2].runtime, "other-installed-id");
  assert.equal(requests[3].catalogRevision, "revision-b");
});

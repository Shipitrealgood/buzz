import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
Object.defineProperty(navigator, "platform", {
  value: "Win32",
  configurable: true,
});
const calls = [];
let list = async () => ({ supported: true, available: true, hosts: [] });
const bridge = {
  invoke(command, args) {
    if (command === "get_relay_http_url")
      return Promise.resolve("https://fixture.test");
    if (command === "get_media_proxy_port") return Promise.resolve(null);
    calls.push({ command, args });
    if (command === "paired_host_list") return list(args);
    throw new Error(`unexpected side effect: ${command}`);
  },
  transformCallback() {
    return 1;
  },
};
globalThis.__TAURI_INTERNALS__ = bridge;
window.__TAURI_INTERNALS__ = bridge;
const React = (await import("react")).default;
const { act } = React;
const { createRoot } = await import("react-dom/client");
const { QueryClient, QueryClientProvider } = await import(
  "@tanstack/react-query"
);
const { WhereToRunFields } = await import("./WhereToRunSection.tsx");
const { emptyWhereToRunDraft, canSubmitWhereToRun } = await import(
  "./whereToRunIntent.ts"
);
const { hostDestination, hostDestinationValue } = await import(
  "../lib/hostDestination.ts"
);
let root, container, client;
const session = {
  expectedRelay: "wss://fixture.test",
  expectedOwner: "ab".repeat(32),
};
const host = {
  scope: {
    community: "https://fixture.test",
    owner: session.expectedOwner,
    host: "cd".repeat(32),
    generation: "ef".repeat(32),
  },
  state: "paired",
  label: "Workshop",
  fresh: true,
  runtimes: ["legacy/ignored"],
  requests: [],
  capacity: 10,
  catalog_revision: "12".repeat(32),
  fresh_for_ms: 60000,
};
let draft = emptyWhereToRunDraft;
async function flush() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 20));
  });
}
async function render(selected = session) {
  if (!root) {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  }
  await act(async () =>
    root.render(
      React.createElement(
        QueryClientProvider,
        { client },
        React.createElement(WhereToRunFields, {
          draft,
          isPending: false,
          onDraftChange: (next) => {
            draft = next;
          },
          backendProviders: [],
          session: selected,
        }),
      ),
    ),
  );
  await flush();
}
afterEach(() => {
  act(() => root?.unmount());
  client?.clear();
  container?.remove();
  root = undefined;
  draft = emptyWhereToRunDraft;
  calls.length = 0;
});

test("actual Run on projects status truthfully and remains closed with legacy capability fields", async () => {
  for (const [state, fresh, expected] of [
    ["paired", true, "Registered · deployment unavailable"],
    ["paired", false, "Saved registration · refresh needed"],
    ["unpaired", true, "Revoked"],
    ["unpair-pending", true, "Revocation pending"],
    ["mystery", false, "Registration status unknown"],
  ]) {
    const row = { ...host, state, fresh };
    list = async () => ({ supported: true, available: true, hosts: [row] });
    draft = {
      ...emptyWhereToRunDraft,
      runOn: hostDestinationValue(row),
      destination: hostDestination(row),
    };
    await render();
    await act(async () =>
      client.invalidateQueries({ queryKey: ["paired-hosts"] }),
    );
    await flush();
    assert.ok(container.textContent.includes(expected), container.textContent);
    assert.equal(canSubmitWhereToRun(draft), false);
    assert.doesNotMatch(
      container.textContent,
      /legacy\/ignored|online|offline/,
    );
  }
  assert.ok(calls.every((c) => c.command === "paired_host_list"));
});

test("late old owner/community query cannot populate the new workspace or turn the selected host into local", async () => {
  let resolveOld;
  list = () =>
    new Promise((resolve) => {
      resolveOld = resolve;
    });
  draft = {
    ...emptyWhereToRunDraft,
    runOn: hostDestinationValue(host),
    destination: hostDestination(host),
  };
  await render();
  list = async () => ({ supported: true, available: true, hosts: [host] }); // even a mis-scoped projection
  await render({
    expectedRelay: "wss://other.test",
    expectedOwner: "12".repeat(32),
  });
  await act(async () =>
    resolveOld({ supported: true, available: true, hosts: [host] }),
  );
  await flush();
  assert.doesNotMatch(container.textContent, /Workshop/);
  assert.match(
    container.textContent,
    /Selected host is unavailable in this workspace/,
  );
  assert.equal(canSubmitWhereToRun(draft), false);
  assert.ok(calls.every((c) => c.command === "paired_host_list"));
});

test("unsupported and unavailable directories stay distinct from empty and keep the discovery affordance", async () => {
  for (const [supported, available, expected] of [
    [false, true, "not supported"],
    [true, false, "registrations unavailable"],
    [true, true, "No registered hosts"],
  ]) {
    list = async () => ({ supported, available, hosts: [] });
    await render();
    await act(async () =>
      client.invalidateQueries({ queryKey: ["paired-hosts"] }),
    );
    await flush();
    assert.ok(container.textContent.includes(expected), container.textContent);
    assert.ok(
      [...container.querySelectorAll("button")].some(
        (b) => b.textContent === "Add Host",
      ),
    );
  }
});

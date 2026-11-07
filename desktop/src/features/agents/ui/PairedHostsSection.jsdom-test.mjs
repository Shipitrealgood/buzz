import assert from "node:assert/strict";
import { afterEach, test } from "node:test";

const calls = [];
let list = async () => ({ supported: true, available: true, hosts: [] });
let pair = async () => {
  throw new Error("Invitation expired");
};
const bridge = {
  invoke(command, args) {
    if (command === "get_relay_http_url")
      return Promise.resolve("https://fixture.test");
    if (command === "get_media_proxy_port") return Promise.resolve(null);
    calls.push({ command, args });
    if (command === "paired_host_list") return list(args);
    if (command === "paired_host_pair") return pair(args);
    if (command === "paired_host_unpair") return Promise.resolve({});
    throw new Error(`unexpected command: ${command}`);
  },
  transformCallback() {
    return Math.random();
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
const { MyHosts } = await import("./PairedHostsSection.tsx");
let root;
let container;
let client;
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
  label: "Fixture host",
  runtimes: ["must-not-display"],
  requests: [],
  fresh: true,
};
afterEach(() => {
  act(() => root?.unmount());
  client?.clear();
  container?.remove();
  root = undefined;
  calls.length = 0;
});
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
        React.createElement(MyHosts, { session: selected }),
      ),
    ),
  );
  await flush();
}
async function click(text) {
  const button = [...container.querySelectorAll("button")].find(
    (b) => b.textContent === text,
  );
  assert.ok(button, `missing button ${text}`);
  await act(async () => button.click());
  await flush();
}
async function refresh() {
  await act(async () => {
    await client.invalidateQueries({ queryKey: ["paired-hosts"] });
  });
  await flush();
}
function submit(value) {
  const input = container.querySelector("input");
  input.value = value;
  container
    .querySelector("form")
    .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
}

test("empty registration journey supports cancel, expiry, retry and revoke without provisioning", async () => {
  list = async () => ({ supported: true, available: true, hosts: [] });
  await render();
  assert.match(container.textContent, /No registered hosts/);
  await click("Add Host");
  container.querySelector("input").value = "private invitation";
  await click("Cancel");
  assert.equal(calls.filter((c) => c.command === "paired_host_pair").length, 0);
  await click("Add Host");
  await act(async () => submit("expired"));
  await flush();
  assert.equal(container.querySelector("input").value, "");
  assert.match(container.textContent, /Invitation expired/);
  pair = async () => {
    list = async () => ({
      supported: true,
      available: true,
      hosts: [{ ...host, state: "pair-pending" }],
    });
    return host;
  };
  await act(async () => submit("fresh invitation"));
  await flush();
  assert.match(container.textContent, /Registration pending/);
  list = async () => ({ supported: true, available: true, hosts: [host] });
  await refresh();
  assert.match(container.textContent, /Registered/);
  assert.doesNotMatch(
    container.textContent,
    /must-not-display|Add agent|Installed runtimes|Host confirmed/,
  );
  await click("Revoke registration");
  await click("Cancel");
  assert.equal(
    calls.filter((c) => c.command === "paired_host_unpair").length,
    0,
  );
  await click("Revoke registration");
  await click("Confirm revoke");
  assert.equal(
    calls.filter((c) => c.command === "paired_host_unpair").length,
    1,
  );
  list = async () => ({
    supported: true,
    available: true,
    hosts: [{ ...host, state: "unpair-pending" }],
  });
  await refresh();
  assert.match(container.textContent, /Revocation pending/);
  list = async () => ({
    supported: true,
    available: true,
    hosts: [{ ...host, state: "unpaired" }],
  });
  await refresh();
  assert.match(container.textContent, /Revoked/);
  assert.ok(
    calls.every((c) =>
      ["paired_host_list", "paired_host_pair", "paired_host_unpair"].includes(
        c.command,
      ),
    ),
    calls.map((c) => c.command).join(","),
  );
});

test("unsupported, failed and changed sessions cannot reuse Add Host or late pairing feedback", async () => {
  root = undefined;
  list = async () => ({ supported: false, available: true, hosts: [] });
  await render();
  assert.match(container.textContent, /not supported/);
  assert.doesNotMatch(container.textContent, /Add Host/);
  list = async () => {
    throw new Error("network failed");
  };
  await refresh();
  assert.match(container.textContent, /Community unavailable/);
  assert.doesNotMatch(container.textContent, /Add Host/);
  list = async () => ({ supported: true, available: true, hosts: [] });
  await click("Retry");
  let reject;
  pair = () =>
    new Promise((_, r) => {
      reject = r;
    });
  await click("Add Host");
  await act(async () => submit("private"));
  await render({
    ...session,
    expectedOwner: "12".repeat(32),
    expectedRelay: "wss://other.test",
  });
  await act(async () => reject(new Error("old session reply")));
  await flush();
  assert.doesNotMatch(container.textContent, /old session reply|Registering/);
  assert.equal(container.querySelector("input"), null);
});

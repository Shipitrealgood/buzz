import { expect, test, type Page } from "@playwright/test";
import { installMockBridge } from "../helpers/bridge";

async function open(
  page: Page,
  supported: boolean,
  platform = "Win32",
  ownerOnly = false,
) {
  await installMockBridge(
    page,
    { ownerOnlyAccessBuild: ownerOnly },
    { relayWsUrl: "wss://community.example" },
  );
  await page.addInitScript(
    (platform) =>
      Object.defineProperty(navigator, "platform", {
        value: platform,
        configurable: true,
      }),
    platform,
  );
  await page.goto("/");
  await page.getByTestId("open-agents-view").waitFor();
  await page.evaluate((supported) => {
    const api = (
      window as unknown as {
        __TAURI_INTERNALS__: {
          invoke: (
            command: string,
            args?: Record<string, unknown>,
          ) => Promise<unknown>;
        };
      }
    ).__TAURI_INTERNALS__;
    const previous = api.invoke.bind(api);
    const hosts = [
      ["31", "11"],
      ["32", "22"],
    ].map(([g, h]) => ({
      scope: {
        community: "https://community.example",
        generation: g.repeat(32),
        host: h.repeat(32),
        owner: "deadbeef".repeat(8),
      },
      requests: [] as {
        request_id: string;
        name: string;
        runtime: string;
        agent: string | null;
        state: string;
        steps: string[];
        issue: string | null;
        evidence_incomplete: boolean;
      }[],
      catalog_revision: "ab".repeat(32),
      state: "paired",
      fresh: false,
      inventory_id: g.repeat(32),
      fresh_for_ms: 0,
      label: "Workshop",
      runtimes: ["test/echo"],
    }));
    api.invoke = async (command, args) => {
      if (command === "paired_host_list") {
        if (document.body.dataset.supersede) {
          hosts[1].inventory_id = "cc".repeat(32);
          hosts[1].label = "New catalog";
        }
        document.body.dataset.pairedListRead = "yes";
        return { supported, available: true, hosts: structuredClone(hosts) };
      }
      if (command === "paired_host_pair") {
        if (args?.invitation !== "private-console-test-input")
          throw new Error("wrong invitation");
        hosts.push({
          ...hosts[0],
          label: "Third host",
          state: "pair-pending",
          scope: {
            ...hosts[0].scope,
            generation: "33".repeat(32),
            host: "44".repeat(32),
          },
        });
        return hosts[2];
      }
      if (command === "paired_host_unpair") {
        const host = hosts.find(
          (h) =>
            h.scope.host === args?.host &&
            h.scope.generation === args?.generation &&
            h.scope.owner === args?.expectedOwner,
        );
        if (!host) throw new Error("wrong host");
        host.state = "unpair-pending";
        return host;
      }
      if (command === "paired_host_create") {
        const h = hosts.find(
          (h) =>
            h.scope.host === args?.host &&
            h.scope.generation === args?.generation &&
            h.scope.owner === args?.expectedOwner,
        );
        if (!h || h.inventory_id !== args?.catalogId)
          throw new Error("wrong create target");
        const draft = args.draft as {
          name: string;
          runtime: string;
          policy: { respond_to: string; channel_add_policy: string };
          channels: string[];
        };
        if (
          draft.name !== "Build helper" ||
          draft.runtime !== "test/echo" ||
          draft.policy.respond_to !== "owner-only" ||
          draft.policy.channel_add_policy !== "owner_only" ||
          draft.channels.length !== 1
        )
          throw new Error("wrong create inputs");
        const tap = String(args.tap);
        if (!/^[0-9a-f]{64}$/.test(tap)) throw new Error("invalid tap");
        if (!h.requests.length) {
          h.requests.push({
            request_id: tap,
            name: draft.name,
            runtime: draft.runtime,
            agent: null,
            state: "pending",
            steps: [],
            issue: null,
            evidence_incomplete: false,
          });
          throw new Error("Lost IPC response after durable save.");
        }
        if (h.requests[0].request_id !== tap)
          throw new Error("duplicate create after lost response");
        return h.requests[0];
      }
      if (command === "paired_host_resume") {
        const r = hosts
          .find((h) => h.scope.host === args?.host)
          ?.requests.find((r) => r.request_id === args?.requestId);
        if (!r) throw new Error("wrong resume target");
        document.body.dataset.resumed = r.request_id;
        r.evidence_incomplete = document.body.dataset.pruned === "yes";
        if (document.body.dataset.finish) {
          r.state = "ready";
          r.agent = "cd".repeat(32);
          r.steps = ["verified"];
        }
        return r;
      }
      if (command === "paired_host_history_ids")
        return [["33".repeat(32)], null];
      if (command === "paired_host_history")
        return {
          name: "Other device agent",
          host_state: "ready",
          agent: "ef".repeat(32),
        };
      if (command === "paired_host_refresh") {
        const host = hosts.find(
          (h) =>
            h.scope.host === args?.host &&
            h.scope.generation === args?.generation,
        );
        if (!host) throw new Error("wrong host");
        return {
          ...host,
          fresh: true,
          fresh_for_ms: document.body.dataset.shortInventory ? 500 : 60_000,
        };
      }
      return previous(command, args);
    };
  }, supported);
  await page.getByTestId("open-agents-view").click();
  await expect(page.getByTestId("agents-page-content")).toBeVisible();
}
test("Windows pairs another host and routes identical labels by enrolled identity", async ({
  page,
}) => {
  await open(page, true);
  const panel = page.getByRole("region", { name: "My hosts" });
  const hosts = panel.getByRole("listitem");
  await expect(hosts).toHaveCount(2);
  await hosts.nth(1).getByRole("button", { name: "Check host" }).click();
  await expect(hosts.nth(1)).toContainText("Host confirmed");
  await expect(hosts.first()).toContainText("inventory cached");
  await panel.getByRole("button", { name: "Pair host" }).click();
  const input = panel.getByLabel("Private host invitation");
  await expect(input).toHaveAttribute("type", "password");
  await input.fill("private-console-test-input");
  await input.press("Enter");
  await expect(hosts).toHaveCount(3);
  await expect(hosts.last()).toContainText("Awaiting host acknowledgement");
  await hosts
    .nth(1)
    .getByRole("button", { name: "Unpair", exact: true })
    .click();
  await hosts.nth(1).getByRole("button", { name: "Confirm unpair" }).click();
  await expect(hosts.nth(1)).toContainText("Unpair pending at host");
  await expect(hosts.first()).toContainText("inventory cached");
});
test("old relay hides Windows host management", async ({ page }) => {
  await open(page, false);
  await expect(page.locator("body")).toHaveAttribute(
    "data-paired-list-read",
    "yes",
  );
  await page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
  await expect(page.getByRole("region", { name: "My hosts" })).toHaveCount(0);
});
test("other platforms keep the stock Agents surface", async ({ page }) => {
  await open(page, true, "Linux x86_64");
  await expect(page.getByRole("region", { name: "My hosts" })).toHaveCount(0);
});

test("host confirmation expires on native lifetime and clears on a newer inventory event", async ({
  page,
}) => {
  await open(page, true);
  const rows = page
    .getByRole("region", { name: "My hosts" })
    .getByRole("listitem");
  const second = rows.nth(1);
  await page.evaluate(() => {
    document.body.dataset.shortInventory = "yes";
  });
  await second.getByRole("button", { name: "Check host" }).click();
  await expect(second).toContainText("Host confirmed");
  await expect(second).toContainText("inventory cached", { timeout: 2500 });
  await page.evaluate(() => {
    delete document.body.dataset.shortInventory;
  });
  await second.getByRole("button", { name: "Check host" }).click();
  await expect(second).toContainText("Host confirmed");
  await page.evaluate(() => {
    document.body.dataset.supersede = "yes";
  });
  await rows.first().getByRole("button", { name: "Check host" }).click();
  await expect(second).toContainText("New catalog");
  await expect(second).toContainText("inventory cached");
});

test("Create on selected host preserves one tap through lost IPC, resumes status and verifies separate history", async ({
  page,
}) => {
  await open(page, true, "Win32", true);
  const panel = page.getByRole("region", { name: "My hosts" });
  const rows = panel.getByRole("listitem");
  await panel
    .getByLabel("Host for new agent")
    .selectOption("32".repeat(32) + "22".repeat(32));
  await panel.getByRole("button", { name: "Add agent", exact: true }).click();
  const form = rows
    .nth(1)
    .getByRole("form", { name: "Create an agent on this host" });
  await form.getByLabel("Agent name").fill("Build helper");
  await form.getByRole("checkbox", { name: "general", exact: true }).check();
  await form.getByRole("button", { name: "Create", exact: true }).dblclick();
  await expect(form.getByRole("alert")).toContainText("Lost IPC response");
  await expect(
    rows.nth(1).getByRole("group", { name: "Build helper provisioning" }),
  ).toContainText("Pending");
  await expect(rows.first()).not.toContainText("Build helper");
  await form.getByRole("button", { name: "Retry same request" }).click();
  await expect(form).toHaveCount(0);
  await expect(page.locator("body")).toHaveAttribute(
    "data-resumed",
    /^[0-9a-f]{64}$/,
  );
  const progress = rows
    .nth(1)
    .getByRole("group", { name: "Build helper provisioning" });
  await page.evaluate(() => {
    document.body.dataset.pruned = "yes";
  });
  await progress.getByRole("button", { name: "Resume request" }).click();
  await expect(progress).toContainText("Evidence incomplete");
  await expect(progress).toContainText("Pending");
  await page.evaluate(() => {
    delete document.body.dataset.pruned;
  });
  await progress.getByRole("button", { name: "Resume request" }).click();
  await expect(progress).not.toContainText("Evidence incomplete");
  await expect(progress).toContainText("Pending");
  await page.evaluate(() => {
    document.body.dataset.finish = "yes";
  });
  await rows.nth(1).getByRole("button", { name: "Resume request" }).click();
  await expect(
    rows.nth(1).getByRole("group", { name: "Build helper provisioning" }),
  ).toContainText("Ready");
  await rows.first().getByRole("button", { name: "Request history" }).click();
  await rows
    .first()
    .getByRole("button", { name: /Inspect request/ })
    .click();
  await expect(rows.first()).toContainText(
    "Other device agent · host reported Ready",
  );
  await expect(
    rows.first().getByRole("button", { name: "Resume request" }),
  ).toHaveCount(0);
});

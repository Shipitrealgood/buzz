import { expect, test, type Locator, type Page } from "@playwright/test";
import type { Host } from "../../src/features/agents/ui/PairedHostsSection";
import { installMockBridge } from "../helpers/bridge";

type HostCall = { command: string; args?: Record<string, unknown> };
type Fixture = {
  supported: boolean;
  available: boolean;
  listError: boolean;
  hosts: Host[];
  calls: HostCall[];
};
declare global {
  interface Window {
    __PAIRED_HOSTS_FIXTURE__: Fixture;
  }
}

async function open(
  page: Page,
  options: {
    platform?: string;
    supported?: boolean;
    available?: boolean;
    listError?: boolean;
    empty?: boolean;
  } = {},
) {
  await page.addInitScript(
    (platform) =>
      Object.defineProperty(navigator, "platform", {
        value: platform,
        configurable: true,
      }),
    options.platform ?? "Win32",
  );
  await installMockBridge(page, {}, { relayWsUrl: "wss://community.example" });
  await page.goto("/");
  await page.getByTestId("open-agents-view").waitFor();
  await page.evaluate((options) => {
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
    const host = (g: string, h: string, label = "Workshop"): Host => ({
      scope: {
        community: "https://community.example",
        generation: g.repeat(32),
        host: h.repeat(32),
        owner: "deadbeef".repeat(8),
      },
      state: "paired",
      label,
      // Even cached legacy capability fields must not become registration UI.
      requests: [],
      runtimes: ["fixture/hidden-runtime"],
      catalog_revision: "ab".repeat(32),
      capacity: 4,
      fresh: true,
      inventory_id: g.repeat(32),
      fresh_for_ms: 60_000,
    });
    const fixture: Fixture = {
      supported: options.supported ?? true,
      available: options.available ?? true,
      listError: options.listError ?? false,
      hosts: options.empty ? [] : [host("31", "11"), host("32", "22")],
      calls: [],
    };
    window.__PAIRED_HOSTS_FIXTURE__ = fixture;
    api.invoke = async (command, args) => {
      if (!command.startsWith("paired_host_")) return previous(command, args);
      fixture.calls.push({ command, args: structuredClone(args) });
      if (
        args?.expectedRelay !== "wss://community.example" ||
        args?.expectedOwner !== "deadbeef".repeat(8)
      )
        throw new Error("wrong active registration session");
      if (command === "paired_host_list") {
        if (fixture.listError) throw new Error("Fixture connection failed");
        return {
          supported: fixture.supported,
          available: fixture.available,
          hosts: structuredClone(fixture.hosts),
        };
      }
      if (command === "paired_host_pair") {
        if (args.invitation === "expired-fixture-input")
          throw new Error("Invitation expired");
        if (args.invitation !== "private-console-test-input")
          throw new Error("wrong invitation");
        const pending = {
          ...host("33", "44", "Added host"),
          state: "pair-pending",
        };
        fixture.hosts.push(pending);
        return structuredClone(pending);
      }
      if (
        command === "paired_host_refresh" ||
        command === "paired_host_unpair"
      ) {
        const selected = fixture.hosts.find(
          (h) =>
            h.scope.host === args.host &&
            h.scope.generation === args.generation,
        );
        if (!selected) throw new Error("wrong host coordinate");
        if (command === "paired_host_unpair") selected.state = "unpair-pending";
        return structuredClone(selected);
      }
      // No successful provisioning/history mock remains to mask accidental IPC.
      throw new Error(`unexpected registration IPC: ${command}`);
    };
  }, options);
  await page.getByTestId("open-agents-view").click();
  await expect(page.getByTestId("agents-page-content")).toBeVisible();
}

async function calls(page: Page, command?: string) {
  return page.evaluate(
    (command) =>
      window.__PAIRED_HOSTS_FIXTURE__.calls.filter(
        (call) => command === undefined || call.command === command,
      ),
    command,
  );
}

async function registrationOnly(page: Page, panel: Locator) {
  await expect(
    panel.getByRole("button", {
      name: /^(Add agent|Create|Resume request|Request history|Check host)$/,
    }),
  ).toHaveCount(0);
  await expect(panel.getByLabel("Host for new agent")).toHaveCount(0);
  await expect(
    panel.getByRole("form", { name: "Create an agent on this host" }),
  ).toHaveCount(0);
  await expect(panel).not.toContainText("fixture/hidden-runtime");
  await expect(panel).not.toContainText("Installed runtimes:");
  await expect(panel).not.toContainText("Host confirmed");
  await expect(
    panel.getByText(/^(online|host online|available)$/i),
  ).toHaveCount(0);
  await expect(panel).not.toContainText("inventory cached");
  expect(
    (await calls(page))
      .map((call) => call.command)
      .filter(
        (command) =>
          ![
            "paired_host_list",
            "paired_host_pair",
            "paired_host_refresh",
            "paired_host_unpair",
          ].includes(command),
      ),
  ).toEqual([]);
}

test("Windows routes registration refresh and revoke by identity despite identical labels", async ({
  page,
}) => {
  await open(page);
  const panel = page.getByRole("region", { name: "My hosts" });
  const rows = panel.getByRole("listitem");
  await expect(rows).toHaveCount(2);
  await expect(rows.first()).toContainText("Workshop");
  await expect(rows.nth(1)).toContainText("Workshop");
  const selected = {
    expectedRelay: "wss://community.example",
    expectedOwner: "deadbeef".repeat(8),
    host: "22".repeat(32),
    generation: "32".repeat(32),
  };
  await rows
    .nth(1)
    .getByRole("button", { name: "Refresh registration" })
    .click();
  await expect
    .poll(() => calls(page, "paired_host_refresh"))
    .toEqual([{ command: "paired_host_refresh", args: selected }]);
  await expect(rows.nth(1)).toContainText("Registered");
  await rows
    .nth(1)
    .getByRole("button", { name: "Revoke registration" })
    .click();
  await rows
    .nth(1)
    .getByRole("button", { name: "Cancel", exact: true })
    .click();
  expect(await calls(page, "paired_host_unpair")).toEqual([]);
  await rows
    .nth(1)
    .getByRole("button", { name: "Revoke registration" })
    .click();
  await rows.nth(1).getByRole("button", { name: "Confirm revoke" }).click();
  await expect(rows.nth(1)).toContainText("Revocation pending");
  expect(await calls(page, "paired_host_unpair")).toEqual([
    { command: "paired_host_unpair", args: selected },
  ]);
  await expect(rows.first()).toContainText("Registered");
  await page.evaluate(() => {
    window.__PAIRED_HOSTS_FIXTURE__.hosts[1].state = "unpaired";
  });
  await expect(rows.nth(1)).toContainText("Revoked", { timeout: 10_000 });
  await expect(rows.first()).toContainText("Registered");
  await expect(
    rows.nth(1).getByRole("button", { name: "Revoke registration" }),
  ).toHaveCount(0);
  await registrationOnly(page, panel);
});

test("empty supported Windows directory offers Add Host, cancel, expired retry and registration", async ({
  page,
}) => {
  await open(page, { empty: true });
  const panel = page.getByRole("region", { name: "My hosts" });
  const rows = panel.getByRole("listitem");
  await expect(panel).toContainText("No registered hosts");
  await panel.getByRole("button", { name: "Add Host", exact: true }).click();
  const input = panel.getByLabel("Private host invitation");
  await expect(input).toHaveAttribute("type", "password");
  await input.fill("cancelled-fixture-input");
  await panel.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(await calls(page, "paired_host_pair")).toEqual([]);
  await panel.getByRole("button", { name: "Add Host", exact: true }).click();
  await expect(input).toHaveValue("");
  await input.fill("expired-fixture-input");
  await input.press("Enter");
  await expect(panel.getByRole("alert")).toHaveText("Invitation expired");
  await expect(input).toHaveValue("");
  await expect(rows).toHaveCount(0);
  await input.fill("private-console-test-input");
  await panel.getByRole("button", { name: "Submit invitation" }).click();
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("Registration pending");
  await expect(input).toHaveCount(0);
  expect(
    (await calls(page, "paired_host_pair")).map(
      (call) => call.args?.invitation,
    ),
  ).toEqual(["expired-fixture-input", "private-console-test-input"]);
  await page.evaluate(() => {
    window.__PAIRED_HOSTS_FIXTURE__.hosts[0].state = "paired";
  });
  await expect(rows.first()).toContainText("Registered", { timeout: 10_000 });
  await registrationOnly(page, panel);
});

test("unsupported Windows community explains registration support without offering Add Host", async ({
  page,
}) => {
  await open(page, { supported: false, empty: true });
  const panel = page.getByRole("region", { name: "My hosts" });
  await expect(panel).toContainText(
    "Host registration is not supported by this community.",
  );
  await expect(
    panel.getByRole("button", { name: "Add Host", exact: true }),
  ).toHaveCount(0);
  await registrationOnly(page, panel);
});

for (const listError of [false, true]) {
  test(`Windows ${listError ? "failed" : "unavailable"} directory retains Retry without authorizing Add Host`, async ({
    page,
  }) => {
    await open(page, { available: false, listError, empty: true });
    const panel = page.getByRole("region", { name: "My hosts" });
    await expect(panel).toContainText("Community unavailable.");
    await expect(
      panel.getByRole("button", { name: "Add Host", exact: true }),
    ).toHaveCount(0);
    await expect(panel).not.toContainText("No registered hosts");
    if (listError)
      await expect(panel.getByRole("alert")).toHaveText(
        "Fixture connection failed",
      );
    await page.evaluate(() => {
      window.__PAIRED_HOSTS_FIXTURE__.available = true;
      window.__PAIRED_HOSTS_FIXTURE__.listError = false;
    });
    await panel.getByRole("button", { name: "Retry", exact: true }).click();
    await expect(panel).toContainText("No registered hosts");
    await expect(
      panel.getByRole("button", { name: "Add Host", exact: true }),
    ).toBeVisible();
    await expect(panel.getByRole("alert")).toHaveCount(0);
    await registrationOnly(page, panel);
  });
}

test("other platforms keep the stock Agents surface without registration IPC", async ({
  page,
}) => {
  await open(page, { platform: "Linux x86_64" });
  await expect(page.getByRole("region", { name: "My hosts" })).toHaveCount(0);
  expect(await calls(page)).toEqual([]);
});

async function openOrdinaryCreate(page: Page) {
  await page.getByTestId("new-agent-card").click();
  await page.locator("#persona-display-name").fill("Hosted fixture");
  await page.getByRole("button", { name: "Advanced", exact: true }).click();
  await expect(page.locator("#agent-run-on")).toBeVisible();
}

test("ordinary Add Agent distinguishes registered hosts and refuses submission even with legacy capability data", async ({
  page,
}) => {
  await open(page);
  await openOrdinaryCreate(page);
  await page.locator("#agent-run-on").press("Enter");
  const hosts = page.getByRole("menuitemradio", { name: /Workshop/ });
  await expect(hosts).toHaveCount(2);
  const labels = await hosts.allTextContents();
  expect(labels[0]).not.toEqual(labels[1]);
  await hosts.nth(1).click();
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: "Host runtime metadata is checked" }),
  ).toContainText("Models remain pending");
  await expect(
    page.getByRole("button", { name: "Prepare agent", exact: true }),
  ).toBeDisabled();
  // Bypass the disabled-button affordance: actual form submission must still
  // stop before persona publication, key creation or provider deployment.
  await page
    .locator("#persona-display-name")
    .evaluate((input) =>
      input
        .closest("form")
        ?.dispatchEvent(
          new Event("submit", { bubbles: true, cancelable: true }),
        ),
    );
  await page.waitForTimeout(100);
  const effects = await page.evaluate(
    () =>
      window.__BUZZ_E2E_COMMAND_LOG__?.filter(({ command }) =>
        /^(create_persona|create_managed_agent|deploy_to_provider|probe_backend_provider|paired_host_create)$/.test(
          command,
        ),
      ) ?? [],
  );
  expect(effects).toEqual([]);
  expect(
    (await calls(page)).every((c) => c.command === "paired_host_list"),
  ).toBe(true);
  await page.locator("#agent-run-on").press("Enter");
  await page
    .getByRole("menuitemradio", { name: "This computer", exact: true })
    .click();
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: "Host runtime metadata is checked" }),
  ).toHaveCount(0);
});

test("ordinary Add Agent keeps Add Host discoverable in an empty directory", async ({
  page,
}) => {
  await open(page, { empty: true });
  await openOrdinaryCreate(page);
  const form = page
    .locator("#persona-display-name")
    .locator("xpath=ancestor::form");
  await expect(form).toContainText("No registered hosts.");
  await form.getByRole("button", { name: "Add Host", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Add Host", exact: true });
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "Add Host", exact: true }).click();
  await expect(dialog.getByLabel("Private host invitation")).toBeVisible();
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(await calls(page, "paired_host_pair")).toEqual([]);
  await dialog.getByRole("button", { name: "Add Host", exact: true }).click();
  await dialog
    .getByLabel("Private host invitation")
    .fill("expired-fixture-input");
  await dialog.getByRole("button", { name: "Submit invitation" }).click();
  await expect(dialog.getByRole("alert")).toHaveText("Invitation expired");
  expect(
    await page.evaluate(
      () =>
        window.__BUZZ_E2E_COMMAND_LOG__?.filter(({ command }) =>
          /^(create_persona|create_managed_agent)$/.test(command),
        ) ?? [],
    ),
  ).toEqual([]);
});

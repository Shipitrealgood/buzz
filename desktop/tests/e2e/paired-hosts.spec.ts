import { expect, test, type Page } from "@playwright/test";
import { installMockBridge } from "../helpers/bridge";

async function open(page: Page, supported: boolean, platform = "Win32") {
  await installMockBridge(page, {}, { relayWsUrl: "wss://community.example" });
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
      state: "paired",
      fresh: false,
      label: "Workshop",
      runtimes: ["test/echo"],
    }));
    api.invoke = async (command, args) => {
      if (command === "paired_host_list") {
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
      if (command === "paired_host_refresh") {
        const host = hosts.find(
          (h) =>
            h.scope.host === args?.host &&
            h.scope.generation === args?.generation,
        );
        if (!host) throw new Error("wrong host");
        return { ...host, fresh: true };
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

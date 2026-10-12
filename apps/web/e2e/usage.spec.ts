import { expect, test } from "@playwright/test";

test("usage overview renders verified totals and partial pricing", async ({ page }) => {
  await page.goto("/#/usage");
  await expect(page.getByRole("heading", { name: "Usage", exact: true })).toBeVisible();
  await expect(page.getByTestId("cost-30d")).toContainText("$12.40");
  await expect(page.getByText("2 of 4 sessions", { exact: false }).first()).toBeVisible();
  await expect(page.getByText("computed", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("heading", { name: "Cost by model" })).toBeVisible();
  await expect(page.getByRole("link", { name: "fixture-session-claude" })).toBeVisible();
  await page.screenshot({ path: "test-results/usage-after.png", fullPage: true });
});

test("session drilldown exposes the subagent tree and event links", async ({ page }) => {
  await page.goto("/#/usage/fixture-session-claude");
  await expect(page.getByTestId("session-drilldown")).toBeVisible();
  await expect(page.getByText("fixture-subagent-build", { exact: true })).toBeVisible();
  await expect(page.getByText("timeline ↗", { exact: true }).first()).toHaveAttribute("href", /timeline/);
  await page.screenshot({ path: "test-results/session-after.png", fullPage: true });
});

test("connections shows a truthful first-run state when no rows are returned", async ({ page }) => {
  await page.route("**/__trace-fixtures/connections", async (route) => {
    await route.fulfill({ json: [] });
  });
  await page.goto("/#/connections");
  await expect(page.getByRole("heading", { name: "Connections" })).toBeVisible();
  await expect(page.getByTestId("connections-empty")).toBeVisible();
  await expect(page.getByText("trc integrations install all", { exact: true })).toBeVisible();
  await expect(page.getByText("trc run \"claude\"", { exact: true })).toBeVisible();
  await page.screenshot({ path: "test-results/connections-after.png", fullPage: true });
});

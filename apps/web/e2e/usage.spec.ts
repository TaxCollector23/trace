import { expect, test } from "@playwright/test";

test("usage overview renders verified totals and partial pricing", async ({ page }) => {
  await page.route("**/__trace-fixtures/usage**", async (route) => {
    const response = await route.fetch();
    const body = await response.json();
    const model = body.by_model?.find((row: { key: string }) => row.key === "claude-3-5-sonnet-20241022");
    if (model) model.api_equivalent_usd = 1.23;
    await route.fulfill({ response, json: body });
  });
  await page.route("**/api/pricing", async (route) => {
    await route.fulfill({
      json: {
        api: {
          pricing_version: "test-rates",
          effective_from: "2026-10-13T00:00:00Z",
          sources: [],
          models: [{
            provider: "anthropic",
            model: "claude-3-5-sonnet-20241022",
            aliases: [],
            effective_from: "2026-10-13T00:00:00Z",
            tiers: [{
              name: "standard",
              min_input_tokens: null,
              input_per_mtok: 3,
              output_per_mtok: 15,
              cache_read_per_mtok: 0.3,
              cache_write_per_mtok: {},
              reasoning_billing: "included",
              notes: null,
            }],
          }],
        },
        subscriptions: [{
          provider: "anthropic",
          plan_id: "claude-pro",
          plan_name: "Claude Pro",
          monthly_price_usd: 20,
          included_api_usage_usd: null,
          region: "US",
          currency: "USD",
          pricing_version: "test-rates",
          effective_from: "2026-10-13T00:00:00Z",
          source_url: "https://example.test/pricing",
          retrieved_at: "2026-10-13",
          source_notes: "test fixture",
        }],
      },
    });
  });
  await page.goto("/#/usage");
  await expect(page.getByRole("heading", { name: "Usage", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Subscription rates" })).toBeVisible();
  await expect(page.getByRole("columnheader", { name: "Included API usage / month" })).toBeVisible();
  await expect(page.getByText("Claude Pro", { exact: true })).toBeVisible();
  await expect(page.getByRole("row", { name: /Claude Pro/ }).getByRole("cell").nth(2)).toHaveText("$1.23");
  await expect(page.getByRole("heading", { name: "API billing rates" })).toBeVisible();
  await expect(page.getByRole("table").nth(1).getByRole("cell", { name: "$3.00" })).toBeVisible();
  await expect(page.getByTestId("cost-30d")).toContainText("$12.40");
  await expect(page.getByRole("heading", { name: "Cost by model" })).not.toBeVisible();
  await page.getByText("Usage details", { exact: true }).click();
  await expect(page.getByRole("heading", { name: "Most expensive sessions" })).toBeVisible();
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

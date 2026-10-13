import { expect, test } from "@playwright/test";

const now = new Date().toISOString();
const runningRun = {
  id: "real-run-from-api",
  project_id: "project-1",
  project_name: "trace",
  command: "Agent session",
  agent_name: "codex",
  user_prompt: null,
  started_at: now,
  ended_at: null,
  starting_commit: null,
  ending_commit: null,
  status: "running",
  exit_code: null,
  created_at: now,
  files_changed: 2,
  command_count: 3,
  passed_commands: 3,
  dangerous_commands: 0,
  secret_warnings: 0,
  estimated_cost: null,
  checks_status: null,
};

test("overview moves from honest idle state to a live run and keeps it after a refresh failure", async ({ page }) => {
  let phase: "idle" | "running" | "failed" = "idle";
  await page.route("**/api/dashboard", async (route) => {
    if (phase === "failed") {
      await route.fulfill({ status: 503, body: "daemon unavailable" });
      return;
    }
    await route.fulfill({
      json: {
        runs: phase === "running" ? [runningRun] : [],
        projects: [{ id: "project-1", name: "trace", path: "/tmp/trace", config_path: "/tmp/trace/.trace/config.toml", created_at: now, updated_at: now }],
        integrations: phase === "running" ? [{ id: "codex", name: "Codex", connected: true }] : [],
      },
    });
  });

  await page.goto("/#/");
  await expect(page.getByRole("heading", { name: "No agent is running" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "No agent integrations configured" })).toBeVisible();
  await page.screenshot({ path: "test-results/dashboard-idle.png", fullPage: true });

  phase = "running";
  await expect(page.getByRole("heading", { name: "Codex is working in trace" })).toBeVisible({ timeout: 7000 });
  await expect(page.getByRole("link", { name: "Open timeline" })).toHaveAttribute("href", "#/timeline/real-run-from-api");
  await expect(page.getByText("unavailable", { exact: true }).first()).toBeVisible();
  await page.screenshot({ path: "test-results/dashboard-active.png", fullPage: true });

  phase = "failed";
  await expect(page.getByText("The last refresh failed", { exact: false })).toBeVisible({ timeout: 7000 });
  await expect(page.getByRole("heading", { name: "Codex was working in trace" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Open timeline" })).toBeVisible();
});

test("overview explains an unreachable local daemon", async ({ page }) => {
  await page.route("**/api/dashboard", (route) => route.fulfill({ status: 503, body: "unavailable" }));
  await page.goto("/#/");
  await expect(page.getByText("The local daemon is not responding", { exact: false })).toBeVisible();
  await expect(page.getByRole("button", { name: "Retry" })).toBeVisible();
});

test("overview remains readable on a narrow dark screen", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.route("**/api/dashboard", (route) => route.fulfill({
    json: {
      runs: [runningRun],
      projects: [],
      integrations: [{ id: "codex", name: "Codex", connected: true }],
    },
  }));
  await page.goto("/#/");
  await page.getByRole("button", { name: "Dark" }).click();
  const geistLoaded = await page.evaluate(async () => {
    await document.fonts.load("14px Geist");
    return document.fonts.check("14px Geist");
  });
  expect(geistLoaded).toBe(true);
  await expect(page.getByRole("heading", { name: "Codex is working in trace" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Open timeline" })).toBeVisible();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
  expect(overflow).toBe(false);
  await page.screenshot({ path: "test-results/dashboard-mobile-dark.png", fullPage: true });
});

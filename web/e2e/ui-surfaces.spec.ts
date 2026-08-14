import { expect, test, type Locator, type Page } from "@playwright/test";

async function boot(page: Page): Promise<void> {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/?seed=4242");
  await page.waitForFunction(() => window.__understory?.slotPoint !== undefined, null, {
    timeout: 20_000,
  });
  await page.getByTestId("speed-Paused").click();
}

async function material(locator: Locator) {
  return locator.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      backgroundImage: style.backgroundImage,
      borderTopWidth: style.borderTopWidth,
      borderTopStyle: style.borderTopStyle,
      boxShadow: style.boxShadow,
    };
  });
}

function expectFinishedSurface(surface: Awaited<ReturnType<typeof material>>, label: string): void {
  expect(surface.backgroundImage, `${label} has no material texture`).not.toBe("none");
  expect(Number.parseFloat(surface.borderTopWidth), `${label} has no bezel`).toBeGreaterThan(0);
  expect(surface.borderTopStyle, `${label} bezel is not drawn`).not.toBe("none");
  expect(surface.boxShadow, `${label} has no physical depth`).not.toBe("none");
}

async function expectNoRawButtons(page: Page, label: string): Promise<void> {
  const raw = await page.locator("button:visible").evaluateAll((buttons) =>
    buttons.flatMap((button) => {
      if (button.classList.contains("elegy-copy")) return [];
      const style = getComputedStyle(button);
      const finished =
        style.backgroundImage !== "none" &&
        Number.parseFloat(style.borderTopWidth) > 0 &&
        style.borderTopStyle !== "none" &&
        style.boxShadow !== "none";
      return finished ? [] : [button.className || button.textContent?.trim() || "unnamed button"];
    }),
  );
  expect(raw, `${label} contains unskinned buttons`).toEqual([]);
}

test("skeuomorphic HUD materials and key states", async ({ page }) => {
  await boot(page);

  for (const [label, locator] of [
    ["top housing", page.locator(".topbar")],
    ["build housing", page.locator(".sidebar")],
    ["roster housing", page.getByTestId("roster")],
    ["CRT readout", page.locator(".readout").first()],
    ["journey glass", page.getByTestId("journey")],
    ["build card", page.locator(".build-card").first()],
    ["crew card", page.locator(".roster-row").first()],
  ] as const) {
    expectFinishedSurface(await material(locator), label);
  }

  const pressed = page.getByTestId("speed-Paused");
  const raised = page.getByTestId("speed-X1");
  await expect(pressed).toHaveAttribute("aria-pressed", "true");
  await expect(raised).toHaveAttribute("aria-pressed", "false");
  expect((await material(pressed)).backgroundImage).not.toBe(
    (await material(raised)).backgroundImage,
  );

  await page.getByTestId("zoom-in").focus();
  await expect(page.getByTestId("zoom-in")).toBeFocused();
  await expectNoRawButtons(page, "HUD");
  await page.screenshot({ path: "capture/ui-surfaces-hud.png" });
});

test("utility shaft cards explain their spatial jobs before they are affordable", async ({
  page,
}) => {
  await boot(page);

  await expect(page.getByTestId("build-shaft.busbar")).toContainText(
    "primary 12 charge/tick trunk between floors",
  );
  await expect(page.getByTestId("build-shaft.vent_stack")).toContainText(
    "hot rooms must touch · must reach the roof",
  );
  await expect(page.getByTestId("build-shaft.chute")).toContainText(
    "automatic relief for surplus in a touching storeroom",
  );
});

test("pressed, disabled, and room-selection console states", async ({ page }) => {
  await boot(page);

  const placing = page.getByTestId("build-room.garden");
  const disabled = page.getByTestId("build-shaft.elevator");
  await expect(disabled).toBeDisabled();
  await placing.click();
  await expect(placing).toHaveAttribute("aria-pressed", "true");

  const resting = await material(page.getByTestId("build-room.bunk"));
  const seated = await material(placing);
  expect(seated.backgroundImage).not.toBe(resting.backgroundImage);
  expect(seated.boxShadow).not.toBe(resting.boxShadow);
  expect(await disabled.evaluate((button) => getComputedStyle(button).opacity)).not.toBe("1");

  await page.screenshot({ path: "capture/ui-surfaces-button-states.png" });

  // The centre of an occupied room is renderer-owned, so the click remains
  // valid as framing and tower width change.
  const roomPoint = await page.evaluate(() => {
    const hooks = window.__understory!;
    for (const floor of hooks.view().tower.floors) {
      const room = floor.rooms[0];
      if (room) return hooks.slotPoint(floor.index, room.slot + room.width / 2);
    }
    return null;
  });
  expect(roomPoint).not.toBeNull();

  // An occupied slot is an honest command failure. It proves the notification
  // surface without fabricating React state or adding a test-only UI route.
  await page.mouse.click(roomPoint!.x, roomPoint!.y);
  const toast = page.getByTestId("command-error");
  await expect(toast).toBeVisible();
  expectFinishedSurface(await material(toast), "command error terminal");

  // Drop placement before selecting the room itself.
  await page.getByTestId("game-canvas").click({ button: "right" });
  await page.mouse.click(roomPoint!.x, roomPoint!.y);
  await expect(page.getByTestId("selection")).toBeVisible();
  await expect(page.getByTestId("remove-room")).toBeVisible();
  expectFinishedSurface(await material(page.getByTestId("remove-room")), "selection action");
  await expectNoRawButtons(page, "room selection");
  await page.locator(".sidebar").screenshot({ path: "capture/ui-surfaces-selection.png" });
});

test("chain panel carries the console material language", async ({ page }) => {
  await boot(page);
  await page.getByTestId("chain-toggle").click();

  const economy = page.getByTestId("economy");
  await expect(economy).toBeVisible();
  expectFinishedSurface(await material(economy), "chain housing");
  expectFinishedSurface(await material(page.locator(".economy-item").first()), "chain item card");
  await expect(page.locator(".economy-flow")).not.toHaveCount(0);

  const bounds = await economy.boundingBox();
  expect(bounds).not.toBeNull();
  expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(1600);
  expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(900);
  await expectNoRawButtons(page, "chain panel");
  await page.screenshot({ path: "capture/ui-surfaces-chain.png" });
});

test("journey boards and arrival use the completed console finish", async ({ page }) => {
  await boot(page);

  const enclave = await page.evaluate(() => {
    const hooks = window.__understory!;
    hooks.send({ SetStriding: { walking: true } });
    let budget = 260_000;
    while (budget > 0) {
      const view = hooks.view();
      if (view.journey.arrived) return false;
      const fork = view.journey.fork;
      if (fork && fork.answer === null) hooks.send({ TakeFork: { branch: 0 } });
      const close = view.journey.region >= 1 && view.journey.region_permille >= 150;
      if (close) {
        hooks.send({ SetStriding: { walking: false } });
        hooks.step(2);
        if (hooks.view().journey.at_enclave) return true;
        hooks.send({ SetStriding: { walking: true } });
      }
      const stride = close ? 60 : 600;
      hooks.step(stride);
      budget -= stride;
    }
    return false;
  });
  expect(enclave).toBe(true);
  await expect(page.getByTestId("enclave")).toBeVisible();
  expectFinishedSurface(await material(page.getByTestId("enclave")), "enclave housing");
  expectFinishedSurface(await material(page.locator(".enclave-offer").first()), "trade card");
  expectFinishedSurface(await material(page.getByTestId("recruit")), "recruit key");
  const reinforce = page.getByTestId("reinforce");
  if ((await reinforce.count()) > 0) {
    expectFinishedSurface(await material(reinforce), "reinforce key");
  }
  await expectNoRawButtons(page, "enclave board");
  await page.screenshot({ path: "capture/ui-surfaces-enclave.png" });

  await page.setViewportSize({ width: 1200, height: 760 });
  const enclaveLayout = await page.evaluate(() => {
    const board = document.querySelector(".enclave")?.getBoundingClientRect() ?? null;
    const roster = document.querySelector(".roster")?.getBoundingClientRect() ?? null;
    return {
      overflow: document.documentElement.scrollWidth > window.innerWidth,
      overlap:
        board !== null &&
        roster !== null &&
        board.left < roster.right &&
        board.right > roster.left &&
        board.top < roster.bottom &&
        board.bottom > roster.top,
    };
  });
  expect(enclaveLayout.overflow).toBe(false);
  expect(enclaveLayout.overlap).toBe(false);
  await page.screenshot({ path: "capture/ui-surfaces-enclave-compact.png" });
  await page.setViewportSize({ width: 1600, height: 900 });

  const fork = await page.evaluate(() => {
    const hooks = window.__understory!;
    hooks.send({ SetStriding: { walking: true } });
    let budget = 100_000;
    while (budget > 0) {
      const view = hooks.view();
      if (view.journey.arrived) return false;
      if (view.journey.fork && view.journey.fork.answer === null) {
        if (view.journey.halt === "fork") return true;
        hooks.step(30);
        budget -= 30;
        continue;
      }
      hooks.step(300);
      budget -= 300;
    }
    return false;
  });
  expect(fork).toBe(true);
  await expect(page.getByTestId("fork")).toBeVisible();
  expectFinishedSurface(await material(page.getByTestId("fork")), "fork housing");
  expectFinishedSurface(await material(page.locator(".fork-way").first()), "fork route key");
  await expectNoRawButtons(page, "fork board");
  await page.screenshot({ path: "capture/ui-surfaces-fork.png" });

  const arrived = await page.evaluate(() => {
    const hooks = window.__understory!;
    const pending = hooks.view().journey.fork;
    if (pending && pending.answer === null) hooks.send({ TakeFork: { branch: 0 } });
    hooks.send({ SetStriding: { walking: true } });
    let budget = 180_000;
    while (budget > 0 && !hooks.view().journey.arrived) {
      const nextFork = hooks.view().journey.fork;
      if (nextFork && nextFork.answer === null) hooks.send({ TakeFork: { branch: 0 } });
      hooks.step(60);
      budget -= 60;
    }
    return hooks.view().journey.arrived;
  });
  expect(arrived).toBe(true);
  await expect(page.getByTestId("arrival")).toBeVisible();
  await page.waitForTimeout(2700);
  expectFinishedSurface(await material(page.locator(".elegy-facts")), "arrival facts terminal");
  expectFinishedSurface(await material(page.locator(".elegy-again")), "walk-again key");
  await expectNoRawButtons(page, "arrival");
  await page.screenshot({ path: "capture/ui-surfaces-arrival.png" });
});

test("boot failure is housed in the same field console", async ({ page }) => {
  await page.route("**/*understory_bridge_bg*.wasm", (route) => route.abort());
  await page.goto("/?seed=4242");
  const failure = page.locator(".boot-fail");
  await expect(failure).toBeVisible({ timeout: 20_000 });
  expectFinishedSurface(await material(failure.locator("h1")), "boot failure nameplate");
  expectFinishedSurface(await material(failure.locator("pre")), "boot diagnostic well");
  await page.screenshot({ path: "capture/ui-surfaces-boot-fail.png" });
});

test("the waking screen uses a CRT status plate", async ({ page }) => {
  await page.route("**/*understory_bridge_bg*.wasm", async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 1_200));
    await route.continue();
  });
  await page.goto("/?seed=4242", { waitUntil: "domcontentloaded" });
  const status = page.locator(".boot-status");
  await expect(status).toBeVisible();
  expectFinishedSurface(await material(status), "boot status terminal");
  await page.screenshot({ path: "capture/ui-surfaces-boot.png" });
});

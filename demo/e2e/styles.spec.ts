import { expect, test } from "./fixtures";

test("style customization guide loads directly and through navigation", async ({
  openRoute,
  page,
}) => {
  await openRoute("/styles", "Make audio UI belong to your application");

  const navigationEntry = page.getByRole("link", {
    name: "Style customization",
  });
  await expect(navigationEntry).toHaveAttribute("aria-current", "page");

  await page.goto("/");
  await navigationEntry.click();
  await expect(page).toHaveURL(/\/styles$/);
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: "Make audio UI belong to your application",
    }),
  ).toBeVisible();

  await openRoute("/styles-prototype?variant=reference", "Page not found");
  await expect(
    page.getByRole("navigation", { name: "Style guide prototype variants" }),
  ).toHaveCount(0);
});

test("guide introduces setup and cascade before the Studio chapter", async ({
  openRoute,
  page,
}) => {
  await openRoute("/styles", "Make audio UI belong to your application");

  const setup = page.getByRole("region", { name: "Stylesheet setup" });
  await expect(setup).toContainText("AudioStyles");
  await expect(setup).toContainText("STYLESHEET");
  await expect(setup.locator("pre")).toBeVisible();
  const setupSource = await setup.locator("pre code").textContent();
  expect(setupSource?.match(/AudioStyles \{\}/g)).toHaveLength(1);
  expect(
    setupSource?.match(/document::Stylesheet \{ href: STYLESHEET \}/g),
  ).toHaveLength(1);

  const cascade = page.getByRole("region", { name: "How the cascade resolves" });
  await expect(cascade).toContainText("--dioxus-audio-*");
  await expect(cascade).toContainText("daisyUI");
  await expect(cascade).toContainText("standalone default");

  const steps = page.locator("ol.styles-orientation > li");
  await expect(steps).toHaveCount(3);
  await expect(steps.nth(0)).toContainText("Brand the application");
  await expect(steps.nth(1)).toContainText("Scope an instance");
  await expect(steps.nth(2)).toContainText("Use theme fallbacks");

  const studio = page.getByRole("region", {
    name: "Studio: one complete app-wide theme",
  });
  await expect(studio.getByText("What to notice", { exact: true })).toBeVisible();
  await expect(studio.getByText("Live demonstration", { exact: true })).toBeVisible();
  await expect(studio.getByText("Exact source recipe", { exact: true })).toBeVisible();
  await expect(studio.locator("pre")).toHaveCount(2);
});

test("Studio tokens inherit through every audio component without leaking", async ({
  openRoute,
  page,
}) => {
  await openRoute("/styles", "Make audio UI belong to your application");

  const expectedTokens = {
    "--dioxus-audio-base-100": "#fffaf2",
    "--dioxus-audio-base-200": "#f2e9dc",
    "--dioxus-audio-base-300": "#d7c8b7",
    "--dioxus-audio-content": "#241c2f",
    "--dioxus-audio-primary": "#7446e8",
    "--dioxus-audio-primary-content": "#fff",
    "--dioxus-audio-warning": "#b86813",
    "--dioxus-audio-error": "#c83d61",
    "--dioxus-audio-success": "#2f8464",
    "--dioxus-audio-radius": "1.15rem",
  } as const;
  const tokenNames = Object.keys(expectedTokens);
  const studio = page.locator(".studio-app");
  const themedElements = [
    studio,
    page.getByRole("combobox", { name: "Recording input" }),
    page.getByRole("status").filter({ hasText: "Microphone ready" }),
    page.getByRole("img", { name: "Morning field notes waveform" }),
    page.getByRole("button", { name: "Play", exact: true }),
  ];

  for (const element of themedElements) {
    await expect(element).toBeVisible();
    const values = await element.evaluate((node, names) => {
      const styles = getComputedStyle(node);
      return Object.fromEntries(
        names.map((name) => [name, styles.getPropertyValue(name).trim()]),
      );
    }, tokenNames);
    expect(values).toEqual(expectedTokens);
  }

  const shellValues = await page.locator("main").evaluate((node, names) => {
    const styles = getComputedStyle(node);
    return Object.fromEntries(
      names.map((name) => [name, styles.getPropertyValue(name).trim()]),
    );
  }, tokenNames);
  expect(shellValues).toEqual(
    Object.fromEntries(tokenNames.map((name) => [name, ""])),
  );
});

test("Studio preview states preserve input selection and generated audio plays", async ({
  openRoute,
  page,
}) => {
  await openRoute("/styles", "Make audio UI belong to your application");

  const studio = page.locator(".studio-app");
  const input = studio.getByRole("combobox", { name: "Recording input" });
  await expect(input).toBeEnabled();
  await expect(input.locator("option")).not.toHaveCount(1);
  await input.selectOption({ index: 1 });
  const selectedInput = await input.inputValue();

  const preview = studio.getByRole("group", {
    name: "Preview microphone state",
  });
  const states = [
    ["Ready", "Microphone ready"],
    ["Recording", "Recording"],
    ["Muted", "Microphone muted by the device"],
    ["Denied", "Microphone access denied"],
  ] as const;

  for (const [buttonName, statusText] of states) {
    const button = preview.getByRole("button", { name: buttonName });
    await button.click();
    await expect(button).toHaveAttribute("aria-pressed", "true");
    await expect(
      preview.locator('button[aria-pressed="true"]'),
    ).toHaveCount(1);
    await expect(
      studio.getByRole("status").filter({ hasText: statusText }),
    ).toHaveText(statusText);
    await expect(input).toHaveValue(selectedInput);
  }

  await expect(
    studio.getByRole("img", { name: "Morning field notes waveform" }),
  ).toBeVisible();
  await studio.getByRole("button", { name: "Play", exact: true }).click();
  const pause = studio.getByRole("button", { name: "Pause", exact: true });
  await expect(pause).toBeVisible();
  await pause.click();
  await expect(
    studio.getByRole("button", { name: "Play", exact: true }),
  ).toBeVisible();
});

test("Studio recipes are identical to independently fetched production sources", async ({
  openRoute,
  page,
}) => {
  await openRoute("/styles", "Make audio UI belong to your application");

  const studio = page.getByRole("region", {
    name: "Studio: one complete app-wide theme",
  });
  const rustRecipe = studio.getByRole("article").filter({
    hasText: "Rust composition",
  });
  const cssRecipe = studio.getByRole("article").filter({
    hasText: "Studio stylesheet",
  });

  const [rustHref, cssHref, renderedRust, renderedCss] = await Promise.all([
    rustRecipe.getByRole("link", { name: "View production source" }).getAttribute("href"),
    cssRecipe.getByRole("link", { name: "View production source" }).getAttribute("href"),
    rustRecipe.locator('code[data-recipe-language="rust"]').textContent(),
    cssRecipe.locator('code[data-recipe-language="css"]').textContent(),
  ]);
  expect(rustHref).toBeTruthy();
  expect(cssHref).toBeTruthy();

  const sources = await page.evaluate(async ([rustUrl, cssUrl]) => {
    const [rustResponse, cssResponse] = await Promise.all([
      fetch(rustUrl),
      fetch(cssUrl),
    ]);
    return {
      rust: { status: rustResponse.status, body: await rustResponse.text() },
      css: { status: cssResponse.status, body: await cssResponse.text() },
    };
  }, [rustHref!, cssHref!]);
  expect(sources.rust.status).toBe(200);
  expect(sources.css.status).toBe(200);
  expect(sources.rust.body).not.toContain("<title>dioxus-audio demo</title>");
  expect(sources.css.body).not.toContain("<title>dioxus-audio demo</title>");

  const startMarker = "// region: studio-recipe";
  const endMarker = "// endregion: studio-recipe";
  expect(sources.rust.body.split(startMarker)).toHaveLength(2);
  expect(sources.rust.body.split(endMarker)).toHaveLength(2);
  const start = sources.rust.body.indexOf(startMarker) + startMarker.length;
  const end = sources.rust.body.indexOf(endMarker, start);
  expect(end).toBeGreaterThan(start);
  const extractedRust = sources.rust.body
    .slice(start, end)
    .replace(/^\n/, "")
    .replace(/\n$/, "");
  expect(extractedRust.trim()).not.toBe("");
  expect(renderedRust).toBe(extractedRust);

  expect(renderedCss).toBe(sources.css.body);
  const authoredTokens = {
    "--dioxus-audio-base-100": "#fffaf2",
    "--dioxus-audio-base-200": "#f2e9dc",
    "--dioxus-audio-base-300": "#d7c8b7",
    "--dioxus-audio-content": "#241c2f",
    "--dioxus-audio-primary": "#7446e8",
    "--dioxus-audio-primary-content": "#ffffff",
    "--dioxus-audio-warning": "#b86813",
    "--dioxus-audio-error": "#c83d61",
    "--dioxus-audio-success": "#2f8464",
    "--dioxus-audio-radius": "1.15rem",
  } as const;
  for (const [name, value] of Object.entries(authoredTokens)) {
    expect(sources.css.body).toContain(`${name}: ${value};`);
  }

  const studioRuleStart = sources.css.body.indexOf(".studio-app {");
  const studioRuleEnd = sources.css.body.indexOf("\n  }", studioRuleStart);
  expect(studioRuleStart).toBeGreaterThanOrEqual(0);
  expect(studioRuleEnd).toBeGreaterThan(studioRuleStart);
  const studioRule = sources.css.body.slice(studioRuleStart, studioRuleEnd);
  expect(studioRule.match(/--dioxus-audio-[\w-]+:/g)).toHaveLength(10);
  const outsideStudioRule =
    sources.css.body.slice(0, studioRuleStart) +
    sources.css.body.slice(studioRuleEnd);
  expect(outsideStudioRule).not.toMatch(/--dioxus-audio-[\w-]+:/);
});

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

test("scoped chapter starts with otherwise identical clip editors", async ({
  openRoute,
  page,
}) => {
  await openRoute("/styles", "Make audio UI belong to your application");

  const studio = page.getByRole("region", {
    name: "Studio: one complete app-wide theme",
  });
  const scoped = page.getByRole("region", {
    name: "Citrus and Midnight: independently scoped clip editors",
  });
  await expect(scoped).toBeVisible();
  expect(
    await studio.evaluate(
      (studioNode, scopedNode) =>
        Boolean(studioNode.compareDocumentPosition(scopedNode) & Node.DOCUMENT_POSITION_FOLLOWING),
      await scoped.elementHandle(),
    ),
  ).toBe(true);
  await expect(scoped.getByText("What to notice", { exact: true })).toBeVisible();
  await expect(scoped.getByText("Live demonstration", { exact: true })).toBeVisible();
  await expect(scoped.getByText("Exact source recipe", { exact: true })).toBeVisible();

  const editors = [scoped.locator(".citrus"), scoped.locator(".midnight")];
  await expect(editors[0].getByRole("heading", { name: "Citrus" })).toBeVisible();
  await expect(editors[1].getByRole("heading", { name: "Midnight" })).toBeVisible();

  for (const editor of editors) {
    await expect(editor.getByText("Generated WAV Audio Data", { exact: true })).toBeVisible();
    await expect(editor.getByText("240 fixed Peaks", { exact: true })).toBeVisible();
    await expect(editor.getByText("2 second duration", { exact: true })).toBeVisible();
    await expect(editor.getByText("18.0% - 82.0%", { exact: true })).toBeVisible();
    await expect(editor.getByRole("slider", { name: "Selection start" })).toHaveValue("18");
    await expect(editor.getByRole("slider", { name: "Selection end" })).toHaveValue("82");
    await expect(editor.getByRole("button", { name: "Play", exact: true })).toBeEnabled();
  }
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
    studio.getByRole("button", { name: "Play", exact: true }),
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

test("scoped tokens stay local while range and Playback remain independent", async ({
  openRoute,
  page,
}) => {
  await openRoute("/styles", "Make audio UI belong to your application");

  const expectedThemes = {
    citrus: {
      "--dioxus-audio-base-100": "#fff8e8",
      "--dioxus-audio-base-200": "#f7e7c4",
      "--dioxus-audio-base-300": "#d9b979",
      "--dioxus-audio-content": "#422716",
      "--dioxus-audio-primary": "#c4561f",
      "--dioxus-audio-primary-content": "#fff8e8",
      "--dioxus-audio-radius": "1.25rem",
    },
    midnight: {
      "--dioxus-audio-base-100": "#091524",
      "--dioxus-audio-base-200": "#10243a",
      "--dioxus-audio-base-300": "#27425f",
      "--dioxus-audio-content": "#e6f4ff",
      "--dioxus-audio-primary": "#28c7d9",
      "--dioxus-audio-primary-content": "#06202a",
      "--dioxus-audio-radius": ".35rem",
    },
  } as const;
  const tokenNames = Object.keys(expectedThemes.citrus);
  const citrus = page.locator(".clip-editor.citrus");
  const midnight = page.locator(".clip-editor.midnight");

  for (const [editor, expectedTokens] of [
    [citrus, expectedThemes.citrus],
    [midnight, expectedThemes.midnight],
  ] as const) {
    for (const element of [
      editor,
      editor.getByRole("group", { name: "Select clip range" }),
      editor.getByRole("button", { name: "Play", exact: true }),
    ]) {
      const values = await element.evaluate((node, names) => {
        const styles = getComputedStyle(node);
        return Object.fromEntries(
          names.map((name) => [name, styles.getPropertyValue(name).trim()]),
        );
      }, tokenNames);
      expect(values).toEqual(expectedTokens);
    }
  }

  const pageValues = await page.locator("main").evaluate((node, names) => {
    const styles = getComputedStyle(node);
    return Object.fromEntries(
      names.map((name) => [name, styles.getPropertyValue(name).trim()]),
    );
  }, tokenNames);
  expect(pageValues).toEqual(
    Object.fromEntries(tokenNames.map((name) => [name, ""])),
  );

  await citrus.getByRole("slider", { name: "Selection start" }).fill("31");
  await expect(citrus.getByText("31.0% - 82.0%", { exact: true })).toBeVisible();
  await expect(midnight.getByText("18.0% - 82.0%", { exact: true })).toBeVisible();

  await citrus.getByRole("button", { name: "Play", exact: true }).click();
  await expect(citrus.getByRole("button", { name: "Pause", exact: true })).toBeVisible();
  await expect(midnight.getByRole("button", { name: "Play", exact: true })).toBeVisible();
  await citrus.getByRole("button", { name: "Pause", exact: true }).click();

  await midnight.getByRole("button", { name: "Play", exact: true }).click();
  await expect(midnight.getByRole("button", { name: "Pause", exact: true })).toBeVisible();
  await expect(citrus.getByRole("button", { name: "Play", exact: true })).toBeVisible();
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

test("scoped recipes are identical to independently fetched production sources", async ({
  openRoute,
  page,
}) => {
  await openRoute("/styles", "Make audio UI belong to your application");

  const scoped = page.getByRole("region", {
    name: "Citrus and Midnight: independently scoped clip editors",
  });
  const rustRecipe = scoped.getByRole("article").filter({
    hasText: "Rust composition",
  });
  const cssRecipe = scoped.getByRole("article").filter({
    hasText: "Scoped-theme stylesheet",
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

  const startMarker = "// region: scoped-recipe";
  const endMarker = "// endregion: scoped-recipe";
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

  const authoredThemes = {
    citrus: expectedThemeSource({
      "base-100": "#fff8e8",
      "base-200": "#f7e7c4",
      "base-300": "#d9b979",
      content: "#422716",
      primary: "#c4561f",
      "primary-content": "#fff8e8",
      radius: "1.25rem",
    }),
    midnight: expectedThemeSource({
      "base-100": "#091524",
      "base-200": "#10243a",
      "base-300": "#27425f",
      content: "#e6f4ff",
      primary: "#28c7d9",
      "primary-content": "#06202a",
      radius: "0.35rem",
    }),
  };
  for (const [theme, declarations] of Object.entries(authoredThemes)) {
    const ruleStart = sources.css.body.indexOf(`.${theme} {`);
    const ruleEnd = sources.css.body.indexOf("\n  }", ruleStart);
    expect(ruleStart).toBeGreaterThanOrEqual(0);
    expect(ruleEnd).toBeGreaterThan(ruleStart);
    const rule = sources.css.body.slice(ruleStart, ruleEnd);
    expect(rule.match(/--dioxus-audio-[\w-]+:/g)).toHaveLength(7);
    for (const declaration of declarations) {
      expect(rule).toContain(declaration);
    }
  }
});

function expectedThemeSource(tokens: Record<string, string>): string[] {
  return Object.entries(tokens).map(
    ([name, value]) => `--dioxus-audio-${name}: ${value};`,
  );
}

import { readFile } from "node:fs/promises";
import type { Locator, Page } from "@playwright/test";

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

test("daisyUI fallback chapter follows scoped examples with Playback and source recipe", async ({
  openRoute,
  page,
}) => {
  await openRoute("/styles", "Make audio UI belong to your application");

  const scoped = page.getByRole("region", {
    name: "Citrus and Midnight: independently scoped clip editors",
  });
  const fallback = page.getByRole("region", {
    name: "daisyUI: automatic host-theme fallback",
  });
  await expect(fallback).toBeVisible();
  expect(
    await scoped.evaluate(
      (scopedNode, fallbackNode) =>
        Boolean(
          scopedNode.compareDocumentPosition(fallbackNode) &
            Node.DOCUMENT_POSITION_FOLLOWING,
        ),
      await fallback.elementHandle(),
    ),
  ).toBe(true);

  await expect(fallback.getByText("What to notice", { exact: true })).toBeVisible();
  await expect(fallback.getByText("Live demonstration", { exact: true })).toBeVisible();
  await expect(fallback.getByRole("img", { name: "Host theme waveform" })).toBeVisible();
  await expect(fallback.getByRole("button", { name: "Play", exact: true })).toBeEnabled();
  await expect(fallback.getByText("Exact source recipe", { exact: true })).toBeVisible();
  await expect(fallback.locator("pre")).toHaveCount(1);
  await expect(fallback.getByText("Why it works", { exact: true })).toBeVisible();
});

test("guide keeps the complete progression and stable reference in order", async ({
  openRoute,
  page,
}) => {
  await openRoute("/styles", "Make audio UI belong to your application");

  const regions = await expectOrderedGuideContent(page);

  const reference = regions.reference;
  const expectedTokens = [
    "--dioxus-audio-base-100",
    "--dioxus-audio-base-200",
    "--dioxus-audio-base-300",
    "--dioxus-audio-content",
    "--dioxus-audio-primary",
    "--dioxus-audio-primary-content",
    "--dioxus-audio-warning",
    "--dioxus-audio-error",
    "--dioxus-audio-success",
    "--dioxus-audio-radius",
  ];
  const rows = reference.locator("tbody tr");
  await expect(rows).toHaveCount(expectedTokens.length);
  await expect(rows.locator("td:first-child")).toHaveText(expectedTokens);
  await expect(reference.getByText("Where the stable boundary ends")).toBeVisible();
  await expect(
    page.getByRole("navigation", { name: "Style guide prototype variants" }),
  ).toHaveCount(0);
});

test("every teaching control is keyboard reachable and package focus stays visible", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => localStorage.setItem("demo-theme", "light"));
  await openRoute("/styles", "Make audio UI belong to your application");

  const studio = page.locator(".studio-app");
  const citrus = page.locator(".clip-editor.citrus");
  const midnight = page.locator(".clip-editor.midnight");
  const fallback = page.getByRole("region", {
    name: "daisyUI: automatic host-theme fallback",
  });
  const controls = teachingControls(page);
  await page.getByRole("link", { name: "Analysis helpers" }).focus();
  await page.keyboard.press("Tab");
  for (const [index, { control, checkFocusRing }] of controls.entries()) {
    if (index > 0) await page.keyboard.press("Tab");
    await expect(control).toBeFocused();

    if (checkFocusRing) {
      await expectContrastingFocusRing(control, `control ${index + 1}`);
    }
  }

  const input = studio.getByRole("combobox", { name: "Recording input" });
  await expect(input.locator("option")).not.toHaveCount(1);
  await input.focus();
  await page.keyboard.press("KeyF");
  await expect(input).not.toHaveValue("");
  await expect(input.locator("option:checked")).toHaveText(/^Fake/);

  const previewStates = [
    ["Ready", "Microphone ready"],
    ["Recording", "Recording"],
    ["Muted", "Microphone muted by the device"],
    ["Denied", "Microphone access denied"],
  ] as const;
  for (const [buttonName, statusText] of previewStates) {
    const button = studio.getByRole("button", { name: buttonName });
    await button.focus();
    await page.keyboard.press("Enter");
    await expect(button).toHaveAttribute("aria-pressed", "true");
    await expect(
      studio.getByRole("status").filter({ hasText: statusText }),
    ).toHaveText(statusText);
  }

  for (const editor of [citrus, midnight]) {
    for (const [name, key] of [
      ["Selection start", "ArrowRight"],
      ["Selection end", "ArrowLeft"],
    ] as const) {
      const slider = editor.getByRole("slider", { name });
      const initialValue = await slider.inputValue();
      await slider.focus();
      await page.keyboard.press(key);
      await expect(slider).not.toHaveValue(initialValue);
    }
  }

  for (const example of [studio, citrus, midnight, fallback]) {
    await exercisePlaybackWithKeyboard(example, page);
  }

  await expectKeyboardOrder(
    page,
    studio.getByRole("button", { name: "Denied" }),
    playerControls(studio),
  );
  const scopedRecipes = sourceRecipeCards(guideRegions(page).scoped);
  await expectKeyboardOrder(
    page,
    scopedRecipes.nth(1).locator("pre"),
    playerControls(fallback),
  );

  const recipes = sourceRecipeCards(page.locator("main"));
  await expect(recipes).toHaveCount(5);
  for (let index = 0; index < (await recipes.count()); index += 1) {
    const recipe = recipes.nth(index);
    const code = recipe.locator("pre");
    await code.evaluate((node) => {
      node.scrollTop = 0;
    });
    await code.focus();
    await page.keyboard.press("PageDown");
    await expect
      .poll(() => code.evaluate((node) => node.scrollTop))
      .toBeGreaterThan(0);
  }

  const themeToggle = page.getByRole("button", { name: "Switch to dark theme" });
  await themeToggle.focus();
  await page.keyboard.press("Enter");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
});

test("styled examples keep their spacing and recipes are highlighted in place", async ({
  openRoute,
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openRoute("/styles", "Make audio UI belong to your application");

  for (const card of [
    page.locator(".studio-app"),
    page.locator(".clip-editor.citrus"),
    page.locator(".clip-editor.midnight"),
  ]) {
    await expect(card).toHaveCSS("padding-left", "20px");
  }

  const snippets = page.locator("pre[data-language]");
  await expect(snippets).toHaveCount(6);
  for (let index = 0; index < (await snippets.count()); index += 1) {
    const tokens = snippets.nth(index).locator('code span[class^="a-"]');
    expect(
      await tokens.count(),
      `snippet ${index + 1} has highlighted tokens`,
    ).toBeGreaterThan(0);
  }

  await expect(
    page.getByRole("link", { name: "View production source" }),
  ).toHaveCount(0);
});

for (const viewport of [
  { name: "desktop", width: 1440, height: 900 },
  { name: "narrow phone", width: 390, height: 844 },
] as const) {
  test(`guide content and controls stay contained at ${viewport.name} width`, async ({
    openRoute,
    page,
  }) => {
    await page.setViewportSize(viewport);
    await openRoute("/styles", "Make audio UI belong to your application");
    await expectGuideContainment(page, viewport.width);
  });
}

test("daisyUI fallback follows both host themes without changing explicit themes", async ({
  openRoute,
  page,
}) => {
  await page.addInitScript(() => localStorage.setItem("demo-theme", "light"));
  await openRoute("/styles", "Make audio UI belong to your application");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");

  const fallback = page.getByRole("region", {
    name: "daisyUI: automatic host-theme fallback",
  });
  const waveform = fallback.getByRole("img", { name: "Host theme waveform" });
  const skipBack = fallback.getByRole("button", { name: "Skip back 15 seconds" });
  const play = fallback.getByRole("button", { name: "Play", exact: true });
  const rate = fallback.getByRole("button", { name: "Playback speed: 1x" });
  const seek = fallback.getByRole("slider", { name: "Seek" });
  const publicTokens = [
    "--dioxus-audio-base-200",
    "--dioxus-audio-base-300",
    "--dioxus-audio-content",
    "--dioxus-audio-primary",
    "--dioxus-audio-primary-content",
  ];

  for (const element of [waveform, skipBack, play, rate, seek]) {
    const values = await readCustomProperties(element, publicTokens);
    expect(values).toEqual(
      Object.fromEntries(publicTokens.map((name) => [name, ""])),
    );
  }

  const explicitThemeTokens = {
    studio: [
      "--dioxus-audio-base-100",
      "--dioxus-audio-base-200",
      "--dioxus-audio-base-300",
      "--dioxus-audio-content",
      "--dioxus-audio-primary",
      "--dioxus-audio-primary-content",
      "--dioxus-audio-warning",
      "--dioxus-audio-error",
      "--dioxus-audio-success",
      "--dioxus-audio-radius",
    ],
    scoped: [
      "--dioxus-audio-base-100",
      "--dioxus-audio-base-200",
      "--dioxus-audio-base-300",
      "--dioxus-audio-content",
      "--dioxus-audio-primary",
      "--dioxus-audio-primary-content",
      "--dioxus-audio-radius",
    ],
  };
  const explicitThemes = [
    [page.locator(".studio-app"), explicitThemeTokens.studio],
    [page.locator(".clip-editor.citrus"), explicitThemeTokens.scoped],
    [page.locator(".clip-editor.midnight"), explicitThemeTokens.scoped],
  ] as const;
  const snapshotExplicitThemes = () =>
    Promise.all(
      explicitThemes.map(([element, names]) =>
        readCustomProperties(element, names),
      ),
    );
  const explicitValuesBefore = await snapshotExplicitThemes();

  const renderedFallbackStyles = () =>
    Promise.all([
      waveform.evaluate((node) => getComputedStyle(node).color),
      skipBack.evaluate((node) => getComputedStyle(node).color),
      rate.evaluate((node) => getComputedStyle(node).backgroundColor),
      renderedScrubberSurfaceColor(seek),
      play.evaluate((node) => getComputedStyle(node).backgroundColor),
      play.evaluate((node) => getComputedStyle(node).color),
    ]);
  const renderedHostThemeStyles = () =>
    page.locator("html").evaluate((root) => {
      const declarations = [
        ["color", "--color-primary"],
        ["color", "--color-base-content"],
        ["background-color", "--color-base-200"],
        ["background-color", "--color-base-300"],
        ["background-color", "--color-primary"],
        ["color", "--color-primary-content"],
      ] as const;

      return declarations.map(([property, variable]) => {
        const probe = document.createElement("span");
        probe.style.setProperty(property, `var(${variable})`);
        root.append(probe);
        const value = getComputedStyle(probe).getPropertyValue(property);
        probe.remove();
        return value;
      });
    });

  const lightFallbackStyles = await renderedFallbackStyles();
  expect(lightFallbackStyles).toEqual(await renderedHostThemeStyles());

  await page.getByRole("button", { name: "Switch to dark theme" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  const darkFallbackStyles = await renderedFallbackStyles();
  expect(darkFallbackStyles).toEqual(await renderedHostThemeStyles());
  expect(darkFallbackStyles).not.toEqual(lightFallbackStyles);
  expect(await snapshotExplicitThemes()).toEqual(explicitValuesBefore);

  await play.click();
  const pause = fallback.getByRole("button", { name: "Pause", exact: true });
  await expect(pause).toBeVisible();
  await pause.click();
  await expect(
    fallback.getByRole("button", { name: "Play", exact: true }),
  ).toBeVisible();
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
    const values = await readCustomProperties(element, tokenNames);
    expect(values).toEqual(expectedTokens);
  }

  const shellValues = await readCustomProperties(page.locator("main"), tokenNames);
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
      const values = await readCustomProperties(element, tokenNames);
      expect(values).toEqual(expectedTokens);
    }
  }

  const pageValues = await readCustomProperties(page.locator("main"), tokenNames);
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

test("Studio recipes are identical to production sources", async ({
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

  const [rustSource, cssSource, renderedRust, renderedCss] = await Promise.all([
    readFile("src/examples/styles/studio.rs", "utf8"),
    readFile("src/examples/styles/studio.css", "utf8"),
    rustRecipe.locator('pre[data-language="rust"] code').textContent(),
    cssRecipe.locator('pre[data-language="css"] code').textContent(),
  ]);

  const extractedRust = extractRecipeRegion(rustSource, "studio-recipe");
  expect(renderedRust).toBe(extractedRust);

  expect(renderedCss).toBe(cssSource.trimEnd());
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
    expect(cssSource).toContain(`${name}: ${value};`);
  }

  const studioRuleStart = cssSource.indexOf(".studio-app {");
  const studioRuleEnd = cssSource.indexOf("\n  }", studioRuleStart);
  expect(studioRuleStart).toBeGreaterThanOrEqual(0);
  expect(studioRuleEnd).toBeGreaterThan(studioRuleStart);
  const studioRule = cssSource.slice(studioRuleStart, studioRuleEnd);
  expect(studioRule.match(/--dioxus-audio-[\w-]+:/g)).toHaveLength(10);
  const outsideStudioRule =
    cssSource.slice(0, studioRuleStart) + cssSource.slice(studioRuleEnd);
  expect(outsideStudioRule).not.toMatch(/--dioxus-audio-[\w-]+:/);
});

test("scoped recipes are identical to production sources", async ({
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
  const [rustSource, cssSource, renderedRust, renderedCss] = await Promise.all([
    readFile("src/examples/styles/scoped.rs", "utf8"),
    readFile("src/examples/styles/scoped.css", "utf8"),
    rustRecipe.locator('pre[data-language="rust"] code').textContent(),
    cssRecipe.locator('pre[data-language="css"] code').textContent(),
  ]);

  const extractedRust = extractRecipeRegion(rustSource, "scoped-recipe");
  expect(renderedRust).toBe(extractedRust);
  expect(renderedCss).toBe(cssSource.trimEnd());

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
    const ruleStart = cssSource.indexOf(`.${theme} {`);
    const ruleEnd = cssSource.indexOf("\n  }", ruleStart);
    expect(ruleStart).toBeGreaterThanOrEqual(0);
    expect(ruleEnd).toBeGreaterThan(ruleStart);
    const rule = cssSource.slice(ruleStart, ruleEnd);
    expect(rule.match(/--dioxus-audio-[\w-]+:/g)).toHaveLength(7);
    for (const declaration of declarations) {
      expect(rule).toContain(declaration);
    }
  }
});

test("daisyUI recipe is identical to its token-free production source", async ({
  openRoute,
  page,
}) => {
  await openRoute("/styles", "Make audio UI belong to your application");

  const fallback = page.getByRole("region", {
    name: "daisyUI: automatic host-theme fallback",
  });
  const rustRecipe = fallback.getByRole("article").filter({
    hasText: "Rust composition",
  });
  const [rustSource, renderedRust] = await Promise.all([
    readFile("src/examples/styles/daisy.rs", "utf8"),
    rustRecipe.locator('pre[data-language="rust"] code').textContent(),
  ]);

  const extractedRust = extractRecipeRegion(rustSource, "daisy-recipe");
  expect(renderedRust).toBe(extractedRust);
  expect(rustSource).not.toMatch(/--dioxus-audio-[\w-]+\s*:/);
  expect(renderedRust).not.toMatch(/--dioxus-audio-[\w-]+\s*:/);
});

function readCustomProperties(element: Locator, names: readonly string[]) {
  return element.evaluate((node, tokenNames) => {
    const styles = getComputedStyle(node);
    return Object.fromEntries(
      tokenNames.map((name) => [name, styles.getPropertyValue(name).trim()]),
    );
  }, names);
}

async function renderedScrubberSurfaceColor(seek: Locator) {
  await seek.scrollIntoViewIfNeeded();
  return seek.evaluate((node) => {
    const seekBounds = node.getBoundingClientRect();
    const centerX = seekBounds.left + seekBounds.width / 2;
    const centerY = seekBounds.top + seekBounds.height / 2;
    const transparent = "rgba(0, 0, 0, 0)";
    const surface = document.elementsFromPoint(centerX, centerY).find((candidate) => {
      if (candidate === node) return false;

      const bounds = candidate.getBoundingClientRect();
      const background = getComputedStyle(candidate).backgroundColor;
      return (
        bounds.width >= seekBounds.width * 0.9 &&
        bounds.height > 0 &&
        bounds.height < seekBounds.height &&
        background !== transparent
      );
    });

    if (!surface) throw new Error("Seek control has no visible track surface");
    return getComputedStyle(surface).backgroundColor;
  });
}

function extractRecipeRegion(source: string, name: string): string {
  const startMarker = `// region: ${name}`;
  const endMarker = `// endregion: ${name}`;
  expect(source.split(startMarker)).toHaveLength(2);
  expect(source.split(endMarker)).toHaveLength(2);

  const start = source.indexOf(startMarker) + startMarker.length;
  const end = source.indexOf(endMarker, start);
  expect(end).toBeGreaterThan(start);
  const region = source.slice(start, end).replace(/^\n/, "").replace(/\n$/, "");
  expect(region.trim()).not.toBe("");
  return region;
}

function expectedThemeSource(tokens: Record<string, string>): string[] {
  return Object.entries(tokens).map(
    ([name, value]) => `--dioxus-audio-${name}: ${value};`,
  );
}

type TeachingControl = {
  control: Locator;
  checkFocusRing: boolean;
};

function guideRegions(page: Page) {
  return {
    setup: page.getByRole("region", { name: "Stylesheet setup" }),
    cascade: page.getByRole("region", { name: "How the cascade resolves" }),
    studio: page.getByRole("region", {
      name: "Studio: one complete app-wide theme",
    }),
    scoped: page.getByRole("region", {
      name: "Citrus and Midnight: independently scoped clip editors",
    }),
    fallback: page.getByRole("region", {
      name: "daisyUI: automatic host-theme fallback",
    }),
    reference: page.getByRole("region", { name: "Stable styling contract" }),
    responsibility: page.getByRole("complementary", {
      name: "Application-author responsibility",
    }),
  };
}

function teachingControls(page: Page): TeachingControl[] {
  const regions = guideRegions(page);
  const studio = page.locator(".studio-app");
  const citrus = regions.scoped.locator(".clip-editor.citrus");
  const midnight = regions.scoped.locator(".clip-editor.midnight");

  return [
    packageControl(studio.getByRole("combobox", { name: "Recording input" })),
    teachingControl(studio.getByRole("button", { name: "Ready" })),
    teachingControl(studio.getByRole("button", { name: "Recording" })),
    teachingControl(studio.getByRole("button", { name: "Muted" })),
    teachingControl(studio.getByRole("button", { name: "Denied" })),
    packageControl(studio.getByRole("button", { name: "Play", exact: true })),
    packageControl(studio.getByRole("button", { name: "Playback speed: 1x" })),
    ...sourceRecipeControls(regions.studio, 2),
    ...clipEditorControls(citrus),
    ...clipEditorControls(midnight),
    ...sourceRecipeControls(regions.scoped, 2),
    packageControl(
      regions.fallback.getByRole("button", { name: "Play", exact: true }),
    ),
    packageControl(
      regions.fallback.getByRole("button", { name: "Playback speed: 1x" }),
    ),
    ...sourceRecipeControls(regions.fallback, 1),
  ];
}

function sourceRecipeControls(
  chapter: Locator,
  count: number,
): TeachingControl[] {
  const controls: TeachingControl[] = [];
  const recipes = sourceRecipeCards(chapter);
  for (let index = 0; index < count; index += 1) {
    const recipe = recipes.nth(index);
    controls.push(teachingControl(recipe.locator("pre")));
  }
  return controls;
}

function sourceRecipeCards(container: Locator) {
  return container.locator("article:has(pre[data-language])");
}

function teachingControl(control: Locator): TeachingControl {
  return { control, checkFocusRing: false };
}

function packageControl(control: Locator): TeachingControl {
  return { control, checkFocusRing: true };
}

function clipEditorControls(editor: Locator): TeachingControl[] {
  return [
    packageControl(editor.getByRole("slider", { name: "Selection start" })),
    packageControl(editor.getByRole("slider", { name: "Selection end" })),
    ...playerControls(editor).map(packageControl),
  ];
}

function playerControls(example: Locator): Locator[] {
  return [
    example.getByRole("slider", { name: "Seek" }),
    example.getByRole("button", { name: "Skip back 15 seconds" }),
    example.getByRole("button", { name: "Play", exact: true }),
    example.getByRole("button", { name: "Skip forward 15 seconds" }),
    example.getByRole("button", { name: /^Playback speed:/ }),
  ];
}

async function visibleFocusRing(control: Locator) {
  return control.evaluate((node) => {
    const element = node as HTMLElement;
    const candidates: Element[] = [element];
    if (element.parentElement) {
      candidates.push(...element.parentElement.children);
    }
    let scope = element.parentElement;
    while (scope && scope.tagName !== "MAIN") {
      candidates.push(scope);
      scope = scope.parentElement;
    }
    const outlined = [...new Set(candidates)].filter((candidate) => {
      const styles = getComputedStyle(candidate);
      const outline = canvasColor(styles.outlineColor);
      return (
        styles.opacity !== "0" &&
        styles.visibility !== "hidden" &&
        candidate.getBoundingClientRect().width > 0 &&
        candidate.getBoundingClientRect().height > 0 &&
        styles.outlineStyle !== "none" &&
        parseFloat(styles.outlineWidth) > 0 &&
        outline[3] > 0
      );
    });
    if (outlined.length === 0) {
      return { width: 0, style: "none", color: "", background: "", contrast: 0 };
    }

    return outlined
      .map((target) => {
        const targetStyles = getComputedStyle(target);
        let ancestor = target.parentElement;
        let background = "rgb(255, 255, 255)";
        while (ancestor) {
          const candidate = getComputedStyle(ancestor).backgroundColor;
          const channels = canvasColor(candidate);
          if (channels[3] > 0) {
            background = candidate;
            break;
          }
          ancestor = ancestor.parentElement;
        }

        return {
          width: parseFloat(targetStyles.outlineWidth),
          style: targetStyles.outlineStyle,
          color: targetStyles.outlineColor,
          background,
          contrast: contrastRatio(
            canvasColor(targetStyles.outlineColor),
            canvasColor(background),
          ),
        };
      })
      .reduce((strongest, ring) =>
        ring.contrast > strongest.contrast ? ring : strongest,
      );

    function canvasColor(color: string): Uint8ClampedArray {
      const canvas = document.createElement("canvas");
      canvas.width = 1;
      canvas.height = 1;
      const context = canvas.getContext("2d", { willReadFrequently: true });
      if (!context) throw new Error("Canvas color conversion is unavailable");
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = color;
      context.fillRect(0, 0, 1, 1);
      return context.getImageData(0, 0, 1, 1).data;
    }

    function contrastRatio(a: Uint8ClampedArray, b: Uint8ClampedArray): number {
      const luminance = (channels: Uint8ClampedArray) => {
        const linear = Array.from(channels.slice(0, 3), (value) => {
          const channel = value / 255;
          return channel <= 0.04045
            ? channel / 12.92
            : ((channel + 0.055) / 1.055) ** 2.4;
        });
        return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
      };
      const lighter = Math.max(luminance(a), luminance(b));
      const darker = Math.min(luminance(a), luminance(b));
      return (lighter + 0.05) / (darker + 0.05);
    }
  });
}

async function exercisePlaybackWithKeyboard(example: Locator, page: Page) {
  const play = example.getByRole("button", { name: "Play", exact: true });
  await play.focus();
  await page.keyboard.press("Enter");
  const pause = example.getByRole("button", { name: "Pause", exact: true });
  await expect(pause).toBeVisible();
  await pause.focus();
  await page.keyboard.press("Enter");
  await expect(example.getByRole("button", { name: "Play", exact: true })).toBeVisible();

  const seek = example.getByRole("slider", { name: "Seek" });
  await expect(seek).toBeEnabled();
  const initialPosition = await seek.inputValue();
  await seek.focus();
  await page.keyboard.press("ArrowRight");
  await expect(seek).not.toHaveValue(initialPosition);

  const skipForward = example.getByRole("button", {
    name: "Skip forward 15 seconds",
  });
  await skipForward.focus();
  await page.keyboard.press("Enter");
  await expect
    .poll(() => seek.inputValue().then(Number))
    .toBeGreaterThan(Number(initialPosition));

  const skipBack = example.getByRole("button", {
    name: "Skip back 15 seconds",
  });
  await skipBack.focus();
  await page.keyboard.press("Enter");
  await expect.poll(() => seek.inputValue().then(Number)).toBe(0);

  const rate = example.getByRole("button", { name: "Playback speed: 1x" });
  await rate.focus();
  await page.keyboard.press("Enter");
  await expect(
    example.getByRole("button", { name: "Playback speed: 1.5x" }),
  ).toBeVisible();
}

async function expectKeyboardOrder(
  page: Page,
  previous: Locator,
  controls: Locator[],
) {
  await previous.focus();
  for (const [index, control] of controls.entries()) {
    await page.keyboard.press("Tab");
    await expect(control).toBeFocused();
    await expectContrastingFocusRing(control, `dynamic control ${index + 1}`);
  }
}

async function expectGuideContainment(page: Page, viewportWidth: number) {
  const regions = await expectOrderedGuideContent(page);
  const orderedRegions = Object.values(regions);
  for (let index = 1; index < orderedRegions.length; index += 1) {
    const previous = await requiredBounds(orderedRegions[index - 1]);
    const current = await requiredBounds(orderedRegions[index]);
    expect(previous.y + previous.height).toBeLessThanOrEqual(current.y + 1);
  }

  const documentWidth = await page.evaluate(() => ({
    client: document.documentElement.clientWidth,
    scroll: document.documentElement.scrollWidth,
  }));
  expect(documentWidth.client).toBe(viewportWidth);
  expect(documentWidth.scroll).toBeLessThanOrEqual(documentWidth.client);

  const recipeCode = page.locator("pre[data-language] code");
  await expect(recipeCode).toHaveCount(6);
  for (let index = 0; index < (await recipeCode.count()); index += 1) {
    await expect(recipeCode.nth(index)).toBeVisible();
  }

  const examples = [
    page.locator(".studio-app"),
    page.locator(".clip-editor.citrus"),
    page.locator(".clip-editor.midnight"),
    regions.fallback.locator("article").first(),
  ];
  for (const example of examples) {
    const exampleBounds = await requiredBounds(example);
    const controls = example.locator("button, select, input");
    for (let index = 0; index < (await controls.count()); index += 1) {
      expectBoundsWithin(await requiredBounds(controls.nth(index)), exampleBounds);
    }
  }

  const mainBounds = await requiredBounds(page.locator("main"));
  const codeBlocks = page.locator("pre");
  let longCodeBlocks = 0;
  for (let index = 0; index < (await codeBlocks.count()); index += 1) {
    const block = codeBlocks.nth(index);
    const bounds = await requiredBounds(block);
    expectBoundsWithin(bounds, mainBounds);
    const containerBounds = await block.evaluate((node) => {
      const container = node.closest("article") ?? node.closest("section");
      if (!container) throw new Error("Code block has no guide container");
      const bounds = container.getBoundingClientRect();
      return {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
      };
    });
    expectBoundsWithin(bounds, containerBounds);
    const overlaps = await block.evaluate((node) => {
      const main = node.closest("main");
      if (!main) throw new Error("Code block is outside the guide");
      const blockBounds = node.getBoundingClientRect();

      return [...main.querySelectorAll("*")]
        .filter(
          (candidate) =>
            candidate !== node &&
            !candidate.contains(node) &&
            !node.contains(candidate) &&
            (candidate.tagName === "PRE" || !candidate.closest("pre")),
        )
        .filter((candidate) => {
          const candidateBounds = candidate.getBoundingClientRect();
          return (
            candidateBounds.width > 0 &&
            candidateBounds.height > 0 &&
            blockBounds.left < candidateBounds.right &&
            blockBounds.right > candidateBounds.left &&
            blockBounds.top < candidateBounds.bottom &&
            blockBounds.bottom > candidateBounds.top
          );
        })
        .map((candidate) => candidate.tagName.toLowerCase());
    });
    expect(overlaps, `code block ${index + 1} overlaps guide content`).toEqual([]);
    const overflow = await block.evaluate((node) => ({
      client: node.clientWidth,
      scroll: node.scrollWidth,
      overflowX: getComputedStyle(node).overflowX,
    }));
    expect(overflow.overflowX).toMatch(/auto|scroll/);
    if (overflow.scroll > overflow.client) longCodeBlocks += 1;
  }
  expect(longCodeBlocks).toBeGreaterThan(0);

  const reference = regions.reference;
  const table = reference.getByRole("table");
  const tableScroller = table.locator("..");
  await expect(reference.getByRole("row")).toHaveCount(11);
  await expect(tableScroller).toHaveCSS("overflow-x", "auto");
  if (viewportWidth === 390) {
    const widths = await tableScroller.evaluate((node) => ({
      client: node.clientWidth,
      scroll: node.scrollWidth,
    }));
    expect(widths.scroll).toBeGreaterThan(widths.client);
    await tableScroller.evaluate((node) => {
      node.scrollLeft = node.scrollWidth;
    });
    const scrollerBounds = await requiredBounds(tableScroller);
    const lastHeaderBounds = await requiredBounds(
      reference.getByRole("columnheader", { name: "Standalone default" }),
    );
    expectBoundsWithin(lastHeaderBounds, scrollerBounds);
  }
}

async function expectOrderedGuideContent(page: Page) {
  const regions = guideRegions(page);
  const orderedContent = Object.values(regions);
  for (const content of orderedContent) await expect(content).toBeVisible();
  await expectInDocumentOrder(orderedContent);
  return regions;
}

async function expectContrastingFocusRing(control: Locator, label: string) {
  const ring = await visibleFocusRing(control);
  expect(ring.width, `focus ring for ${label}`).toBeGreaterThan(0);
  expect(ring.style, `focus ring for ${label}`).not.toBe("none");
  expect(
    ring.contrast,
    `focus ring ${ring.color} against ${ring.background} for ${label}`,
  ).toBeGreaterThanOrEqual(3);
}

type Bounds = {
  x: number;
  y: number;
  width: number;
  height: number;
};

async function requiredBounds(locator: Locator): Promise<Bounds> {
  const bounds = await locator.boundingBox();
  expect(bounds).not.toBeNull();
  return bounds!;
}

function expectBoundsWithin(inner: Bounds, outer: Bounds) {
  expect(inner.x).toBeGreaterThanOrEqual(outer.x - 1);
  expect(inner.y).toBeGreaterThanOrEqual(outer.y - 1);
  expect(inner.x + inner.width).toBeLessThanOrEqual(outer.x + outer.width + 1);
  expect(inner.y + inner.height).toBeLessThanOrEqual(outer.y + outer.height + 1);
}

async function expectInDocumentOrder(elements: Locator[]) {
  for (let index = 1; index < elements.length; index += 1) {
    const previous = elements[index - 1];
    const current = elements[index];
    expect(
      await previous.evaluate(
        (previousNode, currentNode) =>
          Boolean(
            previousNode.compareDocumentPosition(currentNode) &
              Node.DOCUMENT_POSITION_FOLLOWING,
          ),
        await current.elementHandle(),
      ),
    ).toBe(true);
  }
}

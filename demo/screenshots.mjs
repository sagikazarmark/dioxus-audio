import { rm, mkdir } from "node:fs/promises";
import path from "node:path";

import { chromium } from "@playwright/test";

const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:8080";
const outputRoot = "build/screenshots";

const resolutions = [
  { width: 390, height: 844 },
  { width: 768, height: 1024 },
  { width: 1440, height: 900 },
];

await rm(outputRoot, { force: true, recursive: true });

const browser = await chromium.launch({
  args: [
    "--use-fake-device-for-media-stream",
    "--use-fake-ui-for-media-stream",
    "--autoplay-policy=no-user-gesture-required",
    `--unsafely-treat-insecure-origin-as-secure=${new URL(baseURL).origin}`,
  ],
});

async function newDemoContext(viewport) {
  const context = await browser.newContext({
    baseURL,
    colorScheme: "light",
    permissions: ["microphone"],
    reducedMotion: "reduce",
    viewport,
  });
  await context.addInitScript(() => {
    window.localStorage.setItem("demo-theme", "light");
  });

  return context;
}

async function loadPage(page, routePath) {
  const response = await page.goto(routePath, {
    waitUntil: "domcontentloaded",
  });
  if (!response?.ok()) {
    throw new Error(
      `Failed to load ${routePath}: ${response?.status() ?? "no response"}`,
    );
  }

  await page.locator("h1").waitFor({ state: "visible" });
  await page.evaluate(() => document.fonts.ready);
}

async function discoverPages() {
  const context = await newDemoContext(resolutions.at(-1));
  const page = await context.newPage();
  await loadPage(page, "/");

  // The application navigation is the source of truth for exposed pages.
  const linkedPaths = await page.locator("a[href]").evaluateAll(
    (links, origin) =>
      links
        .map((link) => new URL(link.href))
        .filter((url) => url.origin === origin)
        .map((url) => url.pathname),
    new URL(baseURL).origin,
  );
  await context.close();

  const routePaths = [
    ...new Set(["/", ...linkedPaths, "/not-a-documented-route"]),
  ];
  return routePaths.map((routePath) => ({
    name:
      routePath === "/"
        ? "overview"
        : routePath.slice(1).replaceAll("/", "-"),
    path: routePath,
  }));
}

try {
  const pages = await discoverPages();

  for (const resolution of resolutions) {
    const resolutionName = `${resolution.width}x${resolution.height}`;
    const outputDirectory = path.join(outputRoot, resolutionName);
    await mkdir(outputDirectory, { recursive: true });

    const context = await newDemoContext(resolution);
    const page = await context.newPage();
    for (const demoPage of pages) {
      await loadPage(page, demoPage.path);
      await page.screenshot({
        animations: "disabled",
        fullPage: true,
        path: path.join(outputDirectory, `${demoPage.name}.png`),
      });
    }

    await context.close();
  }
} finally {
  await browser.close();
}

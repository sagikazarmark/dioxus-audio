import { expect, test } from "./fixtures";

const documentedRoutes = [
  {
    path: "/recorder",
    heading: "Capture, inspect, and replay",
    section: "use_audio_recorder",
    example: "Capture status",
    source: "pub fn RecorderExample",
    tabbed: true,
  },
  {
    path: "/playback",
    heading: "Load audio only when it is needed",
    section: "AudioPlayer",
    example: "Audio loads on first play",
    source: "pub fn PlaybackExample",
    tabbed: false,
  },
  {
    path: "/playback-source",
    heading: "Load local and remote media by URL",
    section: "URL Playback Source",
    example: "Application-owned Playback Source",
    source: "pub fn UrlPlaybackExample",
    tabbed: false,
  },
  {
    path: "/devices",
    heading: "Discover and select microphones",
    section: "use_audio_input_devices",
    example: "Selected: system default",
    source: "pub fn DevicesExample",
    tabbed: false,
  },
  {
    path: "/visualizers",
    heading: "Render live audio analysis",
    section: "Live visualizers",
    example: "Show processing state",
    source: "pub fn VisualizersExample",
    tabbed: false,
  },
  {
    path: "/decoding",
    heading: "Decode complete audio into planar samples",
    section: "Complete-file decoding",
    example: "No decode requested",
    source: "pub fn DecodingExample",
    tabbed: true,
  },
  {
    path: "/waveforms",
    heading: "Preview and select waveform ranges",
    section: "Waveform components",
    example: "Compact preview",
    source: "pub fn WaveformsExample",
    tabbed: true,
  },
  {
    path: "/analysis",
    heading: "Process audio data without a browser",
    section: "Analysis helpers",
    example: "Source peaks",
    source: "pub fn AnalysisExample",
    tabbed: false,
  },
] as const;

test("overview loads directly from the release bundle", async ({
  openRoute,
  page,
}) => {
  await openRoute("/", "Browser audio building blocks for Dioxus");

  await expect(
    page.getByRole("heading", { name: "Quick start" }),
  ).toBeVisible();
  await expect(page.locator("main pre")).toContainText("use_audio_recorder");
});

for (const route of documentedRoutes) {
  test(`${route.path} loads its example and source directly`, async ({
    openRoute,
    page,
  }) => {
    await openRoute(route.path, route.heading);

    await expect(
      page.getByRole("heading", { level: 2, name: route.section }),
    ).toBeVisible();
    await expect(page.getByText(route.example, { exact: true })).toBeVisible();

    if (route.tabbed) {
      await page.getByRole("button", { name: "Source" }).click();
      await expect(
        page.getByRole("region", { name: "Example source" }),
      ).toBeVisible();
    }

    await expect(
      page.locator("main pre").filter({ hasText: route.source }),
    ).toBeVisible();
  });
}

test("static SPA fallback serves unknown routes and assets", async ({
  openRoute,
  page,
}) => {
  await openRoute("/not-a-documented-route", "Page not found");
  await expect(
    page.getByText("The demo has no page at /not-a-documented-route."),
  ).toBeVisible();

  const missingAsset = await page.evaluate(async () => {
    const response = await fetch("/assets/not-a-real-file.js");
    return {
      body: await response.text(),
      status: response.status,
    };
  });
  expect(missingAsset.status).toBe(200);
  expect(missingAsset.body).toContain("<title>dioxus-audio demo</title>");
});

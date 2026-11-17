// Run with playwright-cli run-code --filename scripts/check-jira-filter-layout.js
// against the browser preview. jsdom cannot measure this overflow regression.
async (page) => {
  await page.getByRole("button", { name: "Jira", exact: true }).click();
  const filters = page.getByRole("group", { name: "Ticket filters", exact: true });
  await filters.waitFor();

  async function checkVerticalFit(label) {
    const dimensions = await filters.evaluate((element) => ({
      available: element.clientHeight,
      content: element.scrollHeight,
    }));
    if (dimensions.content > dimensions.available) {
      throw new Error(`${label}: filter bar overflows vertically: ${JSON.stringify(dimensions)}`);
    }
  }

  for (const width of [1440, 900, 600]) {
    await page.setViewportSize({ width, height: 800 });
    await checkVerticalFit(`${width}px`);
  }

  // Stress the real tab styles without changing saved settings or contacting Jira.
  await filters.evaluate((element) => {
    const template = element.querySelector("button");
    for (let index = 0; index < 8; index++) {
      const tab = template.cloneNode(true);
      tab.textContent = `Saved filter with a longer title ${index + 1}`;
      tab.dataset.layoutProbe = "true";
      element.append(tab);
    }
  });
  try {
    await checkVerticalFit("Overflowing filters");
    const reachable = await filters.evaluate((element) => {
      const last = element.lastElementChild;
      last.focus();
      last.scrollIntoView({ block: "nearest", inline: "nearest" });
      const bar = element.getBoundingClientRect();
      const tab = last.getBoundingClientRect();
      return element.scrollWidth > element.clientWidth && element.scrollLeft > 0 &&
        tab.right <= bar.right + 1 && document.activeElement === last;
    });
    if (!reachable) {
      throw new Error("Overflowing filters must remain reachable with keyboard focus and horizontal scrolling");
    }
  } finally {
    await filters.evaluate((element) => {
      element.querySelectorAll("[data-layout-probe]").forEach((tab) => tab.remove());
      element.scrollLeft = 0;
    });
  }
  return "Filter bar fits vertically at 1440, 900 and 600px; overflowed tabs remain reachable";
}

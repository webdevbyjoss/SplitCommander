import { test, expect } from "@playwright/test";
import { injectTauriMocks } from "./tauri-mock";

test.beforeEach(async ({ page }) => {
  await injectTauriMocks(page);
  await page.goto("/");
  // Wait for directory entries to load (mock invoke completes near-instantly)
  await expect(page.locator('[data-testid="pane-left"] .row')).not.toHaveCount(0);
});

test.describe("App startup", () => {
  test("renders folder picker buttons in breadcrumb bars", async ({ page }) => {
    const leftPicker = page.locator('[data-testid="pane-left"] .picker-btn');
    const rightPicker = page.locator('[data-testid="pane-right"] .picker-btn');
    await expect(leftPicker).toBeVisible();
    await expect(rightPicker).toBeVisible();
  });

  test("renders two browse panes", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');
    const rightPane = page.locator('[data-testid="pane-right"]');

    await expect(leftPane).toBeVisible();
    await expect(rightPane).toBeVisible();
  });

  test("shows directory contents in both panes", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');
    const rightPane = page.locator('[data-testid="pane-right"]');

    await expect(leftPane.locator("text=Documents")).toBeVisible();
    await expect(leftPane.locator("text=Desktop")).toBeVisible();
    await expect(rightPane.locator("text=Documents")).toBeVisible();
    await expect(rightPane.locator("text=Desktop")).toBeVisible();
  });

  test("shows item count in breadcrumb bar", async ({ page }) => {
    const leftCrumb = page.locator('[data-testid="breadcrumb-left"]');
    await expect(leftCrumb.locator("text=5 items")).toBeVisible();
  });

  test("shows breadcrumb navigation", async ({ page }) => {
    const leftCrumb = page.locator('[data-testid="breadcrumb-left"]');
    await expect(leftCrumb).toBeVisible();
    await expect(leftCrumb.locator("text=testuser")).toBeVisible();
  });

  test("renders bottom bar with shortcuts", async ({ page }) => {
    const bottomBar = page.locator("footer");
    await expect(bottomBar).toBeVisible();
    await expect(bottomBar.locator("text=compare")).toBeVisible();
  });

  test("shows key hints in footer", async ({ page }) => {
    const footer = page.locator("footer");
    await expect(footer.locator("text=compare")).toBeVisible();
    await expect(footer.locator("text=move")).toBeVisible();
  });
});

test.describe("Directory navigation", () => {
  test("double-click opens a directory", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');

    await leftPane.locator('[data-testid="row-Documents"]').dblclick();

    // Wait for new contents to appear
    await expect(leftPane.locator("text=report.pdf")).toBeVisible();
    await expect(leftPane.locator("text=photos")).toBeVisible();

    // Breadcrumb should update
    const crumb = page.locator('[data-testid="breadcrumb-left"]');
    await expect(crumb.locator("text=Documents")).toBeVisible();
  });

  test("double-click '..' navigates up", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');

    // Navigate into Documents first
    await leftPane.locator('[data-testid="row-Documents"]').dblclick();
    await expect(leftPane.locator("text=report.pdf")).toBeVisible();

    // Click ".." to go back
    await leftPane.locator('[data-testid="row-parent"]').dblclick();

    // Wait for home dir contents
    await expect(leftPane.locator("text=Documents")).toBeVisible();
    await expect(leftPane.locator("text=Desktop")).toBeVisible();
  });

  test("can navigate deep into directory tree", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');

    // Navigate: home -> Projects
    await leftPane.locator('[data-testid="row-Projects"]').dblclick();
    await expect(leftPane.locator("text=my-app")).toBeVisible();

    // Navigate: Projects -> my-app
    await leftPane.locator('[data-testid="row-my-app"]').dblclick();
    await expect(leftPane.locator("text=package.json")).toBeVisible();
    await expect(leftPane.locator("text=index.html")).toBeVisible();
    await expect(leftPane.locator("text=src")).toBeVisible();
  });

  test("clicking column header sorts entries", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');

    // Navigate into Documents (has report.pdf + photos dir)
    await leftPane.locator('[data-testid="row-Documents"]').dblclick();
    await expect(leftPane.locator("text=report.pdf")).toBeVisible();

    // Click Type header to sort by type
    await leftPane.locator("button.col-header.col-type").click();

    // Type header should now be active
    await expect(leftPane.locator("button.col-header.col-type.active")).toBeVisible();

    // Click Size header to sort by size
    await leftPane.locator("button.col-header.col-size").click();
    await expect(leftPane.locator("button.col-header.col-size.active")).toBeVisible();

    // Click Size again to reverse sort direction
    await leftPane.locator("button.col-header.col-size").click();
    await expect(leftPane.locator("button.col-header.col-size.active")).toBeVisible();
  });

  test("shows file type/extension column", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');

    // Home dir has notes.txt (type "txt"); .zshrc hidden by default
    // Check that the Type column header exists
    await expect(leftPane.locator("button.col-header.col-type")).toBeVisible();

    // Navigate to Documents to check extension display
    await leftPane.locator('[data-testid="row-Documents"]').dblclick();
    await expect(leftPane.locator('[data-testid="row-report.pdf"] .col-type')).toHaveText("pdf");
    await expect(leftPane.locator('[data-testid="row-photos"] .col-type')).toHaveText("dir");
  });

  test("breadcrumb click navigates to that path", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');

    // Navigate deep: home -> Projects -> my-app
    await leftPane.locator('[data-testid="row-Projects"]').dblclick();
    await expect(leftPane.locator("text=my-app")).toBeVisible();
    await leftPane.locator('[data-testid="row-my-app"]').dblclick();
    await expect(leftPane.locator("text=package.json")).toBeVisible();

    // Click "testuser" in breadcrumb to go back to home
    const crumb = page.locator('[data-testid="breadcrumb-left"]');
    await crumb.locator("button", { hasText: "testuser" }).click();

    // Wait for home dir contents
    await expect(leftPane.locator("text=Documents")).toBeVisible();
  });
});

test.describe("Keyboard navigation", () => {
  test("Tab switches active pane", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');
    const rightPane = page.locator('[data-testid="pane-right"]');

    await expect(leftPane).toHaveClass(/active/);

    await page.keyboard.press("Tab");
    await expect(rightPane).toHaveClass(/active/);

    await page.keyboard.press("Tab");
    await expect(leftPane).toHaveClass(/active/);
  });

  test("Arrow keys move selection in active pane", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');

    // Press ArrowDown a few times then ArrowUp
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowUp");

    // Verify a selected row exists
    const selectedRow = leftPane.locator(".row.selected");
    await expect(selectedRow).toBeVisible();
  });

  test("Enter opens selected directory", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');

    // ".." is selected by default, press ArrowDown to select first entry (Desktop)
    await page.keyboard.press("ArrowDown");

    // Press Enter to open Desktop
    await page.keyboard.press("Enter");

    // Wait for Desktop contents
    await expect(leftPane.locator("text=todo.txt")).toBeVisible();
  });

  test("ArrowUp to '..' then Enter navigates up", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');

    // Navigate into Documents
    await leftPane.locator('[data-testid="row-Documents"]').dblclick();
    await expect(leftPane.locator("text=report.pdf")).toBeVisible();

    // ".." is already selected by default after entering a directory
    // Press Enter to go up
    await page.keyboard.press("Enter");

    // Wait for home dir
    await expect(leftPane.locator("text=Documents")).toBeVisible();
    await expect(leftPane.locator("text=Desktop")).toBeVisible();
  });

  test("going up re-selects the directory we came from", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');

    // Navigate into Documents
    await leftPane.locator('[data-testid="row-Documents"]').dblclick();
    await expect(leftPane.locator("text=report.pdf")).toBeVisible();

    // ".." is selected by default — press Enter to go back up
    await page.keyboard.press("Enter");

    // Should be back at home, and "Documents" row should be selected
    await expect(leftPane.locator("text=Desktop")).toBeVisible();
    await expect(leftPane.locator('[data-testid="row-Documents"].selected')).toBeVisible();
  });
});

test.describe("Compare mode", () => {
  test("pressing g enters compare mode with merged rows", async ({ page }) => {
    const leftPane = page.locator('[data-testid="pane-left"]');
    const rightPane = page.locator('[data-testid="pane-right"]');

    // Navigate left pane to Documents
    await leftPane.locator('[data-testid="row-Documents"]').dblclick();
    await expect(leftPane.locator("text=report.pdf")).toBeVisible();

    // Switch to right pane and navigate to Desktop
    await page.keyboard.press("Tab");
    await rightPane.locator('[data-testid="row-Desktop"]').dblclick();
    await expect(rightPane.locator("text=todo.txt")).toBeVisible();

    // Press 'g' to compare
    await page.keyboard.press("g");

    // ComparePane should appear with merged entries
    const comparePane = page.locator('[data-testid="compare-pane"]');
    await expect(comparePane).toBeVisible();

    // Summary should show onlyLeft and onlyRight counts
    await expect(page.locator(".sum-item.only-left")).toBeVisible();
    await expect(page.locator(".sum-item.only-right")).toBeVisible();

    // Status badges should be visible for entries
    await expect(comparePane.locator('[data-testid^="status-"]').first()).toBeVisible();
  });

  test("double-click dir in compare mode navigates deeper", async ({ page }) => {
    // Both panes start at home — compare shows same dirs
    await page.keyboard.press("g");

    const comparePane = page.locator('[data-testid="compare-pane"]');
    await expect(comparePane).toBeVisible();

    // Double-click a directory to navigate into it
    await comparePane.locator('[data-testid="compare-row-Documents"]').dblclick();

    // Should now see Documents contents
    await expect(comparePane.locator('[data-testid="compare-row-report.pdf"]')).toBeVisible();
  });

  test("backspace navigates up in compare mode", async ({ page }) => {
    await page.keyboard.press("g");
    const comparePane = page.locator('[data-testid="compare-pane"]');
    await expect(comparePane).toBeVisible();

    // Navigate into Documents
    await comparePane.locator('[data-testid="compare-row-Documents"]').dblclick();
    await expect(comparePane.locator('[data-testid="compare-row-report.pdf"]')).toBeVisible();

    // Backspace to go up
    await page.keyboard.press("Backspace");

    // Should be back at root compare level
    await expect(comparePane.locator('[data-testid="compare-row-Documents"]')).toBeVisible();
  });

  test("s toggles identical files in compare mode", async ({ page }) => {
    await page.keyboard.press("g");
    const comparePane = page.locator('[data-testid="compare-pane"]');
    await expect(comparePane).toBeVisible();

    // Wait for pending dirs to resolve (both panes are same dir — dirs resolve to "same")
    await expect(comparePane.locator('[data-testid="status-pending"]')).toHaveCount(0);

    // Both panes are same dir — all entries should be "same"
    const sameRows = comparePane.locator('.row.status-same');
    const initialCount = await sameRows.count();
    expect(initialCount).toBeGreaterThan(0);

    // Press 's' to hide identical
    await page.keyboard.press("s");

    // Same rows should be hidden
    await expect(comparePane.locator('.row.status-same')).toHaveCount(0);

    // Press 's' again to show them
    await page.keyboard.press("s");
    await expect(comparePane.locator('.row.status-same')).not.toHaveCount(0);
  });

  test("directories show spinners then resolve progressively", async ({ page }) => {
    // Both panes start at home — compare shows dirs with pending spinners
    await page.keyboard.press("g");
    const comparePane = page.locator('[data-testid="compare-pane"]');
    await expect(comparePane).toBeVisible();

    // Dirs should initially appear (as pending or already resolved)
    await expect(comparePane.locator('[data-testid="compare-row-Documents"]')).toBeVisible();

    // Wait for all pending spinners to disappear (dirs resolve one by one)
    await expect(comparePane.locator('[data-testid="status-pending"]')).toHaveCount(0);

    // After resolution, dir rows should have same/modified status badges
    const docRow = comparePane.locator('[data-testid="compare-row-Documents"]');
    await expect(docRow.locator('[data-testid="status-same"]')).toBeVisible();
  });

  test("Escape returns to browse mode after comparison", async ({ page }) => {
    await page.keyboard.press("g");
    await expect(page.locator('[data-testid="compare-pane"]')).toBeVisible();

    // Press Escape to go back to browse
    await page.keyboard.press("Escape");

    // Should be back in browse mode
    await expect(page.locator('[data-testid="pane-left"]')).toBeVisible();
    await expect(page.locator('[data-testid="pane-right"]')).toBeVisible();
  });
});

test.describe("File operations", () => {
  test("t opens mkdir prompt and Enter creates directory", async ({ page }) => {
    await page.keyboard.press("t");
    const input = page.locator('[data-testid="mkdir-input"]');
    await expect(input).toBeVisible();
    await expect(input).toBeFocused();
    await input.fill("new-folder");
    await input.press("Enter");
    await expect(input).not.toBeVisible();
  });

  test("mkdir prompt can be cancelled with Escape", async ({ page }) => {
    await page.keyboard.press("t");
    const input = page.locator('[data-testid="mkdir-input"]');
    await expect(input).toBeVisible();
    await input.press("Escape");
    await expect(input).not.toBeVisible();
  });
});

test.describe("Terminal panel", () => {
  test("backtick opens terminal, double-Escape closes it", async ({ page }) => {
    const panel = page.locator('[data-testid="terminal-panel"]');

    // Terminal should be hidden initially
    await expect(panel).toBeHidden();

    // Press backtick to show
    await page.keyboard.press("`");
    await expect(panel).toBeVisible();

    // Double-Escape closes terminal (single Escape passes to shell)
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");
    await expect(panel).toBeHidden();
  });

  test("terminal panel has drag handle and close button", async ({ page }) => {
    await page.keyboard.press("`");
    const handle = page.locator('[data-testid="terminal-drag-handle"]');
    await expect(handle).toBeVisible();
    const closeBtn = page.locator('[data-testid="terminal-close-btn"]');
    await expect(closeBtn).toBeVisible();
  });

  test("close button hides terminal panel", async ({ page }) => {
    await page.keyboard.press("`");
    const panel = page.locator('[data-testid="terminal-panel"]');
    await expect(panel).toBeVisible();

    await page.locator('[data-testid="terminal-close-btn"]').click();
    await expect(panel).toBeHidden();
  });

  test("terminal panel renders xterm container", async ({ page }) => {
    await page.keyboard.press("`");
    const panel = page.locator('[data-testid="terminal-panel"]');
    await expect(panel).toBeVisible();

    // xterm.js creates .xterm elements (one per side); check the active one is visible
    await expect(panel.locator(".xterm").first()).toBeVisible();
  });

  test("terminal shows side label matching active pane", async ({ page }) => {
    await page.keyboard.press("`");
    const panel = page.locator('[data-testid="terminal-panel"]');
    await expect(panel).toBeVisible();

    // Default active pane is left
    await expect(panel.locator(".side-label")).toHaveText("left");
  });

  test("footer shows terminal hints when terminal is open", async ({ page }) => {
    const footer = page.locator("footer");

    // Before opening terminal
    await expect(footer.locator("text=compare")).toBeVisible();

    // Open terminal
    await page.keyboard.press("`");
    await expect(footer.locator("text=close terminal")).toBeVisible();
    await expect(footer.locator("text=compare")).not.toBeVisible();
  });
});

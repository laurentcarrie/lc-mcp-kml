import { test, expect } from '@playwright/test'
import { setupMockRoutes } from './fixtures/setup'

test.beforeEach(async ({ page }) => {
  await setupMockRoutes(page)
  await page.goto('/configure')
})

test.describe('ConfigPage layout', () => {
  test('shows header with navigation and action buttons', async ({ page }) => {
    await expect(page.locator('button.back-btn')).toHaveText('Home')
    await expect(page.locator('h2')).toHaveText('Configure')
    await expect(page.locator('button.visualize-btn')).toBeVisible()
    await expect(page.locator('button.export-btn')).toBeVisible()
    await expect(page.locator('.import-btn')).toBeVisible()
  })

  test('shows prompt textarea and model selector', async ({ page }) => {
    await expect(page.locator('.prompt-input')).toBeVisible()
    await expect(page.locator('.model-select')).toBeVisible()
    await expect(page.locator('.prompt-btn')).toHaveText('Generate')
  })

  test('shows map container', async ({ page }) => {
    await expect(page.locator('.map-container')).toBeVisible()
  })

  test('back button navigates home', async ({ page }) => {
    await page.click('button.back-btn')
    await expect(page).toHaveURL('/')
  })
})

test.describe('Choice tree management', () => {
  test('can add a Folder', async ({ page }) => {
    await page.click('.tree-add-folder')
    await expect(page.locator('.tree-item')).toHaveCount(1)
    await expect(page.locator('.tree-type-hint')).toHaveText('Folder')
  })

  test('can add multiple items via folder + button', async ({ page }) => {
    // Add a folder
    await page.click('.tree-add-folder')
    // Expand it
    await page.locator('.tree-toggle').click()
    // Click + inside the folder
    await page.locator('.tree-add-btn').click()
    // Choose Point from menu
    await page.locator('.tree-add-menu-item', { hasText: 'Point' }).click()
    // Should have folder with one child
    await expect(page.locator('.tree-item')).toHaveCount(2)
  })

  test('can remove an item', async ({ page }) => {
    await page.click('.tree-add-folder')
    await expect(page.locator('.tree-item')).toHaveCount(1)
    await page.locator('.tree-remove').click()
    await expect(page.locator('.tree-item')).toHaveCount(0)
  })

  test('can rename a folder', async ({ page }) => {
    await page.click('.tree-add-folder')
    await page.locator('.tree-toggle').click()
    const nameInput = page.locator('.tree-detail input[type="text"]').first()
    await nameInput.fill('My Folder')
    await expect(page.locator('.tree-label')).toHaveText('My Folder')
  })

  test('can toggle visibility with eye button', async ({ page }) => {
    await page.click('.tree-add-folder')
    const item = page.locator('.tree-item').first()
    // Initially visible
    await expect(item).not.toHaveClass(/tree-hidden/)
    // Click eye to hide
    await page.locator('.tree-eye').click()
    await expect(item).toHaveClass(/tree-hidden/)
    // Click again to show
    await page.locator('.tree-eye').click()
    await expect(item).not.toHaveClass(/tree-hidden/)
  })
})

test.describe('Add menu types', () => {
  for (const type of ['ConcentricCircles', 'Point', 'Folder', 'UnionCircles', 'Segments', 'TriangleBisect', 'RawKml', 'Route']) {
    test(`can add ${type} via folder menu`, async ({ page }) => {
      await page.click('.tree-add-folder')
      await page.locator('.tree-toggle').click()
      await page.locator('.tree-add-btn').click()
      await page.locator('.tree-add-menu-item', { hasText: type }).click()
      await expect(page.locator('.tree-type-hint').nth(1)).toHaveText(type)
    })
  }
})

test.describe('Visualize', () => {
  test('sends generate request and renders KML on map', async ({ page }) => {
    // Add a folder so there's something to send
    await page.click('.tree-add-folder')

    const [request] = await Promise.all([
      page.waitForRequest('**/api/generate'),
      page.click('button.visualize-btn'),
    ])

    expect(request.method()).toBe('POST')
    const body = request.postDataJSON()
    expect(body).toHaveProperty('choices')

    // Map should have leaflet layers rendered (SVG paths from KML)
    await expect(page.locator('.leaflet-overlay-pane svg path')).toHaveCount(1, { timeout: 5000 })
  })

  test('shows error on failed generate', async ({ page }) => {
    // Override the mock to return an error
    await page.route('**/api/generate', route =>
      route.fulfill({ status: 500, body: 'Server error' })
    )
    await page.click('.tree-add-folder')
    await page.click('button.visualize-btn')
    await expect(page.locator('.error-bar')).toBeVisible()
    await expect(page.locator('.error-bar')).toContainText('Server error')
  })
})

test.describe('LLM prompt', () => {
  test('submits prompt and adds choices to tree', async ({ page }) => {
    await page.locator('.prompt-input').fill('show RER A stations')

    const [request] = await Promise.all([
      page.waitForRequest('**/api/prompt'),
      page.click('.prompt-btn'),
    ])

    expect(request.method()).toBe('POST')
    const body = request.postDataJSON()
    expect(body.prompt).toBe('show RER A stations')
    expect(body.model).toBeTruthy()

    // Should have added a Point choice from mock response
    await expect(page.locator('.tree-item')).toHaveCount(1, { timeout: 5000 })
    await expect(page.locator('.tree-label').first()).toContainText('Nanterre')
  })

  test('Enter key submits prompt', async ({ page }) => {
    const promptInput = page.locator('.prompt-input')
    await promptInput.fill('test query')
    await promptInput.press('Enter')
    // Wait for the request to complete
    await expect(page.locator('.tree-item')).toHaveCount(1, { timeout: 5000 })
  })
})

test.describe('Import / Export JSON', () => {
  test('export creates downloadable JSON', async ({ page }) => {
    // Add an item first
    await page.click('.tree-add-folder')
    await page.locator('.tree-toggle').click()
    const nameInput = page.locator('.tree-detail input[type="text"]').first()
    await nameInput.fill('Export Test')

    const [download] = await Promise.all([
      page.waitForEvent('download'),
      page.click('button.export-btn'),
    ])
    expect(download.suggestedFilename()).toBe('input.json')
  })

  test('import loads JSON into tree', async ({ page }) => {
    const json = JSON.stringify({
      choices: [
        { Point: { kml: 'rer/RER-A.kml', name: 'Gare de Lyon', color: null } },
        { Folder: { name: 'Test Folder', choices: [] } },
      ]
    })

    // Create a file chooser and upload
    const fileChooserPromise = page.waitForEvent('filechooser')
    await page.locator('.import-btn').click()
    const fileChooser = await fileChooserPromise
    await fileChooser.setFiles({
      name: 'test.json',
      mimeType: 'application/json',
      buffer: Buffer.from(json),
    })

    await expect(page.locator('.tree-item')).toHaveCount(2)
    await expect(page.locator('.tree-label').first()).toContainText('Gare de Lyon')
    await expect(page.locator('.tree-label').nth(1)).toContainText('Test Folder')
  })
})

test.describe('Route display', () => {
  test('displays route with distance and duration on map and in tree', async ({ page }) => {
    // Import a config with a Route choice
    const json = JSON.stringify({
      choices: [
        {
          Route: {
            name: 'Test Route',
            from: { kml: 'rer/RER-A.kml', name: 'Station A', color: null },
            to: { kml: 'rer/RER-A.kml', name: 'Station B', color: null },
            color: 'ff0000ff',
            mode: 'foot',
          },
        },
      ],
    })
    const fileChooserPromise = page.waitForEvent('filechooser')
    await page.locator('.import-btn').click()
    const fileChooser = await fileChooserPromise
    await fileChooser.setFiles({
      name: 'route.json',
      mimeType: 'application/json',
      buffer: Buffer.from(json),
    })

    // Verify Route appears in the tree
    await expect(page.locator('.tree-item')).toHaveCount(1)
    await expect(page.locator('.tree-type-hint')).toHaveText('Route')
    await expect(page.locator('.tree-label')).toContainText('Test Route')

    // Expand and verify route editor fields
    await page.locator('.tree-label').click()
    await expect(page.locator('.sub-label', { hasText: 'From' })).toBeVisible()
    await expect(page.locator('.sub-label', { hasText: 'To' })).toBeVisible()
    await expect(page.locator('select').filter({ hasText: 'Walking' })).toBeVisible()

    // Visualize - the mock returns KML with "Test Route (1.2 km, 15 min)"
    await Promise.all([
      page.waitForRequest('**/api/generate'),
      page.click('button.visualize-btn'),
    ])

    // Route line should be rendered on the map
    await expect(page.locator('.leaflet-overlay-pane svg path')).toHaveCount(1, { timeout: 5000 })

    // Route tooltip with distance/duration should appear on the map
    await expect(page.locator('.route-tooltip')).toBeVisible({ timeout: 5000 })
    await expect(page.locator('.route-tooltip')).toContainText('1.2 km, 15 min')

    // Tree label should update with route info after visualization
    await expect(page.locator('.tree-label')).toContainText('1.2 km, 15 min')
  })
})

test.describe('Editor toggle', () => {
  test('can hide and show the editor panel', async ({ page }) => {
    await expect(page.locator('.config-editor')).toBeVisible()
    await page.click('.toggle-editor-btn')
    await expect(page.locator('.config-editor')).not.toBeVisible()
    await page.click('.toggle-editor-btn')
    await expect(page.locator('.config-editor')).toBeVisible()
  })
})

test.describe('Pin point mode', () => {
  test('toggling pin mode adds crosshair class to map', async ({ page }) => {
    const map = page.locator('.map-container')
    await expect(map).not.toHaveClass(/map-crosshair/)
    await page.click('.pin-btn')
    await expect(map).toHaveClass(/map-crosshair/)
    await expect(page.locator('.map-click-hint')).toBeVisible()
    // Toggle off
    await page.click('.pin-btn')
    await expect(map).not.toHaveClass(/map-crosshair/)
  })
})

test.describe('Model selection', () => {
  test('persists model choice', async ({ page }) => {
    await page.locator('.model-select').selectOption('gpt-4o')
    // Reload page
    await page.reload()
    await expect(page.locator('.model-select')).toHaveValue('gpt-4o')
  })
})

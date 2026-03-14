import { Page } from '@playwright/test'
import { mockLibraryFiles, mockGenerateKml } from './mock-data'

export async function setupMockRoutes(page: Page) {
  // Mock S3 library listing
  await page.route('**/api/list', route =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(mockLibraryFiles),
    })
  )

  // Mock KML generation
  await page.route('**/api/generate', route =>
    route.fulfill({
      status: 200,
      contentType: 'application/vnd.google-earth.kml+xml',
      body: mockGenerateKml,
    })
  )

  // Mock LLM prompt
  await page.route('**/api/prompt', route =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        choices: [
          {
            Point: {
              kml: 'rer/RER-A.kml',
              name: 'Nanterre - Préfecture',
              color: 'ff0000ff',
            },
          },
        ],
      }),
    })
  )

  // Mock S3 proxy for KML files
  await page.route('**/api/rer/**', route =>
    route.fulfill({
      status: 200,
      contentType: 'application/vnd.google-earth.kml+xml',
      body: mockKmlFile,
    })
  )

  await page.route('**/api/bus/**', route =>
    route.fulfill({
      status: 200,
      contentType: 'application/vnd.google-earth.kml+xml',
      body: mockKmlFile,
    })
  )
}

const mockKmlFile = `<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
<Document>
  <name>Test Line</name>
  <Style id="stop_style">
    <IconStyle>
      <color>ff0000ff</color>
      <scale>0.6</scale>
      <Icon><href>https://maps.google.com/mapfiles/kml/shapes/bus.png</href></Icon>
    </IconStyle>
  </Style>
  <Placemark>
    <name>Station A</name>
    <styleUrl>#stop_style</styleUrl>
    <Point><coordinates>2.19,48.92,0</coordinates></Point>
  </Placemark>
  <Placemark>
    <name>Station B</name>
    <styleUrl>#stop_style</styleUrl>
    <Point><coordinates>2.20,48.93,0</coordinates></Point>
  </Placemark>
</Document>
</kml>`

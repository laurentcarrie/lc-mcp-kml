export const mockLibraryFiles = [
  { key: 'rer/RER-A.kml', size: 12345 },
  { key: 'rer/RER-B.kml', size: 23456 },
  { key: 'bus/bus-1.kml', size: 5000 },
  { key: 'bus/bus-26.kml', size: 6000 },
  { key: '94/94017_Champigny-sur-Marne.kml', size: 8000 },
  { key: '75/75056_Paris.kml', size: 11000 },
]

export const mockGenerateKml = `<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
<Document>
  <Style id="style_1">
    <LineStyle><color>ff0000ff</color><width>3</width></LineStyle>
  </Style>
  <Style id="style_point">
    <IconStyle>
      <color>ff0000ff</color>
      <scale>0.6</scale>
      <Icon><href>https://maps.google.com/mapfiles/kml/shapes/bus.png</href></Icon>
    </IconStyle>
  </Style>
  <Placemark>
    <name>Test Point</name>
    <styleUrl>#style_point</styleUrl>
    <Point><coordinates>2.19,48.92,0</coordinates></Point>
  </Placemark>
  <Placemark>
    <name>Test Route (1.2 km, 15 min)</name>
    <styleUrl>#style_1</styleUrl>
    <LineString>
      <coordinates>2.19,48.92,0 2.20,48.93,0 2.21,48.94,0</coordinates>
    </LineString>
  </Placemark>
</Document>
</kml>`

import { useState, useEffect, useRef } from 'react'
import { useSearchParams, useNavigate } from 'react-router-dom'
import L from 'leaflet'
import 'leaflet/dist/leaflet.css'
import omnivore from 'leaflet-omnivore'
import LibraryModal from '../components/LibraryModal'
import './MapPage.css'

const API_BASE = import.meta.env.VITE_API_BASE || '/api'

function kmlColorToHex(kmlColor) {
  if (!kmlColor || kmlColor.length !== 8) return '#3388ff'
  const r = kmlColor.substring(6, 8)
  const g = kmlColor.substring(4, 6)
  const b = kmlColor.substring(2, 4)
  return '#' + r + g + b
}

export default function MapPage() {
  const navigate = useNavigate()
  const [searchParams, setSearchParams] = useSearchParams()
  const [kmlPath, setKmlPath] = useState(searchParams.get('kml') || '')
  const [libraryOpen, setLibraryOpen] = useState(false)
  const mapRef = useRef(null)
  const mapInstance = useRef(null)
  const layersRef = useRef({})

  useEffect(() => {
    if (!mapInstance.current) {
      mapInstance.current = L.map(mapRef.current).setView([48.92, 2.19], 13)
      L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
        attribution: '&copy; OpenStreetMap contributors'
      }).addTo(mapInstance.current)
    }
    return () => {
      if (mapInstance.current) {
        mapInstance.current.remove()
        mapInstance.current = null
      }
    }
  }, [])

  useEffect(() => {
    const kml = searchParams.get('kml')
    if (kml) {
      setKmlPath(kml)
      loadKml(kml)
    }
  }, [searchParams])

  function loadKml(path) {
    const p = path || kmlPath
    if (!p) return
    setSearchParams({ kml: p })

    // Remove previous layer for the same path, keep others
    if (layersRef.current[p]) {
      mapInstance.current.removeLayer(layersRef.current[p])
      delete layersRef.current[p]
    }

    const url = API_BASE + '/' + p
    fetch(url)
      .then(r => r.text())
      .then(kmlText => {
        const parser = new DOMParser()
        const doc = parser.parseFromString(kmlText, 'text/xml')

        const styles = {}
        doc.querySelectorAll('Style').forEach(s => {
          const id = s.getAttribute('id')
          const lineEl = s.querySelector('LineStyle > color')
          const polyEl = s.querySelector('PolyStyle > color')
          const iconHref = s.querySelector('IconStyle > Icon > href')
          const iconColor = s.querySelector('IconStyle > color')
          if (id) {
            const polyColorRaw = polyEl ? polyEl.textContent.trim() : null
            const lineColorRaw = lineEl ? lineEl.textContent.trim() : null
            styles[id] = {
              lineColor: lineColorRaw ? kmlColorToHex(lineColorRaw) : null,
              lineAlpha: lineColorRaw ? parseInt(lineColorRaw.substring(0, 2), 16) / 255 : null,
              polyColor: polyColorRaw ? kmlColorToHex(polyColorRaw) : null,
              polyAlpha: polyColorRaw ? parseInt(polyColorRaw.substring(0, 2), 16) / 255 : null,
              iconHref: iconHref ? iconHref.textContent.trim() : null,
              iconColor: iconColor ? iconColor.textContent.trim() : null,
            }
          }
        })

        const layer = omnivore.kml.parse(kmlText)
        const applyStyles = (parentLayer) => {
          parentLayer.eachLayer(l => {
            if (l.eachLayer && !l.feature) { applyStyles(l); return }
            if (l.feature?.properties?.styleUrl) {
              const styleId = l.feature.properties.styleUrl.replace('#', '')
              const style = styles[styleId]
              if (style) {
                if (style.iconHref && l.setIcon) {
                  if (style.iconColor) {
                    const c = kmlColorToHex(style.iconColor)
                    l.setIcon(L.divIcon({
                      className: '',
                      html: `<svg width="18" height="18" viewBox="0 0 18 18"><rect x="1" y="1" width="16" height="16" rx="4" fill="${c}" stroke="white" stroke-width="1.5"/><path d="M5.5 4.5h7a1.5 1.5 0 0 1 1.5 1.5v5a1.5 1.5 0 0 1-1.5 1.5h-7A1.5 1.5 0 0 1 4 11V6a1.5 1.5 0 0 1 1.5-1.5zM6 8h6M6 10h6M6.5 12.5l-1 2M11.5 12.5l1 2" stroke="white" stroke-width="1" fill="none" stroke-linecap="round"/></svg>`,
                      iconSize: [18, 18],
                      iconAnchor: [9, 9],
                      popupAnchor: [0, -9],
                    }))
                  } else {
                    l.setIcon(L.icon({
                      iconUrl: style.iconHref,
                      iconSize: [24, 24],
                      iconAnchor: [12, 12],
                      popupAnchor: [0, -12],
                    }))
                  }
                }
                if (l.setStyle) {
                  const styleObj = {}
                  if (style.lineColor) {
                    styleObj.color = style.lineColor
                    styleObj.weight = 3
                    styleObj.opacity = style.lineAlpha ?? 1
                  }
                  if (style.polyColor) {
                    styleObj.fillColor = style.polyColor
                    styleObj.fillOpacity = style.polyAlpha ?? 0.3
                    styleObj.fill = true
                    if (!style.lineColor) {
                      styleObj.color = style.polyColor
                      styleObj.weight = 2
                      styleObj.opacity = style.polyAlpha ?? 0.3
                    }
                  }
                  if (Object.keys(styleObj).length) l.setStyle(styleObj)
                }
              }
            }
            if (l.feature?.properties?.name) {
              l.bindPopup('<b>' + l.feature.properties.name + '</b>')
            }
          })
        }
        applyStyles(layer)
        layer.addTo(mapInstance.current)
        mapInstance.current.fitBounds(layer.getBounds().pad(0.1))
        layersRef.current[p] = layer
      })
  }

  return (
    <div className="map-page">
      <div className="map-bar">
        <button className="back-btn" onClick={() => navigate('/')}>Home</button>
        <input
          type="text"
          placeholder="KML path (e.g. bus/bus-1.kml)"
          value={kmlPath}
          onChange={e => setKmlPath(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && loadKml()}
        />
        <button className="load-btn" onClick={() => loadKml()}>Load</button>
        <button className="library-btn" onClick={() => setLibraryOpen(true)}>Library</button>
      </div>
      <div className="map-container" ref={mapRef} />
      <LibraryModal
        open={libraryOpen}
        onClose={() => setLibraryOpen(false)}
        onSelect={key => { setLibraryOpen(false); setKmlPath(key); loadKml(key) }}
      />
    </div>
  )
}

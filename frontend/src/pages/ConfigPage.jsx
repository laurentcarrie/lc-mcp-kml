import { useState, useEffect, useRef, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import L from 'leaflet'
import 'leaflet/dist/leaflet.css'
import omnivore from 'leaflet-omnivore'
import LibraryModal from '../components/LibraryModal'
import './ConfigPage.css'

const API_BASE = import.meta.env.VITE_API_BASE || '/api'

const CHOICE_TYPES = [
  'ConcentricCircles', 'Point', 'Folder', 'UnionCircles',
  'Segments', 'TriangleBisect', 'RawKml'
]

function makeDefault(type) {
  switch (type) {
    case 'ConcentricCircles': return {
      ConcentricCircles: {
        center: { kml: '', name: '', color: null },
        name: '',
        v_radius: [500, 1000, 1500],
        circle_on_top: false,
        colors: null,
      }
    }
    case 'Point': return {
      Point: { kml: '', name: '', color: null }
    }
    case 'Folder': return {
      Folder: { name: '', choices: [] }
    }
    case 'UnionCircles': return {
      UnionCircles: { name: '', centers: [], radius: 500, circle_on_top: false, color: null }
    }
    case 'Segments': return {
      Segments: { name: '', kml: '', neighbours: [], color: null }
    }
    case 'TriangleBisect': return {
      TriangleBisect: {
        point1: { kml: '', name: '', color: null },
        point2: { kml: '', name: '', color: null },
        radius_factor: 0.6,
      }
    }
    case 'RawKml': return {
      RawKml: { path: '', color: null, alpha: 0.0 }
    }
    default: return null
  }
}

function getType(choice) {
  return Object.keys(choice)[0]
}

function getData(choice) {
  return Object.values(choice)[0]
}

function Field({ label, value, onChange, type = 'text', placeholder }) {
  return (
    <div className="field">
      <label>{label}</label>
      {type === 'checkbox' ? (
        <input type="checkbox" checked={!!value} onChange={e => onChange(e.target.checked)} />
      ) : (
        <input type={type} value={value ?? ''} onChange={e => onChange(type === 'number' ? parseFloat(e.target.value) || 0 : e.target.value)} placeholder={placeholder} />
      )}
    </div>
  )
}

function PointEditor({ point, onChange }) {
  const update = (k, v) => onChange({ ...point, [k]: v })
  const [libraryOpen, setLibraryOpen] = useState(false)
  const [placemarks, setPlacemarks] = useState([])
  const [loadingKml, setLoadingKml] = useState(false)

  const fetchPlacemarks = useCallback((kmlPath) => {
    if (!kmlPath) { setPlacemarks([]); return }
    setLoadingKml(true)
    fetch(API_BASE + '/' + kmlPath)
      .then(r => { if (!r.ok) throw new Error('Not found'); return r.text() })
      .then(text => {
        const doc = new DOMParser().parseFromString(text, 'text/xml')
        const names = []
        doc.querySelectorAll('Placemark').forEach(pm => {
          if (!pm.querySelector('Point')) return
          const nameEl = pm.querySelector('name')
          const n = nameEl ? nameEl.textContent.trim() : ''
          if (n) names.push(n)
        })
        setPlacemarks(names)
      })
      .catch(() => setPlacemarks([]))
      .finally(() => setLoadingKml(false))
  }, [])

  useEffect(() => {
    fetchPlacemarks(point.kml)
  }, [point.kml, fetchPlacemarks])

  useEffect(() => {
    if (placemarks.length > 0 && !point.name) {
      onChange({ ...point, name: placemarks[0] })
    }
  }, [placemarks])

  return (
    <div className="sub-editor point-editor">
      <div className="field">
        <label>KML library path</label>
        <input
          type="text"
          value={point.kml ?? ''}
          onChange={e => update('kml', e.target.value)}
          placeholder="e.g. bus/bus-1.kml"
        />
        <button className="browse-btn" onClick={() => setLibraryOpen(true)}>Browse</button>
      </div>
      <div className="field">
        <label>Name</label>
        {loadingKml ? (
          <input type="text" disabled placeholder="Loading placemarks..." />
        ) : placemarks.length > 0 ? (
          <select value={point.name} onChange={e => update('name', e.target.value)}>
            {placemarks.map((n, i) => <option key={i} value={n}>{n}</option>)}
          </select>
        ) : point.kml ? (
          <span className="field-hint">No point placemarks in this file (use RawKml for polygons)</span>
        ) : (
          <input
            type="text"
            value={point.name ?? ''}
            onChange={e => update('name', e.target.value)}
            placeholder="placemark name"
          />
        )}
      </div>
      <Field label="Color" value={point.color} onChange={v => update('color', v || null)} placeholder="AABBGGRR (optional)" />
      <LibraryModal
        open={libraryOpen}
        onClose={() => setLibraryOpen(false)}
        onSelect={key => { update('kml', key); setLibraryOpen(false) }}
      />
    </div>
  )
}

function RadiiEditor({ radii, onChange }) {
  const text = radii.join(', ')
  return (
    <Field
      label="Radii"
      value={text}
      onChange={v => onChange(v.split(',').map(s => parseFloat(s.trim())).filter(n => !isNaN(n)))}
      placeholder="500, 1000, 1500"
    />
  )
}

function ColorsEditor({ colors, onChange }) {
  const text = (colors || []).join(', ')
  return (
    <Field
      label="Colors"
      value={text}
      onChange={v => {
        const arr = v.split(',').map(s => s.trim()).filter(Boolean)
        onChange(arr.length ? arr : null)
      }}
      placeholder="AABBGGRR, ... (optional)"
    />
  )
}

function NeighboursEditor({ neighbours, onChange }) {
  return (
    <div className="neighbours-editor">
      <label>Neighbours</label>
      {neighbours.map((pair, i) => (
        <div key={i} className="neighbour-row">
          <input value={pair[0]} onChange={e => {
            const n = [...neighbours]
            n[i] = [e.target.value, pair[1]]
            onChange(n)
          }} placeholder="Station A" />
          <span>-</span>
          <input value={pair[1]} onChange={e => {
            const n = [...neighbours]
            n[i] = [pair[0], e.target.value]
            onChange(n)
          }} placeholder="Station B" />
          <button className="remove-btn small" onClick={() => onChange(neighbours.filter((_, j) => j !== i))}>x</button>
        </div>
      ))}
      <button className="add-btn small" onClick={() => onChange([...neighbours, ['', '']])}>+ Neighbour</button>
    </div>
  )
}

function RawKmlEditor({ data, update }) {
  const [libraryOpen, setLibraryOpen] = useState(false)
  return (
    <>
      <div className="field">
        <label>Path</label>
        <input type="text" value={data.path ?? ''} onChange={e => update({ ...data, path: e.target.value })} placeholder="path/to/file.kml" />
        <button className="browse-btn" onClick={() => setLibraryOpen(true)}>Browse</button>
      </div>
      <Field label="Color" value={data.color} onChange={v => update({ ...data, color: v || null })} placeholder="AABBGGRR (optional)" />
      <Field label="Alpha" value={data.alpha} onChange={v => update({ ...data, alpha: v })} type="number" />
      <LibraryModal
        open={libraryOpen}
        onClose={() => setLibraryOpen(false)}
        onSelect={key => { update({ ...data, path: key }); setLibraryOpen(false) }}
      />
    </>
  )
}

function ChoiceEditor({ choice, onChange, onRemove, onMoveUp, onMoveDown }) {
  const type = getType(choice)
  const data = getData(choice)
  const update = (newData) => onChange({ [type]: newData })

  return (
    <div className="choice-editor">
      <div className="choice-header">
        <span className="choice-type">{type}</span>
        <div className="choice-actions">
          <button className="move-btn" onClick={onMoveUp} title="Move up">^</button>
          <button className="move-btn" onClick={onMoveDown} title="Move down">v</button>
          <button className="remove-btn" onClick={onRemove} title="Remove">x</button>
        </div>
      </div>
      <div className="choice-body">
        {type === 'ConcentricCircles' && <>
          <Field label="Name" value={data.name} onChange={v => update({ ...data, name: v })} />
          <label className="sub-label">Center</label>
          <PointEditor point={data.center} onChange={v => update({ ...data, center: v })} />
          <RadiiEditor radii={data.v_radius} onChange={v => update({ ...data, v_radius: v })} />
          <Field label="Circle on top" value={data.circle_on_top} onChange={v => update({ ...data, circle_on_top: v })} type="checkbox" />
          <ColorsEditor colors={data.colors} onChange={v => update({ ...data, colors: v })} />
        </>}
        {type === 'Point' && <PointEditor point={data} onChange={v => update(v)} />}
        {type === 'Folder' && <>
          <Field label="Name" value={data.name} onChange={v => update({ ...data, name: v })} />
          <ChoiceList choices={data.choices} onChange={v => update({ ...data, choices: v })} />
        </>}
        {type === 'UnionCircles' && <>
          <Field label="Name" value={data.name} onChange={v => update({ ...data, name: v })} />
          <Field label="Radius" value={data.radius} onChange={v => update({ ...data, radius: v })} type="number" />
          <Field label="Circle on top" value={data.circle_on_top} onChange={v => update({ ...data, circle_on_top: v })} type="checkbox" />
          <Field label="Color" value={data.color} onChange={v => update({ ...data, color: v || null })} placeholder="AABBGGRR (optional)" />
          <label className="sub-label">Centers</label>
          {data.centers.map((c, i) => (
            <div key={i} className="center-row">
              <PointEditor point={c} onChange={v => {
                const centers = [...data.centers]
                centers[i] = v
                update({ ...data, centers })
              }} />
              <button className="remove-btn small" onClick={() => update({ ...data, centers: data.centers.filter((_, j) => j !== i) })}>x</button>
            </div>
          ))}
          <button className="add-btn small" onClick={() => update({ ...data, centers: [...data.centers, { kml: '', name: '', color: null }] })}>+ Center</button>
        </>}
        {type === 'Segments' && <>
          <Field label="Name" value={data.name} onChange={v => update({ ...data, name: v })} />
          <Field label="KML file" value={data.kml} onChange={v => update({ ...data, kml: v })} />
          <Field label="Color" value={data.color} onChange={v => update({ ...data, color: v || null })} placeholder="AABBGGRR (optional)" />
          <NeighboursEditor neighbours={data.neighbours} onChange={v => update({ ...data, neighbours: v })} />
        </>}
        {type === 'TriangleBisect' && <>
          <label className="sub-label">Point 1</label>
          <PointEditor point={data.point1} onChange={v => update({ ...data, point1: v })} />
          <label className="sub-label">Point 2</label>
          <PointEditor point={data.point2} onChange={v => update({ ...data, point2: v })} />
          <Field label="Radius factor" value={data.radius_factor} onChange={v => update({ ...data, radius_factor: v })} type="number" />
        </>}
        {type === 'RawKml' && <RawKmlEditor data={data} update={update} />}
      </div>
    </div>
  )
}

function ChoiceList({ choices, onChange }) {
  const addChoice = (type) => {
    onChange([...choices, makeDefault(type)])
  }
  const updateChoice = (i, val) => {
    const c = [...choices]
    c[i] = val
    onChange(c)
  }
  const removeChoice = (i) => onChange(choices.filter((_, j) => j !== i))
  const moveChoice = (i, dir) => {
    const j = i + dir
    if (j < 0 || j >= choices.length) return
    const c = [...choices]
    ;[c[i], c[j]] = [c[j], c[i]]
    onChange(c)
  }

  return (
    <div className="choice-list">
      {choices.map((ch, i) => (
        <ChoiceEditor
          key={i}
          choice={ch}
          onChange={val => updateChoice(i, val)}
          onRemove={() => removeChoice(i)}
          onMoveUp={() => moveChoice(i, -1)}
          onMoveDown={() => moveChoice(i, 1)}
        />
      ))}
      <div className="add-choice">
        {CHOICE_TYPES.map(t => (
          <button key={t} className="add-btn" onClick={() => addChoice(t)}>+ {t}</button>
        ))}
      </div>
    </div>
  )
}

function kmlColorToHex(kmlColor) {
  if (!kmlColor || kmlColor.length !== 8) return '#3388ff'
  const r = kmlColor.substring(6, 8)
  const g = kmlColor.substring(4, 6)
  const b = kmlColor.substring(2, 4)
  return '#' + r + g + b
}

export default function ConfigPage() {
  const navigate = useNavigate()
  const [choices, setChoices] = useState([])
  const [error, setError] = useState(null)
  const [loading, setLoading] = useState(false)
  const [prompt, setPrompt] = useState('')
  const [prompting, setPrompting] = useState(false)
  const mapRef = useRef(null)
  const mapInstance = useRef(null)
  const currentLayer = useRef(null)

  useEffect(() => {
    if (!mapInstance.current && mapRef.current) {
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

  async function visualize(overrideChoices) {
    setError(null)
    setLoading(true)
    try {
      const resp = await fetch(API_BASE + '/generate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ choices: overrideChoices || choices }),
      })
      if (!resp.ok) {
        const msg = await resp.text()
        throw new Error(msg || resp.statusText)
      }
      const kmlText = await resp.text()

      if (currentLayer.current) {
        mapInstance.current.removeLayer(currentLayer.current)
        currentLayer.current = null
      }

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
          styles[id] = {
            lineColor: lineEl ? kmlColorToHex(lineEl.textContent.trim()) : null,
            polyColor: polyEl ? kmlColorToHex(polyEl.textContent.trim()) : null,
            iconHref: iconHref ? iconHref.textContent.trim() : null,
            iconColor: iconColor ? iconColor.textContent.trim() : null,
          }
        }
      })

      const layer = omnivore.kml.parse(kmlText)
      layer.eachLayer(l => {
        if (l.feature?.properties?.styleUrl) {
          const styleId = l.feature.properties.styleUrl.replace('#', '')
          const style = styles[styleId]
          if (style) {
            if (style.iconHref && l.setIcon) {
              l.setIcon(L.icon({
                iconUrl: style.iconHref,
                iconSize: [24, 24],
                iconAnchor: [12, 12],
                popupAnchor: [0, -12],
              }))
            }
            const color = style.lineColor || style.polyColor
            if (color && l.setStyle) {
              l.setStyle({ color, weight: 3 })
            }
          }
        }
        if (l.feature?.properties?.name) {
          l.bindPopup('<b>' + l.feature.properties.name + '</b>')
        }
      })
      layer.addTo(mapInstance.current)
      if (layer.getLayers().length > 0) {
        mapInstance.current.fitBounds(layer.getBounds().pad(0.1))
      }
      currentLayer.current = layer
    } catch (e) {
      console.error('Visualize error:', e)
      setError(e.message)
    } finally {
      setLoading(false)
    }
  }

  function exportJson() {
    const blob = new Blob([JSON.stringify({ choices }, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'input.json'
    a.click()
    URL.revokeObjectURL(url)
  }

  function importJson(e) {
    const file = e.target.files[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = () => {
      try {
        const data = JSON.parse(reader.result)
        if (data.choices) setChoices(data.choices)
        else setError('Invalid JSON: missing "choices" field')
      } catch (err) {
        setError('Invalid JSON: ' + err.message)
      }
    }
    reader.readAsText(file)
  }

  async function generateFromPrompt() {
    if (!prompt.trim()) return
    setError(null)
    setPrompting(true)
    try {
      const resp = await fetch(API_BASE + '/prompt', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ prompt: prompt.trim() }),
      })
      if (!resp.ok) {
        const msg = await resp.text()
        throw new Error(msg || resp.statusText)
      }
      const data = await resp.json()
      if (data.choices) {
        setChoices(data.choices)
        // Auto-visualize after successful prompt
        setTimeout(() => visualize(data.choices), 100)
      }
      else throw new Error('Invalid response: missing choices')
    } catch (e) {
      console.error('Prompt error:', e)
      setError(e.message)
    } finally {
      setPrompting(false)
    }
  }

  return (
    <div className="config-page">
      <div className="config-bar">
        <button className="back-btn" onClick={() => navigate('/')}>Home</button>
        <h2>Configure</h2>
        <div className="config-bar-actions">
          <label className="import-btn">
            Import JSON
            <input type="file" accept=".json" onChange={importJson} hidden />
          </label>
          <button className="export-btn" onClick={exportJson}>Export JSON</button>
          <button className="visualize-btn" onClick={visualize} disabled={loading}>
            {loading ? 'Generating...' : 'Visualize'}
          </button>
        </div>
      </div>
      <div className="config-split">
        <div className="config-editor">
          <div className="prompt-bar">
            <textarea
              className="prompt-input"
              value={prompt}
              onChange={e => setPrompt(e.target.value)}
              placeholder="Describe what to show on the map, e.g.: draw a blue circle of radius 500m around each station of RER A"
              onKeyDown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); generateFromPrompt() } }}
              rows={2}
            />
            <button className="prompt-btn" onClick={generateFromPrompt} disabled={prompting}>
              {prompting ? 'Thinking...' : 'Generate'}
            </button>
          </div>
          <ChoiceList choices={choices} onChange={setChoices} />
        </div>
        <div className="config-map">
          {error && <div className="error-bar">{error}</div>}
          <div className="map-container" ref={mapRef} />
        </div>
      </div>
    </div>
  )
}

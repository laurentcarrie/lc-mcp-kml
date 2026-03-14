import { useState, useEffect, useRef, useCallback, createContext, useContext } from 'react'
import { useNavigate } from 'react-router-dom'
import L from 'leaflet'
import 'leaflet/dist/leaflet.css'
import omnivore from 'leaflet-omnivore'
import LibraryModal from '../components/LibraryModal'
import './ConfigPage.css'

const API_BASE = import.meta.env.VITE_API_BASE || '/api'
const ChoicesContext = createContext([])

const CHOICE_TYPES = [
  'ConcentricCircles', 'Point', 'Folder', 'UnionCircles',
  'Segments', 'TriangleBisect', 'RawKml', 'Route'
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
    case 'Route': return {
      Route: {
        name: '',
        from: { kml: '', name: '', color: null },
        to: { kml: '', name: '', color: null },
        color: null,
        mode: 'foot',
      }
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

function ColorField({ label, value, onChange }) {
  // value is KML AABBGGRR or null; display as HTML color picker
  const kmlToHex = (kml) => {
    if (!kml || kml.length !== 8) return '#3388ff'
    return '#' + kml.substring(6, 8) + kml.substring(4, 6) + kml.substring(2, 4)
  }
  const hexToKml = (hex) => {
    const r = hex.substring(1, 3)
    const g = hex.substring(3, 5)
    const b = hex.substring(5, 7)
    return 'ff' + b + g + r
  }
  return (
    <div className="field">
      <label>{label}</label>
      <input
        type="color"
        value={value ? kmlToHex(value) : '#3388ff'}
        onChange={e => onChange(hexToKml(e.target.value))}
        style={{ width: 40, height: 28, padding: 0, border: '1px solid #ccc', borderRadius: 4, cursor: 'pointer' }}
      />
      {value && <button
        className="tree-remove"
        style={{ opacity: 1, fontSize: 13 }}
        onClick={() => onChange(null)}
        title="Clear color"
      >{'\u00D7'}</button>}
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
      <ColorField label="Color" value={point.color} onChange={v => update('color', v)} />
      <LibraryModal
        open={libraryOpen}
        onClose={() => setLibraryOpen(false)}
        onSelect={key => {
          if (typeof key === 'object' && key.type === 'mappoint') {
            onChange({ ...point, kml: '', name: key.name, lat: key.lat, lng: key.lng })
          } else {
            update('kml', key)
          }
          setLibraryOpen(false)
        }}
        choices={useContext(ChoicesContext)}
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
      <ColorField label="Color" value={data.color} onChange={v => update({ ...data, color: v })} />
      <Field label="Alpha" value={data.alpha} onChange={v => update({ ...data, alpha: v })} type="number" />
      <LibraryModal
        open={libraryOpen}
        onClose={() => setLibraryOpen(false)}
        onSelect={key => { update({ ...data, path: key }); setLibraryOpen(false) }}
        choices={useContext(ChoicesContext)}
      />
    </>
  )
}

// Module-level drag state so all ChoiceLists can coordinate
let dragState = { choice: null, removeFromSource: null }
// Top-level setChoices ref for atomic drag-and-drop
let globalSetChoices = null
// Global hidden set using object references (works at any depth)
const globalHiddenSet = new Set()
let globalSetHiddenVersion = () => {}
// Route info from last visualization
let globalRouteInfo = {}
// Callback to center map on a choice
let globalCenterOnChoice = null

function choiceLabel(choice) {
  const type = getType(choice)
  const data = getData(choice)
  switch (type) {
    case 'Folder': return data.name || 'Folder'
    case 'Point': return data.name || data.kml || 'Point'
    case 'ConcentricCircles': return data.name || 'Circles'
    case 'UnionCircles': return data.name || 'Union'
    case 'Segments': return data.name || 'Segments'
    case 'RawKml': return data.path?.replace(/.*\//, '').replace('.kml', '') || 'RawKml'
    case 'TriangleBisect': return `${data.point1?.name || '?'} / ${data.point2?.name || '?'}`
    case 'Route': {
      const label = data.name || `${data.from?.name || '?'} → ${data.to?.name || '?'}`
      const info = globalRouteInfo[data.name]
      return info ? `${label} (${info})` : label
    }
    case 'MapPoint': return data.name || `${data.lat?.toFixed(4)}, ${data.lng?.toFixed(4)}`
    default: return type
  }
}

function ChoiceEditor({ choice, onChange, onRemove, visible, onToggleVisible, depth = 0 }) {
  const [expanded, setExpanded] = useState(false)
  const [isDragOver, setIsDragOver] = useState(false)
  const [addMenuOpen, setAddMenuOpen] = useState(false)
  const [menuPos, setMenuPos] = useState({ top: 0, left: 0 })
  const type = getType(choice)
  const data = getData(choice)
  const update = (newData) => onChange({ [type]: newData })
  const isFolder = type === 'Folder'
  const hasChildren = isFolder && data.choices.length > 0

  const handleDragOver = (e) => {
    if (!isFolder) return
    if (dragState.choice === choice) return
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(true)
  }
  const handleDragLeave = (e) => {
    if (!isFolder) return
    e.stopPropagation()
    setIsDragOver(false)
  }
  const handleDrop = (e) => {
    if (!isFolder) return
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(false)
    if (!dragState.choice || dragState.choice === choice) return
    const dropped = dragState.choice
    dragState = { choice: null, removeFromSource: null }
    // Atomic update: remove from source + add to this folder in one pass
    if (globalSetChoices) {
      globalSetChoices(prev => {
        const removeAndAdd = (list) => {
          // First remove the dragged item from wherever it is
          let filtered = list.filter(c => c !== dropped)
          // Then add it to matching folders
          return filtered.map(c => {
            if (c === choice) {
              const t = getType(c)
              const d = getData(c)
              return { [t]: { ...d, choices: [...d.choices, dropped] } }
            }
            if (getType(c) === 'Folder') {
              const t = getType(c)
              const d = getData(c)
              return { [t]: { ...d, choices: removeAndAdd(d.choices) } }
            }
            return c
          })
        }
        return removeAndAdd(prev)
      })
    }
  }

  return (
    <div
      className={`tree-item ${isFolder ? 'tree-folder' : ''} ${!visible ? 'tree-hidden' : ''} ${isDragOver ? 'tree-drag-over' : ''}`}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div
        className="tree-row"
        style={{ paddingLeft: depth * 16 + 6 }}
        draggable
        onDragStart={e => {
          e.stopPropagation()
          dragState = { choice, removeFromSource: onRemove }
          e.dataTransfer.effectAllowed = 'move'
        }}
        onDragEnd={() => { dragState = { choice: null, removeFromSource: null } }}
      >
        <span className="tree-toggle" onClick={e => { e.stopPropagation(); setExpanded(!expanded) }}>
          {isFolder ? (expanded ? '\u25BE' : '\u25B8') : ' '}
        </span>
        <button className={`tree-eye ${visible ? '' : 'tree-eye-off'}`} onClick={e => { e.stopPropagation(); onToggleVisible() }} title={visible ? 'Hide' : 'Show'}>
          {visible ? '\uD83D\uDC41' : '\u2014'}
        </button>
        <button className="tree-center" onClick={e => { e.stopPropagation(); if (globalCenterOnChoice) globalCenterOnChoice(choice) }} title="Center on map">
          {'\u2316'}
        </button>
        {data.color && <span className="tree-color-dot" style={{ background: kmlColorToHex(data.color) }} />}
        <span className="tree-label" onClick={() => setExpanded(!expanded)}>{choiceLabel(choice)}</span>
        <span className="tree-type-hint">{type}</span>
        {isFolder && <div className="tree-add-wrap" onClick={e => e.stopPropagation()}>
          <button className="tree-add-btn" onClick={e => {
            if (!addMenuOpen) {
              const rect = e.currentTarget.getBoundingClientRect()
              setMenuPos({ top: rect.bottom + 2, left: rect.right })
            }
            setAddMenuOpen(!addMenuOpen)
          }} title="Add item">+</button>
          {addMenuOpen && <div className="tree-add-menu" style={{ position: 'fixed', top: menuPos.top, left: menuPos.left, transform: 'translateX(-100%)' }}>
            {CHOICE_TYPES.map(t => (
              <div key={t} className="tree-add-menu-item" onClick={() => {
                update({ ...data, choices: [...data.choices, makeDefault(t)] })
                setAddMenuOpen(false)
                setExpanded(true)
              }}>{t}</div>
            ))}
          </div>}
        </div>}
        <button className="tree-remove" onClick={e => { e.stopPropagation(); onRemove() }} title="Remove">{'\u00D7'}</button>
      </div>
      {expanded && isFolder && (
        <div className="tree-children">
          <div className="tree-detail" style={{ paddingLeft: depth * 16 + 28 }}>
            <Field label="Name" value={data.name} onChange={v => update({ ...data, name: v })} />
          </div>
          {data.choices.map((ch, i) => (
            <ChoiceEditor
              key={i}
              choice={ch}
              onChange={val => {
                const c = [...data.choices]; c[i] = val
                update({ ...data, choices: c })
              }}
              onRemove={() => {
                globalHiddenSet.delete(ch)
                update({ ...data, choices: data.choices.filter((_, j) => j !== i) })
              }}
              visible={visible && !globalHiddenSet.has(ch)}
              onToggleVisible={() => {
                if (globalHiddenSet.has(ch)) globalHiddenSet.delete(ch)
                else globalHiddenSet.add(ch)
                globalSetHiddenVersion(v => v + 1)
              }}
              depth={depth + 1}
            />
          ))}
        </div>
      )}
      {expanded && !isFolder && (
        <div className="tree-detail" style={{ paddingLeft: depth * 16 + 28 }}>
          {type === 'ConcentricCircles' && <>
            <Field label="Name" value={data.name} onChange={v => update({ ...data, name: v })} />
            <label className="sub-label">Center</label>
            <PointEditor point={data.center} onChange={v => update({ ...data, center: v })} />
            <RadiiEditor radii={data.v_radius} onChange={v => update({ ...data, v_radius: v })} />
            <Field label="Circle on top" value={data.circle_on_top} onChange={v => update({ ...data, circle_on_top: v })} type="checkbox" />
            <ColorsEditor colors={data.colors} onChange={v => update({ ...data, colors: v })} />
          </>}
          {type === 'Point' && <PointEditor point={data} onChange={v => update(v)} />}
          {type === 'UnionCircles' && <>
            <Field label="Name" value={data.name} onChange={v => update({ ...data, name: v })} />
            <Field label="Radius" value={data.radius} onChange={v => update({ ...data, radius: v })} type="number" />
            <Field label="Circle on top" value={data.circle_on_top} onChange={v => update({ ...data, circle_on_top: v })} type="checkbox" />
            <ColorField label="Color" value={data.color} onChange={v => update({ ...data, color: v })} />
            <label className="sub-label">Centers</label>
            {data.centers.map((c, i) => (
              <div key={i} className="center-row">
                <PointEditor point={c} onChange={v => {
                  const centers = [...data.centers]; centers[i] = v
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
            <ColorField label="Color" value={data.color} onChange={v => update({ ...data, color: v })} />
            <NeighboursEditor neighbours={data.neighbours} onChange={v => update({ ...data, neighbours: v })} />
          </>}
          {type === 'TriangleBisect' && <>
            <label className="sub-label">Point 1</label>
            <PointEditor point={data.point1} onChange={v => update({ ...data, point1: v })} />
            <label className="sub-label">Point 2</label>
            <PointEditor point={data.point2} onChange={v => update({ ...data, point2: v })} />
            <Field label="Radius factor" value={data.radius_factor} onChange={v => update({ ...data, radius_factor: v })} type="number" />
          </>}
          {type === 'Route' && <>
            <Field label="Name" value={data.name} onChange={v => update({ ...data, name: v })} />
            <label className="sub-label">From</label>
            <PointEditor point={data.from} onChange={v => update({ ...data, from: v })} />
            <label className="sub-label">To</label>
            <PointEditor point={data.to} onChange={v => update({ ...data, to: v })} />
            <ColorField label="Color" value={data.color} onChange={v => update({ ...data, color: v })} />
            <div className="field">
              <label>Mode</label>
              <select value={data.mode || 'foot'} onChange={e => update({ ...data, mode: e.target.value })}>
                <option value="foot">Walking</option>
                <option value="bike">Cycling</option>
                <option value="car">Driving</option>
              </select>
            </div>
          </>}
          {type === 'RawKml' && <RawKmlEditor data={data} update={update} />}
          {type === 'MapPoint' && <>
            <Field label="Name" value={data.name} onChange={v => update({ ...data, name: v })} />
            <Field label="Lat" value={data.lat} onChange={v => update({ ...data, lat: v })} type="number" />
            <Field label="Lng" value={data.lng} onChange={v => update({ ...data, lng: v })} type="number" />
            <ColorField label="Color" value={data.color} onChange={v => update({ ...data, color: v })} />
            <Field label="Icon URL" value={data.icon} onChange={v => update({ ...data, icon: v || null })} placeholder="(optional)" />
          </>}
        </div>
      )}
    </div>
  )
}

function ChoiceList({ choices, onChange }) {
  return (
    <div className="tree-list">
      {choices.map((ch, i) => (
        <ChoiceEditor
          key={i}
          choice={ch}
          onChange={val => {
            const c = [...choices]; c[i] = val; onChange(c)
          }}
          onRemove={() => {
            globalHiddenSet.delete(ch)
            onChange(choices.filter((_, j) => j !== i))
          }}
          visible={!globalHiddenSet.has(ch)}
          onToggleVisible={() => {
            if (globalHiddenSet.has(ch)) globalHiddenSet.delete(ch)
            else globalHiddenSet.add(ch)
            globalSetHiddenVersion(v => v + 1)
          }}
        />
      ))}
      <div className="tree-add-folder" onClick={() => onChange([...choices, makeDefault('Folder')])}>+ Folder</div>
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

function PointCreator({ latLng, onCancel, onCreate }) {
  const [name, setName] = useState('')
  const [color, setColor] = useState('#3388ff')
  const [icon, setIcon] = useState('')

  const handleCreate = () => {
    const hexToKml = (hex) => {
      const r = hex.substring(1, 3)
      const g = hex.substring(3, 5)
      const b = hex.substring(5, 7)
      return 'ff' + b + g + r
    }
    onCreate({
      lat: latLng.lat,
      lng: latLng.lng,
      name: name || `Point ${latLng.lat.toFixed(4)}, ${latLng.lng.toFixed(4)}`,
      color: hexToKml(color),
      icon: icon || null,
    })
  }

  return (
    <div className="point-creator">
      <div className="point-creator-title">New point at {latLng.lat.toFixed(5)}, {latLng.lng.toFixed(5)}</div>
      <div className="field">
        <label>Name</label>
        <input type="text" value={name} onChange={e => setName(e.target.value)} placeholder="Point name" autoFocus
          onKeyDown={e => { if (e.key === 'Enter') handleCreate() }} />
      </div>
      <div className="field">
        <label>Color</label>
        <input type="color" value={color} onChange={e => setColor(e.target.value)} style={{ width: 40, height: 28, padding: 0 }} />
      </div>
      <div className="field">
        <label>Icon URL</label>
        <input type="text" value={icon} onChange={e => setIcon(e.target.value)} placeholder="(optional)" />
      </div>
      <div className="point-creator-actions">
        <button className="prompt-btn" onClick={handleCreate}>Create</button>
        <button className="export-btn" onClick={onCancel}>Cancel</button>
      </div>
    </div>
  )
}

export default function ConfigPage() {
  const navigate = useNavigate()
  const [choices, setChoices] = useState([])
  globalSetChoices = setChoices
  const [hiddenVersion, setHiddenVersion] = useState(0)
  globalSetHiddenVersion = setHiddenVersion
  const [error, setError] = useState(null)
  const [loading, setLoading] = useState(false)
  const [prompt, setPrompt] = useState('')
  const [prompting, setPrompting] = useState(false)
  const [editorVisible, setEditorVisible] = useState(true)
  const [model, setModel] = useState(localStorage.getItem('llm-model') || 'claude-sonnet')
  const [clickMode, setClickMode] = useState(false)
  const [retryCountdown, setRetryCountdown] = useState(0)
  const retryTimer = useRef(null)
  const [pendingLatLng, setPendingLatLng] = useState(null)
  const [routeInfo, setRouteInfo] = useState({}) // route name -> "1.2 km, 15 min"
  const mapRef = useRef(null)
  const mapInstance = useRef(null)
  const currentLayer = useRef(null)
  const mapPointsLayer = useRef(null)

  useEffect(() => {
    if (!mapInstance.current && mapRef.current) {
      mapInstance.current = L.map(mapRef.current).setView([48.92, 2.19], 13)
      L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
        attribution: '&copy; OpenStreetMap contributors'
      }).addTo(mapInstance.current)
      mapPointsLayer.current = L.layerGroup().addTo(mapInstance.current)
      mapInstance.current.on('click', (e) => {
        if (clickModeRef.current) {
          setPendingLatLng({ lat: e.latlng.lat, lng: e.latlng.lng })
        }
      })
    }
    return () => {
      if (mapInstance.current) {
        mapInstance.current.remove()
        mapInstance.current = null
      }
    }
  }, [])

  const clickModeRef = useRef(false)
  useEffect(() => { clickModeRef.current = clickMode }, [clickMode])

  function visibleChoices(choiceList) {
    return (choiceList || []).filter(ch => !globalHiddenSet.has(ch)).map(ch => {
      if (getType(ch) === 'Folder') {
        const d = getData(ch)
        return { Folder: { ...d, choices: visibleChoices(d.choices) } }
      }
      return ch
    })
  }

  function collectMapPoints(choiceList) {
    const pts = []
    ;(choiceList || []).forEach(ch => {
      if (globalHiddenSet.has(ch)) return
      const type = getType(ch)
      if (type === 'MapPoint') pts.push(getData(ch))
      if (type === 'Folder') {
        collectMapPoints(getData(ch).choices).forEach(p => pts.push(p))
      }
    })
    return pts
  }

  function renderMapPoints() {
    if (!mapPointsLayer.current) return
    mapPointsLayer.current.clearLayers()
    const pts = collectMapPoints(choices)
    pts.forEach(pt => {
      const color = pt.color ? kmlColorToHex(pt.color) : '#3388ff'
      const markerOpts = {}
      if (pt.icon) {
        markerOpts.icon = L.icon({ iconUrl: pt.icon, iconSize: [24, 24], iconAnchor: [12, 12], popupAnchor: [0, -12] })
      } else {
        markerOpts.icon = L.divIcon({
          className: 'map-point-marker',
          html: `<div style="width:14px;height:14px;border-radius:50%;background:${color};border:2px solid white;box-shadow:0 1px 3px rgba(0,0,0,0.4)"></div>`,
          iconSize: [14, 14],
          iconAnchor: [7, 7],
        })
      }
      L.marker([pt.lat, pt.lng], markerOpts)
        .bindPopup(`<b>${pt.name || 'Point'}</b><br>${pt.lat.toFixed(5)}, ${pt.lng.toFixed(5)}`)
        .addTo(mapPointsLayer.current)
    })
  }

  // Center on choice: collect bounds from rendered layers matching the choice
  globalCenterOnChoice = useCallback((choice) => {
    if (!mapInstance.current) return
    const type = getType(choice)
    const data = getData(choice)
    const bounds = L.latLngBounds([])

    // Direct coordinates
    const addPoint = (pt) => {
      if (pt && pt.lat != null && pt.lng != null) bounds.extend([pt.lat, pt.lng])
    }

    if (type === 'MapPoint') { addPoint(data); }
    else if (type === 'Point') { addPoint(data); }
    else if (type === 'ConcentricCircles') { addPoint(data.center); }
    else if (type === 'UnionCircles') { (data.centers || []).forEach(addPoint); }
    else if (type === 'Route') { addPoint(data.from); addPoint(data.to); }

    // Search rendered KML layers by matching feature names
    if (currentLayer.current) {
      const label = choiceLabel(choice)
      const name = data.name || ''
      currentLayer.current.eachLayer(l => {
        const fn = l.feature?.properties?.name || ''
        if (!fn) return
        const match = fn === name || fn === label || fn.startsWith(name + ' (')
        if (match) {
          if (l.getBounds) bounds.extend(l.getBounds())
          else if (l.getLatLng) bounds.extend(l.getLatLng())
        }
      })
    }

    // For Folder: recurse into children
    if (type === 'Folder' && data.choices) {
      data.choices.forEach(ch => {
        const ct = getType(ch)
        const cd = getData(ch)
        if (cd.lat != null && cd.lng != null) bounds.extend([cd.lat, cd.lng])
        if (ct === 'ConcentricCircles' && cd.center?.lat != null) bounds.extend([cd.center.lat, cd.center.lng])
        if (ct === 'Route') { addPoint(cd.from); addPoint(cd.to); }
      })
      // Also match folder name in KML
      if (currentLayer.current) {
        currentLayer.current.eachLayer(l => {
          if (l.feature?.properties?.name && data.choices.some(ch => {
            const cd = getData(ch)
            return l.feature.properties.name === (cd.name || '')
          })) {
            if (l.getBounds) bounds.extend(l.getBounds())
            else if (l.getLatLng) bounds.extend(l.getLatLng())
          }
        })
      }
    }

    // For RawKml: match all placemarks from the rendered layer (they share the style)
    if (type === 'RawKml' && currentLayer.current) {
      const pathId = 'rawkml_color_' + (data.path || '').replace(/[/.\-]/g, '_')
      currentLayer.current.eachLayer(l => {
        const su = l.feature?.properties?.styleUrl || ''
        if (su.includes(pathId)) {
          if (l.getBounds) bounds.extend(l.getBounds())
          else if (l.getLatLng) bounds.extend(l.getLatLng())
        }
      })
    }

    if (bounds.isValid()) {
      mapInstance.current.fitBounds(bounds.pad(0.1))
    }
  }, [])

  useEffect(() => { renderMapPoints() }, [choices, hiddenVersion])

  // Re-visualize when visibility toggles change (only if already visualized)
  const prevHiddenVersion = useRef(0)
  useEffect(() => {
    if (prevHiddenVersion.current !== hiddenVersion && currentLayer.current) {
      prevHiddenVersion.current = hiddenVersion
      visualize()
    }
  }, [hiddenVersion])

  async function visualize(overrideChoices) {
    setError(null)
    setLoading(true)
    const filterMapPoints = (list) => list.filter(c => getType(c) !== 'MapPoint').map(c => {
      if (getType(c) === 'Folder') {
        const d = getData(c)
        return { Folder: { ...d, choices: d.choices.filter(ch => getType(ch) !== 'MapPoint') } }
      }
      return c
    })
    const toSend = filterMapPoints(overrideChoices || visibleChoices(choices))
    try {
      const resp = await fetch(API_BASE + '/generate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ choices: toSend }),
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
          const polyColorRaw = polyEl ? polyEl.textContent.trim() : null
          styles[id] = {
            lineColor: lineEl ? kmlColorToHex(lineEl.textContent.trim()) : null,
            polyColor: polyColorRaw ? kmlColorToHex(polyColorRaw) : null,
            polyAlpha: polyColorRaw ? parseInt(polyColorRaw.substring(0, 2), 16) / 255 : null,
            iconHref: iconHref ? iconHref.textContent.trim() : null,
            iconColor: iconColor ? iconColor.textContent.trim() : null,
          }
        }
      })

      const layer = omnivore.kml.parse(kmlText)
      const newRouteInfo = {}
      const applyStyles = (parentLayer) => {
        parentLayer.eachLayer(l => {
          // Recurse into nested layer groups (KML Folders)
          if (l.eachLayer && !l.feature) { applyStyles(l); return }
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
              if (l.setStyle) {
                const styleObj = {}
                if (style.lineColor) {
                  styleObj.color = style.lineColor
                  styleObj.weight = 3
                }
                if (style.polyColor) {
                  styleObj.fillColor = style.polyColor
                  styleObj.fillOpacity = style.polyAlpha ?? 0.3
                  if (!style.lineColor) {
                    styleObj.color = style.polyColor
                    styleObj.weight = 2
                  }
                }
                if (Object.keys(styleObj).length) l.setStyle(styleObj)
              }
            }
          }
          if (l.feature?.properties?.name) {
            const name = l.feature.properties.name
            l.bindPopup('<b>' + name + '</b>')
            // Extract route info like "Route Name (1.2 km, 15 min)"
            const routeMatch = name.match(/^(.*?)\s*\((\d+(?:\.\d+)?\s*(?:km|m),\s*\d+\s*min)\)$/)
            if (routeMatch) {
              const routeName = routeMatch[1].trim()
              newRouteInfo[routeName] = routeMatch[2]
              if (l.bindTooltip) {
                l.bindTooltip(routeMatch[2], { permanent: true, direction: 'center', className: 'route-tooltip' })
              }
            }
          }
        })
      }
      applyStyles(layer)
      globalRouteInfo = newRouteInfo
      setRouteInfo(newRouteInfo)
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
        body: JSON.stringify({ prompt: prompt.trim(), model }),
      })
      if (!resp.ok) {
        const msg = await resp.text()
        throw new Error(msg || resp.statusText)
      }
      const data = await resp.json()
      if (data.error) {
        setError(data.error)
      }
      if (data.choices && data.choices.length > 0) {
        const merged = [...choices, ...data.choices]
        setChoices(merged)
        setTimeout(() => visualize(merged), 100)
      } else if (!data.error) {
        setError('The model returned no results for this prompt.')
      }
    } catch (e) {
      console.error('Prompt error:', e)
      const msg = e.message
      setError(msg)
      // Parse retryDelay from rate limit errors
      const retryMatch = msg.match(/retryDelay"?\s*:\s*"?(\d+)/i) || msg.match(/retry in (\d+)/i)
      if (retryMatch) {
        let secs = parseInt(retryMatch[1], 10)
        if (retryTimer.current) clearInterval(retryTimer.current)
        setRetryCountdown(secs)
        retryTimer.current = setInterval(() => {
          secs--
          if (secs <= 0) {
            clearInterval(retryTimer.current)
            retryTimer.current = null
            setRetryCountdown(0)
            setError(null)
          } else {
            setRetryCountdown(secs)
          }
        }, 1000)
      }
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
          <button className="visualize-btn" onClick={() => visualize()} disabled={loading}>
            {loading ? 'Generating...' : 'Visualize'}
          </button>
          <button className={`pin-btn ${clickMode ? 'pin-btn-active' : ''}`} onClick={() => { setClickMode(!clickMode); setPendingLatLng(null) }} title={clickMode ? 'Cancel pin mode' : 'Pin a point on map'}>
            {'\uD83D\uDCCD'}
          </button>
          <button className="toggle-editor-btn" onClick={() => { setEditorVisible(!editorVisible); setTimeout(() => mapInstance.current?.invalidateSize(), 50) }} title={editorVisible ? 'Hide editor' : 'Show editor'}>
            {editorVisible ? '\u25C0' : '\u25B6'}
          </button>
        </div>
      </div>
      <div className="config-split">
        {editorVisible && <div className="config-editor">
          <div className="prompt-bar">
            <textarea
              className="prompt-input"
              value={prompt}
              onChange={e => setPrompt(e.target.value)}
              placeholder="Describe what to show on the map, e.g.: draw a blue circle of radius 500m around each station of RER A"
              onKeyDown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); generateFromPrompt() } }}
              rows={2}
            />
            <div className="prompt-controls">
              <select className="model-select" value={model} onChange={e => { setModel(e.target.value); localStorage.setItem('llm-model', e.target.value) }}>
                <option value="claude-sonnet">Claude Sonnet</option>
                <option value="claude-haiku">Claude Haiku</option>
                <option value="gpt-4o">GPT-4o</option>
                <option value="gpt-4o-mini">GPT-4o mini</option>
                <option value="gemini-2.5-flash">Gemini 2.5 Flash</option>
                <option value="gemini-2.5-pro">Gemini 2.5 Pro</option>
              </select>
              <button className="prompt-btn" onClick={generateFromPrompt} disabled={prompting || retryCountdown > 0}>
                {prompting ? 'Thinking...' : retryCountdown > 0 ? `Retry in ${retryCountdown}s` : 'Generate'}
              </button>
            </div>
          </div>
          <ChoicesContext.Provider value={choices}>
            <ChoiceList choices={choices} onChange={setChoices} />
          </ChoicesContext.Provider>
        </div>}
        <div className="config-map">
          {error && <div className="error-bar">{error}</div>}
          {clickMode && !pendingLatLng && <div className="map-click-hint">Click on the map to place a point</div>}
          <div className={`map-container ${clickMode ? 'map-crosshair' : ''}`} ref={mapRef} />
          {pendingLatLng && <PointCreator latLng={pendingLatLng} onCancel={() => setPendingLatLng(null)} onCreate={(pt) => {
            setChoices([...choices, { MapPoint: pt }])
            setPendingLatLng(null)
            setClickMode(false)
          }} />}
        </div>
      </div>
    </div>
  )
}

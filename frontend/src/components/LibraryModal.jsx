import { useState, useEffect } from 'react'
import './LibraryModal.css'

const API_BASE = import.meta.env.VITE_API_BASE || '/api'

function formatSize(bytes) {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

function buildTree(files) {
  const root = { folders: {}, files: [] }
  for (const f of files) {
    const parts = f.key.split('/')
    let node = root
    for (let i = 0; i < parts.length - 1; i++) {
      const dir = parts[i]
      if (!node.folders[dir]) node.folders[dir] = { folders: {}, files: [] }
      node = node.folders[dir]
    }
    node.files.push({ key: f.key, name: parts[parts.length - 1], size: f.size })
  }
  return root
}

function FolderView({ node, path, onSelect, onClose }) {
  const [expanded, setExpanded] = useState({})

  const toggle = (name) => setExpanded(prev => ({ ...prev, [name]: !prev[name] }))

  const folderNames = Object.keys(node.folders).sort()

  return (
    <div className="library-tree">
      {folderNames.map(name => (
        <div key={name} className="library-tree-folder">
          <button className="library-folder-btn" onClick={() => toggle(name)}>
            <span className="folder-arrow">{expanded[name] ? '▼' : '▶'}</span>
            <span className="folder-icon">📁</span>
            <span className="folder-name">{name}</span>
            <span className="folder-count">
              {countFiles(node.folders[name])}
            </span>
          </button>
          {expanded[name] && (
            <div className="library-tree-children">
              <FolderView
                node={node.folders[name]}
                path={path ? path + '/' + name : name}
                onSelect={onSelect}
                onClose={onClose}
              />
            </div>
          )}
        </div>
      ))}
      {node.files.map(f => (
        <button
          key={f.key}
          className="library-file-btn"
          onClick={() => { onSelect(f.key); onClose() }}
        >
          <span className="file-name">{f.name}</span>
          <span className="file-size">{formatSize(f.size)}</span>
        </button>
      ))}
    </div>
  )
}

function countFiles(node) {
  let count = node.files.length
  for (const sub of Object.values(node.folders)) {
    count += countFiles(sub)
  }
  return count
}

function collectTreeItems(choices) {
  const paths = new Set()
  const mapPoints = []
  for (const ch of (choices || [])) {
    const type = Object.keys(ch)[0]
    const data = Object.values(ch)[0]
    if (type === 'MapPoint') {
      mapPoints.push(data)
    }
    if (type === 'Point' && data.kml) paths.add(data.kml)
    if (type === 'ConcentricCircles' && data.center?.kml) paths.add(data.center.kml)
    if (type === 'Segments' && data.kml) paths.add(data.kml)
    if (type === 'RawKml' && data.path) paths.add(data.path)
    if (type === 'Route') {
      if (data.from?.kml) paths.add(data.from.kml)
      if (data.to?.kml) paths.add(data.to.kml)
    }
    if (type === 'TriangleBisect') {
      if (data.point1?.kml) paths.add(data.point1.kml)
      if (data.point2?.kml) paths.add(data.point2.kml)
    }
    if (type === 'UnionCircles') {
      for (const c of (data.centers || [])) { if (c.kml) paths.add(c.kml) }
    }
    if (type === 'Folder') {
      const sub = collectTreeItems(data.choices)
      for (const p of sub.paths) paths.add(p)
      mapPoints.push(...sub.mapPoints)
    }
  }
  return { paths, mapPoints }
}

export default function LibraryModal({ open, onClose, onSelect, choices }) {
  const [files, setFiles] = useState([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState(null)

  useEffect(() => {
    if (open) {
      setLoading(true)
      setError(null)
      fetch(API_BASE + '/list')
        .then(r => {
          if (!r.ok) throw new Error('Failed to fetch file list')
          return r.json()
        })
        .then(data => {
          setFiles(data)
          setLoading(false)
        })
        .catch(e => {
          setError(e.message)
          setLoading(false)
        })
    }
  }, [open])

  if (!open) return null

  const tree = buildTree(files)
  const { paths: treePathSet, mapPoints } = collectTreeItems(choices)
  const treePaths = [...treePathSet].filter(Boolean).sort()

  return (
    <div className="library-overlay" onClick={onClose}>
      <div className="library-modal" onClick={e => e.stopPropagation()}>
        <div className="library-header">
          <h2>KML Library</h2>
          <button className="library-close" onClick={onClose}>x</button>
        </div>
        <div className="library-body">
          {mapPoints.length > 0 && (
            <div className="library-tree-section">
              <div className="library-section-title">Map points</div>
              {mapPoints.map((pt, i) => (
                <button key={i} className="library-file-btn library-file-mappoint" onClick={() => {
                  onSelect({ type: 'mappoint', name: pt.name, lat: pt.lat, lng: pt.lng })
                  onClose()
                }}>
                  <span className="file-name">{pt.name || `${pt.lat?.toFixed(4)}, ${pt.lng?.toFixed(4)}`}</span>
                  <span className="file-size">{pt.lat?.toFixed(4)}, {pt.lng?.toFixed(4)}</span>
                </button>
              ))}
            </div>
          )}
          {treePaths.length > 0 && (
            <div className="library-tree-section">
              <div className="library-section-title">Used in current tree</div>
              {treePaths.map(p => (
                <button key={p} className="library-file-btn library-file-used" onClick={() => { onSelect(p); onClose() }}>
                  <span className="file-name">{p}</span>
                </button>
              ))}
            </div>
          )}
          {loading && <p className="library-status">Loading...</p>}
          {error && <p className="library-error">{error}</p>}
          {!loading && !error && files.length === 0 && <p className="library-status">No KML files found</p>}
          {!loading && !error && files.length > 0 && (
            <>
              {treePaths.length > 0 && <div className="library-section-title">S3 Library</div>}
              <FolderView node={tree} path="" onSelect={onSelect} onClose={onClose} />
            </>
          )}
        </div>
      </div>
    </div>
  )
}

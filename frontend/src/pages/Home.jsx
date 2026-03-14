import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import LibraryModal from '../components/LibraryModal'
import './Home.css'

export default function Home() {
  const navigate = useNavigate()
  const [libraryOpen, setLibraryOpen] = useState(false)

  return (
    <div className="home">
      <h1>KML Utils</h1>
      <p className="home-description">Show configurable items on a map</p>
      <div className="home-buttons">
        <button className="home-btn configure" onClick={() => navigate('/configure')}>
          Configure
        </button>
        <button className="home-btn map" onClick={() => navigate('/map')}>
          Map
        </button>
        <button className="home-btn library" onClick={() => setLibraryOpen(true)}>
          Library
        </button>
      </div>
      <LibraryModal
        open={libraryOpen}
        onClose={() => setLibraryOpen(false)}
        onSelect={key => navigate('/map?kml=' + encodeURIComponent(key))}
      />
    </div>
  )
}

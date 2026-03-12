import { useNavigate } from 'react-router-dom'
import './Home.css'

export default function Home() {
  const navigate = useNavigate()

  return (
    <div className="home">
      <h1>KML Utils</h1>
      <div className="home-buttons">
        <button className="home-btn configure" onClick={() => navigate('/configure')}>
          Configure
        </button>
        <button className="home-btn map" onClick={() => navigate('/map')}>
          Map
        </button>
      </div>
    </div>
  )
}

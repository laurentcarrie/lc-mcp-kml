import { useNavigate } from 'react-router-dom'
import './ConfigPage.css'

export default function ConfigPage() {
  const navigate = useNavigate()

  return (
    <div className="config-page">
      <div className="config-bar">
        <button className="back-btn" onClick={() => navigate('/')}>Home</button>
        <h2>Configure</h2>
      </div>
      <div className="config-content">
        <p>Configuration options will appear here.</p>
      </div>
    </div>
  )
}

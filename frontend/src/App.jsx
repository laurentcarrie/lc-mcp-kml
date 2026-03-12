import { Routes, Route } from 'react-router-dom'
import Home from './pages/Home.jsx'
import MapPage from './pages/MapPage.jsx'
import ConfigPage from './pages/ConfigPage.jsx'

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Home />} />
      <Route path="/map" element={<MapPage />} />
      <Route path="/configure" element={<ConfigPage />} />
    </Routes>
  )
}

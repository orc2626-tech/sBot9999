import { Routes, Route } from 'react-router-dom'
import Layout from './components/Layout'
import KeyboardShortcuts from './components/KeyboardShortcuts'
import Overview from './pages/Overview'
import Live from './pages/Live'
import Strategy from './pages/Strategy'
import Insurance from './pages/Insurance'
import Risk from './pages/Risk'
import Positions from './pages/Positions'
import Execution from './pages/Execution'
import AI from './pages/AI'
import Diagnostics from './pages/Diagnostics'
import Control from './pages/Control'
import Ladder from './pages/Ladder'

export default function App() {
  return (
    <>
      <KeyboardShortcuts />
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<Overview />} />
          <Route path="/live" element={<Live />} />
          <Route path="/strategy" element={<Strategy />} />
          <Route path="/insurance" element={<Insurance />} />
          <Route path="/risk" element={<Risk />} />
          <Route path="/positions" element={<Positions />} />
          <Route path="/execution" element={<Execution />} />
          <Route path="/ai" element={<AI />} />
          <Route path="/diagnostics" element={<Diagnostics />} />
          <Route path="/ladder" element={<Ladder />} />
          <Route path="/control" element={<Control />} />
        </Route>
      </Routes>
    </>
  )
}

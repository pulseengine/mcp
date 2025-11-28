import React from 'react'
import ReactDOM from 'react-dom/client'
import { GreetingUI } from './GreetingUI'
import './index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <GreetingUI />
  </React.StrictMode>,
)

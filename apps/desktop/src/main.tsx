import React from 'react';
import ReactDOM from 'react-dom/client';
import { initTauriTestApi } from '@/lib/backend/shell';
import App from './App';
import './index.css';

initTauriTestApi();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

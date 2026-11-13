import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router';
import './index.css';
import { AppLayout } from './AppLayout';
import { AuthPage } from './AuthPage';
import { restoreSession, useSession } from './session';

void restoreSession();

function RequireAuth({ children }: { children: React.ReactNode }) {
  const status = useSession((s) => s.status);
  if (status === 'loading') return <div className="grid h-dvh place-items-center text-zinc-400">Загрузка…</div>;
  return status === 'authed' ? children : <Navigate to="/login" replace />;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<AuthPage mode="login" />} />
        <Route path="/register" element={<AuthPage mode="register" />} />
        <Route path="/channels/:serverId?/:channelId?" element={<RequireAuth><AppLayout /></RequireAuth>} />
        <Route path="*" element={<Navigate to="/channels" replace />} />
      </Routes>
    </BrowserRouter>
  </StrictMode>,
);

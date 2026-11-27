import * as React from 'react';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router';
import './index.css';
import { AppLayout } from './AppLayout';
import { AuthPage } from './AuthPage';
import { restoreSession, useSession } from './session';

class ErrorBoundary extends React.Component<{ children: React.ReactNode }, { hasError: boolean }> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error): { hasError: boolean } {
    console.error('React error boundary caught:', error);
    return { hasError: true };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('Uncaught error:', error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="fixed inset-0 flex items-center justify-center bg-zinc-900/90 backdrop-blur-sm p-6 text-center">
          <div className="max-w-2xl bg-zinc-800 rounded-xl p-8 border border-zinc-700 shadow-2xl">
            <h1 className="text-3xl font-bold text-red-400 mb-4">Ошибка приложения</h1>
            <p className="text-zinc-300 mb-6">Произошла непредвиденная ошибка. Пожалуйста, перезагрузите страницу.</p>
            <button
              onClick={() => window.location.reload()}
              className="btn-primary text-lg py-3 px-6 rounded-lg font-medium transition-colors"
            >
              Перезагрузить страницу
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

// Применяем сохранённую тему до первого рендера, чтобы не было «вспышки» и светлая тема работала сразу.
const initialTheme = localStorage.getItem('nexus-theme') ?? 'auto';
const initialDark = initialTheme === 'dark' || (initialTheme === 'auto' && window.matchMedia('(prefers-color-scheme: dark)').matches);
document.documentElement.classList.toggle('nexus-dark', initialDark);
document.documentElement.classList.toggle('nexus-light', !initialDark);

void restoreSession();

// PWA регистрация с виртуального модуля vite-plugin-pwa
import { registerSW } from 'virtual:pwa-register';

registerSW({
  immediate: true,
  onRegisteredSW(swScriptUrl, registration) {
    console.debug('SW registered:', swScriptUrl, registration);
  },
  onRegisterError(error) {
    console.error('SW registration error:', error);
  },
});

function RequireAuth({ children }: { children: React.ReactNode }) {
  const status = useSession((s) => s.status);
  if (status === 'loading') return <div className="grid h-dvh place-items-center text-zinc-400">Загрузка…</div>;
  return status === 'authed' ? children : <Navigate to="/login" replace />;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ErrorBoundary>
      <BrowserRouter>
        <Routes>
          <Route path="/login" element={<AuthPage mode="login" />} />
          <Route path="/register" element={<AuthPage mode="register" />} />
          <Route path="/channels/:serverId?/:channelId?" element={<RequireAuth><AppLayout /></RequireAuth>} />
          <Route path="*" element={<Navigate to="/channels" replace />} />
        </Routes>
      </BrowserRouter>
    </ErrorBoundary>
  </StrictMode>,
);

import React from 'react';
import ReactDOM from 'react-dom/client';

import App from './App';
import ChatProvider from './chat/ChatProvider';
import I18nProvider from './i18n/I18nProvider';

import './app.css';

const rootElement = document.getElementById('root');

if (!rootElement) {
  throw new Error('Root element #root was not found.');
}

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <I18nProvider>
      <ChatProvider>
        <App />
      </ChatProvider>
    </I18nProvider>
  </React.StrictMode>,
);

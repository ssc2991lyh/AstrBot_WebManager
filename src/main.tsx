import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';

// 网页端只跑 Linux 无头服务器，无 macOS 分支（#4 决策：删除 is_macos）
ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App isMacOS={false} />
  </React.StrictMode>
);

import { BrowserRouter, Routes, Route, useNavigate, useLocation } from 'react-router-dom';
import { lazy, Suspense, useEffect, useMemo } from 'react';
import {
  Badge,
  Layout,
  Menu,
  ConfigProvider,
  App as AntdApp,
  theme,
} from 'antd';
import zhCN from 'antd/locale/zh_CN';
import {
  DesktopOutlined,
  CloudDownloadOutlined,
  SaveOutlined,
  FileTextOutlined,
  ToolOutlined,
  InfoCircleOutlined,
  MessageOutlined,
} from '@ant-design/icons';
import { ErrorBoundary } from './components/ErrorBoundary';
import { TitleBar } from './components/TitleBar';
import { AntdStaticProvider } from './antdStatic';
import { useAppStore, useUpdateStore, initEventListeners, cleanupEventListeners } from './stores';
import { useResolvedTheme } from './hooks';
const Dashboard = lazy(() => import('./pages/Dashboard'));
const Versions = lazy(() => import('./pages/Versions'));
const Backup = lazy(() => import('./pages/Backup'));
const Logs = lazy(() => import('./pages/Logs'));
const Advanced = lazy(() => import('./pages/Advanced'));
const About = lazy(() => import('./pages/About'));
const WebUIView = lazy(() => import('./pages/WebUIView'));
const FileManager = lazy(() => import('./pages/FileManager'));
const ConnectQQ = lazy(() => import('./pages/ConnectQQ'));
import './App.css';

const { Sider, Content } = Layout;
const UPDATE_INTERVAL_MS = 16 * 60 * 60 * 1000;

// Height of the custom titlebar. Must stay in sync with .titlebar { height } in App.css.
const TITLEBAR_HEIGHT = 40;

function DefaultCredentialsListener() {
  // 阶段2 用 WebSocket 订阅 default-credentials-detected 事件；阶段0(Mock) 先 no-op
  return null;
}

function AppLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const { token } = theme.useToken();
  const reloadSnapshot = useAppStore((s) => s.reloadSnapshot);
  const hasUpdate = useUpdateStore((s) => s.hasUpdate);

  useEffect(() => {
    void reloadSnapshot();
  }, [location.pathname, reloadSnapshot]);

  const menuItems = useMemo(
    () => [
      {
        key: '/',
        icon: <DesktopOutlined />,
        label: '实例',
      },
      {
        key: '/connectqq',
        icon: <MessageOutlined />,
        label: '连接QQ',
      },
      {
        key: '/versions',
        icon: <CloudDownloadOutlined />,
        label: '版本',
      },
      {
        key: '/backup',
        icon: <SaveOutlined />,
        label: '备份',
      },
      {
        key: '/logs',
        icon: <FileTextOutlined />,
        label: '日志',
      },
      {
        key: '/advanced',
        icon: <ToolOutlined />,
        label: '高级',
      },
      {
        key: '/about',
        icon: <InfoCircleOutlined />,
        label: (
          <Badge dot={hasUpdate} offset={[6, 0]}>
            关于
          </Badge>
        ),
      },
    ],
    [hasUpdate]
  );

  return (
    <div style={{ height: '100%', minHeight: 0, display: 'flex', flexDirection: 'column' }}>
      <Layout
        style={{
          flex: 1,
          minHeight: 0,
          overflow: 'hidden',
          background: token.colorBgLayout,
        }}
      >
        <Sider
          width={180}
          style={{
            overflow: 'auto',
            height: '100%',
            minHeight: 0,
            background: token.colorBgContainer,
            borderRight: `1px solid ${token.colorBorderSecondary}`,
          }}
        >
          <Menu
            mode="inline"
            selectedKeys={[location.pathname]}
            items={menuItems}
            onClick={({ key }) => navigate(key)}
            style={{ borderRight: 0, background: token.colorBgContainer }}
          />
        </Sider>
        <Layout style={{ minHeight: 0, background: token.colorBgLayout }}>
          <Content
            style={{
              padding: 24,
              overflow: 'auto',
              height: '100%',
              minHeight: 0,
              background: token.colorBgLayout,
            }}
          >
            <ErrorBoundary>
              <Routes>
                <Route path="/" element={<Dashboard />} />
                <Route path="/versions" element={<Versions />} />
                <Route path="/backup" element={<Backup />} />
                <Route path="/logs" element={<Logs />} />
                <Route path="/advanced" element={<Advanced />} />
                <Route path="/about" element={<About />} />
                <Route path="/connectqq" element={<ConnectQQ />} />
                <Route path="/instance/:instanceId/files" element={<FileManager />} />
              </Routes>
            </ErrorBoundary>
          </Content>
        </Layout>
      </Layout>
    </div>
  );
}

function App({ isMacOS }: { isMacOS: boolean }) {
  const resolvedTheme = useResolvedTheme();

  useEffect(() => {
    void initEventListeners();
    void useAppStore.getState().reloadSnapshot();
    void useUpdateStore.getState().checkForUpdate();

    const timer = setInterval(() => {
      void useUpdateStore.getState().checkForUpdate();
    }, UPDATE_INTERVAL_MS);

    return () => {
      cleanupEventListeners();
      clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme;
    document.documentElement.style.colorScheme = resolvedTheme;
  }, [resolvedTheme]);

  // On non-macOS, a custom titlebar occupies the top TITLEBAR_HEIGHT px.
  // All Ant Design overlays that render via portals (Drawer, message, notification)
  // are offset below the titlebar so interactive elements are never hidden behind it.
  // The titlebar itself has z-index: 99999 in normal document flow, which places it
  // above any portal-based overlay (z-index ~1000) in the root stacking context.
  const titlebarOffset = isMacOS ? 0 : TITLEBAR_HEIGHT;

  return (
    <ConfigProvider
      locale={zhCN}
      theme={{
        algorithm: resolvedTheme === 'dark' ? theme.darkAlgorithm : theme.defaultAlgorithm,
        token: {
          borderRadius: 8,
        },
      }}
      drawer={{
        styles: {
          wrapper: {
            top: titlebarOffset,
            height: `calc(100% - ${titlebarOffset}px)`,
          },
        },
      }}
    >
      <AntdApp message={{ top: titlebarOffset + 8 }} notification={{ top: titlebarOffset + 8 }}>
        <AntdStaticProvider />
        <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
          {!isMacOS && <TitleBar />}
          <div style={{ flex: 1, height: 0, minHeight: 0, overflow: 'hidden' }}>
            <BrowserRouter>
              <DefaultCredentialsListener />
              <Suspense>
                <Routes>
                  <Route path="/webui/:instanceId" element={<WebUIView />} />
                  <Route path="/*" element={<AppLayout />} />
                </Routes>
              </Suspense>
            </BrowserRouter>
          </div>
        </div>
      </AntdApp>
    </ConfigProvider>
  );
}

export default App;

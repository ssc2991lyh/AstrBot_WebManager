import { useState, useEffect, useCallback, useRef } from 'react';
import { Button, Modal, Input, Tooltip } from 'antd';
import { CloudOutlined } from '@ant-design/icons';

const STORAGE_KEY = 'astrbot_web_manager.snowluma_console_url';

function getDefaultUrl(): string {
  if (typeof window === 'undefined') return 'http://127.0.0.1:5099';
  return `http://${window.location.hostname}:5099`;
}

export function TitleBar() {
  const [url, setUrl] = useState<string>(() => {
    if (typeof window === 'undefined') return getDefaultUrl();
    return localStorage.getItem(STORAGE_KEY) || getDefaultUrl();
  });
  const [editOpen, setEditOpen] = useState(false);
  const [inputValue, setInputValue] = useState('');
  const longPressTimer = useRef<number | null>(null);

  // 当 hostname 变化时（用户从本机换到局域网 IP），若未显式保存过则自动刷新默认地址
  useEffect(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (!saved) setUrl(getDefaultUrl());
  }, []);

  const openConsole = useCallback(() => {
    window.open(url, '_blank', 'noopener,noreferrer');
  }, [url]);

  const openEdit = useCallback(() => {
    setInputValue(url);
    setEditOpen(true);
  }, [url]);

  const saveUrl = useCallback(() => {
    let v = inputValue.trim();
    if (!v) {
      v = getDefaultUrl();
    } else if (!/^https?:\/\//i.test(v)) {
      v = 'http://' + v;
    }
    localStorage.setItem(STORAGE_KEY, v);
    setUrl(v);
    setEditOpen(false);
  }, [inputValue]);

  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      openEdit();
    },
    [openEdit]
  );

  const handleMouseDown = useCallback(() => {
    longPressTimer.current = window.setTimeout(() => {
      longPressTimer.current = null;
      openEdit();
    }, 600);
  }, [openEdit]);

  const cancelLongPress = useCallback(() => {
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current);
      longPressTimer.current = null;
    }
  }, []);

  return (
    <div className="titlebar">
      <div className="titlebar-title">AstrBot Web Manager</div>
      <div className="titlebar-controls">
        <Tooltip title="快速进入 SnowLuma 控制台（右键/长按编辑地址）">
          <Button
            type="text"
            className="titlebar-btn"
            icon={<CloudOutlined />}
            onClick={openConsole}
            onContextMenu={handleContextMenu}
            onMouseDown={handleMouseDown}
            onMouseUp={cancelLongPress}
            onMouseLeave={cancelLongPress}
          />
        </Tooltip>
      </div>
      <Modal
        title="编辑 SnowLuma 控制台地址"
        open={editOpen}
        onOk={saveUrl}
        onCancel={() => setEditOpen(false)}
        okText="保存"
        cancelText="取消"
      >
        <Input
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          placeholder={getDefaultUrl()}
          onPressEnter={saveUrl}
        />
      </Modal>
    </div>
  );
}

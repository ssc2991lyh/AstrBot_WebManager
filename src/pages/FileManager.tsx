import { useCallback, useEffect, useMemo, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  App,
  Breadcrumb,
  Button,
  Input,
  Modal,
  Space,
  Table,
  Tag,
  Tooltip,
  Upload,
  Popconfirm,
} from 'antd';
import {
  FolderOutlined,
  FileOutlined,
  ReloadOutlined,
  UploadOutlined,
  FolderAddOutlined,
  DownloadOutlined,
  EditOutlined,
  DeleteOutlined,
  CompressOutlined,
  ExpandOutlined,
  ArrowLeftOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { api, FileItem } from '../api';

function formatSize(size: number, isDir: boolean): string {
  if (isDir) return '—';
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / 1024 / 1024).toFixed(1)} MB`;
}
function formatTime(ts?: number): string {
  if (!ts) return '—';
  return new Date(ts * 1000).toLocaleString();
}

export default function FileManager() {
  const { instanceId = '' } = useParams<{ instanceId: string }>();
  const navigate = useNavigate();
  const { message, modal } = App.useApp();

  const [segments, setSegments] = useState<string[]>([]);
  const [items, setItems] = useState<FileItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [editing, setEditing] = useState<{ path: string; content: string } | null>(null);
  const [editSaving, setEditSaving] = useState(false);
  const [mkdirOpen, setMkdirOpen] = useState(false);
  const [mkdirName, setMkdirName] = useState('');

  const currentPath = useMemo(() => segments.join('/'), [segments]);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.files.lists(instanceId, currentPath);
      setItems(data.items || []);
    } catch (e: any) {
      message.error(`列目录失败：${e.message}`);
    } finally {
      setLoading(false);
    }
  }, [instanceId, currentPath, message]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const navigateTo = (idx: number) => {
    // idx: -1 = root
    setSegments(idx < 0 ? [] : segments.slice(0, idx + 1));
    setSelected([]);
  };

  const enterDir = (name: string) => {
    setSegments([...segments, name]);
    setSelected([]);
  };

  const openFile = async (name: string) => {
    const path = currentPath ? `${currentPath}/${name}` : name;
    try {
      const data = await api.files.content(instanceId, path);
      setEditing({ path, content: data.content });
    } catch (e: any) {
      message.error(`读取失败：${e.message}`);
    }
  };

  const saveEdit = async () => {
    if (!editing) return;
    setEditSaving(true);
    try {
      await api.files.write(instanceId, editing.path, editing.content);
      message.success('已保存');
      setEditing(null);
    } catch (e: any) {
      message.error(`保存失败：${e.message}`);
    } finally {
      setEditSaving(false);
    }
  };

  const doMkdir = async () => {
    if (!mkdirName.trim()) return;
    const path = currentPath ? `${currentPath}/${mkdirName.trim()}` : mkdirName.trim();
    try {
      await api.files.mkdir(instanceId, path);
      message.success('已创建文件夹');
      setMkdirOpen(false);
      setMkdirName('');
      void refresh();
    } catch (e: any) {
      message.error(`创建失败：${e.message}`);
    }
  };

  const doDelete = async (paths: string[]) => {
    try {
      await api.files.remove(instanceId, paths);
      message.success(`已删除 ${paths.length} 项`);
      setSelected([]);
      void refresh();
    } catch (e: any) {
      message.error(`删除失败：${e.message}`);
    }
  };

  const doRename = (name: string) => {
    const path = currentPath ? `${currentPath}/${name}` : name;
    let newName = '';
    modal.confirm({
      title: `重命名 ${name}`,
      content: <Input placeholder="新名称" onChange={(e) => (newName = e.target.value)} />,
      onOk: async () => {
        if (!newName.trim()) return;
        const newPath = currentPath ? `${currentPath}/${newName.trim()}` : newName.trim();
        try {
          await api.files.rename(instanceId, path, newPath);
          message.success('已重命名');
          void refresh();
        } catch (e: any) {
          message.error(`重命名失败：${e.message}`);
        }
      },
    });
  };

  const promptDest = (title: string, onOk: (dest: string) => void) => {
    let dest = '';
    modal.confirm({
      title,
      content: (
        <Input
          placeholder="目标目录（相对 core，空=当前目录）"
          defaultValue={currentPath}
          onChange={(e) => (dest = e.target.value)}
        />
      ),
      onOk: () => onOk(dest),
    });
  };

  const doCopy = () => {
    const paths = selected.map((n) => (currentPath ? `${currentPath}/${n}` : n));
    promptDest('复制选中项到', async (dest) => {
      try {
        await api.files.copy(instanceId, paths, dest);
        message.success('已复制');
        setSelected([]);
        void refresh();
      } catch (e: any) {
        message.error(`复制失败：${e.message}`);
      }
    });
  };

  const doMove = () => {
    const paths = selected.map((n) => (currentPath ? `${currentPath}/${n}` : n));
    promptDest('移动选中项到', async (dest) => {
      try {
        await api.files.move(instanceId, paths, dest);
        message.success('已移动');
        setSelected([]);
        void refresh();
      } catch (e: any) {
        message.error(`移动失败：${e.message}`);
      }
    });
  };

  const doCompress = () => {
    const paths = selected.map((n) => (currentPath ? `${currentPath}/${n}` : n));
    let dest = `${currentPath ? currentPath.split('/').pop() + '_' : ''}archive.zip`;
    modal.confirm({
      title: '压缩选中项',
      content: (
        <Input
          placeholder="压缩包路径（相对 core）"
          defaultValue={dest}
          onChange={(e) => (dest = e.target.value)}
        />
      ),
      onOk: async () => {
        try {
          await api.files.compress(instanceId, paths, dest);
          message.success('已压缩');
          setSelected([]);
          void refresh();
        } catch (e: any) {
          message.error(`压缩失败：${e.message}`);
        }
      },
    });
  };

  const doDecompress = (name: string) => {
    const path = currentPath ? `${currentPath}/${name}` : name;
    let dest = currentPath;
    modal.confirm({
      title: `解压 ${name}`,
      content: (
        <Input
          placeholder="解压到目录（相对 core，空=当前目录）"
          defaultValue={dest}
          onChange={(e) => (dest = e.target.value)}
        />
      ),
      onOk: async () => {
        try {
          await api.files.decompress(instanceId, path, dest);
          message.success('已解压');
          void refresh();
        } catch (e: any) {
          message.error(`解压失败：${e.message}`);
        }
      },
    });
  };

  const uploadProps = {
    multiple: true,
    showUploadList: false,
    customRequest: async (opts: any) => {
      const file: File = opts.file;
      try {
        const buf = await file.arrayBuffer();
        const bytes = new Uint8Array(buf);
        const { upload_id } = await api.files.uploadInit(instanceId, currentPath, file.name);
        const CHUNK = 1024 * 1024;
        let offset = 0;
        while (offset < bytes.length) {
          const slice = bytes.subarray(offset, offset + CHUNK);
          await api.files.uploadChunk(instanceId, upload_id, slice);
          offset += CHUNK;
          opts.onProgress({ percent: Math.floor((offset / bytes.length) * 100) });
        }
        await api.files.uploadFinish(instanceId, upload_id);
        opts.onSuccess({});
      } catch (e: any) {
        opts.onError(e);
        message.error(`上传 ${file.name} 失败：${e.message}`);
      }
    },
    onChange: () => {
      void refresh();
    },
  };

  const columns: ColumnsType<FileItem> = [
    {
      title: '名称',
      dataIndex: 'name',
      render: (name: string, row) => (
        <Space>
          {row.is_dir ? <FolderOutlined /> : <FileOutlined />}
          <a
            onClick={() => {
              if (row.is_dir) enterDir(name);
              else void openFile(name);
            }}
          >
            {name}
          </a>
        </Space>
      ),
    },
    {
      title: '类型',
      dataIndex: 'is_dir',
      width: 80,
      render: (d: boolean) => (d ? <Tag color="blue">目录</Tag> : <Tag>文件</Tag>),
    },
    {
      title: '大小',
      dataIndex: 'size',
      width: 120,
      render: (s: number, row) => formatSize(s, row.is_dir),
    },
    {
      title: '修改时间',
      dataIndex: 'modified',
      width: 180,
      render: (ts?: number) => formatTime(ts),
    },
    {
      title: '操作',
      width: 220,
      render: (_: unknown, row) => {
        const path = currentPath ? `${currentPath}/${row.name}` : row.name;
        return (
          <Space size="small">
            {!row.is_dir && (
              <Tooltip title="编辑">
                <Button
                  size="small"
                  icon={<EditOutlined />}
                  onClick={() => void openFile(row.name)}
                />
              </Tooltip>
            )}
            {!row.is_dir && (
              <Tooltip title="下载">
                <Button
                  size="small"
                  icon={<DownloadOutlined />}
                  onClick={() =>
                    window.open(api.files.downloadUrl(instanceId, path), '_blank')
                  }
                />
              </Tooltip>
            )}
            {row.name.endsWith('.zip') && (
              <Tooltip title="解压">
                <Button
                  size="small"
                  icon={<ExpandOutlined />}
                  onClick={() => doDecompress(row.name)}
                />
              </Tooltip>
            )}
            <Tooltip title="重命名">
              <Button size="small" icon={<EditOutlined />} onClick={() => doRename(row.name)} />
            </Tooltip>
            <Popconfirm title="确认删除？" onConfirm={() => void doDelete([path])}>
              <Button size="small" danger icon={<DeleteOutlined />} />
            </Popconfirm>
          </Space>
        );
      },
    },
  ];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <Space style={{ marginBottom: 12 }} wrap>
        <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/')}>
          返回实例列表
        </Button>
        <Button icon={<ReloadOutlined />} onClick={() => void refresh()} loading={loading}>
          刷新
        </Button>
        <Button
          icon={<FolderAddOutlined />}
          onClick={() => {
            setMkdirName('');
            setMkdirOpen(true);
          }}
        >
          新建文件夹
        </Button>
        <Upload {...uploadProps}>
          <Button icon={<UploadOutlined />}>上传</Button>
        </Upload>
        <Popconfirm
          title={`删除选中的 ${selected.length} 项？`}
          disabled={!selected.length}
          onConfirm={() =>
            void doDelete(selected.map((n) => (currentPath ? `${currentPath}/${n}` : n)))
          }
        >
          <Button icon={<DeleteOutlined />} danger disabled={!selected.length}>
            删除选中
          </Button>
        </Popconfirm>
        <Button
          icon={<CompressOutlined />}
          disabled={!selected.length}
          onClick={doCompress}
        >
          压缩选中
        </Button>
        <Button disabled={!selected.length} onClick={doCopy}>
          复制选中
        </Button>
        <Button disabled={!selected.length} onClick={doMove}>
          移动选中
        </Button>
      </Space>

      <Breadcrumb
        style={{ marginBottom: 12 }}
        items={[
          { title: <a onClick={() => navigateTo(-1)}>core</a> },
          ...segments.map((s, i) => ({
            title: <a onClick={() => navigateTo(i)}>{s}</a>,
          })),
        ]}
      />

      <div style={{ flex: 1, minHeight: 0, overflow: 'auto' }}>
        <Table<FileItem>
          rowKey="name"
          loading={loading}
          columns={columns}
          dataSource={items}
          pagination={false}
          size="small"
          rowSelection={{
            selectedRowKeys: selected,
            onChange: (keys) => setSelected(keys as string[]),
          }}
        />
      </div>

      <Modal
        title={`编辑：${editing?.path}`}
        open={!!editing}
        onCancel={() => setEditing(null)}
        onOk={saveEdit}
        confirmLoading={editSaving}
        width={800}
        okText="保存"
        cancelText="取消"
      >
        <Input.TextArea
          value={editing?.content}
          onChange={(e) => setEditing((p) => (p ? { ...p, content: e.target.value } : p))}
          style={{ height: 420, fontFamily: 'monospace', fontSize: 13 }}
          spellCheck={false}
        />
      </Modal>

      <Modal
        title="新建文件夹"
        open={mkdirOpen}
        onCancel={() => setMkdirOpen(false)}
        onOk={doMkdir}
        okText="创建"
        cancelText="取消"
      >
        <Input
          value={mkdirName}
          onChange={(e) => setMkdirName(e.target.value)}
          placeholder="文件夹名称"
          onPressEnter={doMkdir}
        />
      </Modal>
    </div>
  );
}

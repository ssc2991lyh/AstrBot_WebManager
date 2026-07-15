import { Tag, Space, Tooltip } from 'antd';
import { WarningOutlined } from '@ant-design/icons';
import type { InstanceStatus, DeployProgress } from '../types';
import { isInstanceDeploying } from '../utils';

interface InstanceStatusTagProps {
  instance: InstanceStatus;
  deployProgress?: DeployProgress | null;
}

export function InstanceStatusTag({ instance, deployProgress }: InstanceStatusTagProps) {
  const isDeploying = isInstanceDeploying(instance.id, deployProgress);

  if (isDeploying) {
    return <Tag color="processing">部署中</Tag>;
  }

  let tagColor: string;
  let tagText: string;

  switch (instance.state) {
    case 'running':
      tagColor = 'green';
      tagText = '运行中';
      break;
    case 'starting':
      tagColor = 'processing';
      tagText = '启动中';
      break;
    case 'stopping':
      tagColor = 'processing';
      tagText = '正在停止';
      break;
    case 'stopped':
    default:
      tagColor = 'default';
      tagText = '已停止';
      break;
  }

  return (
    <Space size="small">
      <Tag color={tagColor}>{tagText}</Tag>
      {instance.state !== 'stopped' && instance.pid_tracker_not_available && (
        <Tooltip title="Launcher无法正确追踪此实例运行状态">
          <WarningOutlined style={{ color: '#faad14' }} />
        </Tooltip>
      )}
    </Space>
  );
}

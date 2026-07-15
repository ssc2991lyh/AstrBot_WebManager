import { Button, Space, Typography } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';

const { Title } = Typography;

interface PageHeaderProps {
  title: string;
  onRefresh?: () => void;
  refreshLoading?: boolean;
  actions?: React.ReactNode;
}

export function PageHeader({ title, onRefresh, refreshLoading = false, actions }: PageHeaderProps) {
  return (
    <div
      style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: 16,
      }}
    >
      <Title level={4} style={{ margin: 0 }}>
        {title}
      </Title>
      <Space>
        {onRefresh && (
          <Button icon={<ReloadOutlined />} onClick={onRefresh} loading={refreshLoading}>
            刷新
          </Button>
        )}
        {actions}
      </Space>
    </div>
  );
}

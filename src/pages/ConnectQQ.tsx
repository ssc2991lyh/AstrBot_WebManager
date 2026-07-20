import { Card, Empty } from 'antd';
import { PageHeader } from '../components';

export default function ConnectQQ() {
  return (
    <>
      <PageHeader title="连接QQ" />
      <Card style={{ marginTop: 24 }}>
        <Empty description="连接QQ 页面正在开发中，敬请期待" />
      </Card>
    </>
  );
}

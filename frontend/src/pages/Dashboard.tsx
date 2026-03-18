import React from 'react';
import { Card, Col, Row, Statistic, Typography } from 'antd';
import {
  BookOutlined,
  DatabaseOutlined,
  SafetyCertificateOutlined,
  AppstoreOutlined,
  PartitionOutlined,
  CheckSquareOutlined,
} from '@ant-design/icons';

const { Title } = Typography;

const Dashboard: React.FC = () => {
  return (
    <div>
      <Title level={3}>Dashboard</Title>
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={8} xl={4}>
          <Card hoverable>
            <Statistic
              title="Glossary Terms"
              value={0}
              prefix={<BookOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={8} xl={4}>
          <Card hoverable>
            <Statistic
              title="Data Elements"
              value={0}
              prefix={<DatabaseOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={8} xl={4}>
          <Card hoverable>
            <Statistic
              title="Critical Data Elements"
              value={0}
              prefix={<DatabaseOutlined />}
              valueStyle={{ color: '#FF4D4F' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={8} xl={4}>
          <Card hoverable>
            <Statistic
              title="Quality Rules"
              value={0}
              prefix={<SafetyCertificateOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={8} xl={4}>
          <Card hoverable>
            <Statistic
              title="Applications"
              value={0}
              prefix={<AppstoreOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={8} xl={4}>
          <Card hoverable>
            <Statistic
              title="Pending Tasks"
              value={0}
              prefix={<CheckSquareOutlined />}
              valueStyle={{ color: '#FAAD14' }}
            />
          </Card>
        </Col>
      </Row>
      <Row gutter={[16, 16]} style={{ marginTop: 24 }}>
        <Col xs={24} lg={12}>
          <Card title="Recent Activity">
            <Typography.Text type="secondary">
              No recent activity. Start by creating a glossary term or data element.
            </Typography.Text>
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card title="My Pending Tasks">
            <Typography.Text type="secondary">
              No pending tasks assigned to you.
            </Typography.Text>
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default Dashboard;

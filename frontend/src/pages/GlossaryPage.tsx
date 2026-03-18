import React from 'react';
import { Button, Card, Input, Space, Table, Tag, Typography } from 'antd';
import { PlusOutlined, SearchOutlined } from '@ant-design/icons';

const { Title } = Typography;

const statusColors: Record<string, string> = {
  DRAFT: 'default',
  PROPOSED: 'processing',
  UNDER_REVIEW: 'warning',
  REVISED: 'orange',
  ACCEPTED: 'success',
  REJECTED: 'error',
  DEPRECATED: 'default',
};

const columns = [
  { title: 'Term Name', dataIndex: 'term_name', key: 'term_name', sorter: true },
  { title: 'Domain', dataIndex: 'domain_name', key: 'domain_name' },
  { title: 'Definition', dataIndex: 'definition', key: 'definition', ellipsis: true },
  {
    title: 'Status',
    dataIndex: 'status_code',
    key: 'status_code',
    render: (status: string) => (
      <Tag color={statusColors[status] || 'default'}>{status}</Tag>
    ),
  },
  { title: 'Owner', dataIndex: 'owner_name', key: 'owner_name' },
  {
    title: 'Actions',
    key: 'actions',
    render: () => <a>View</a>,
  },
];

const GlossaryPage: React.FC = () => {
  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 16 }}>
        <Title level={3}>Business Glossary</Title>
        <Button type="primary" icon={<PlusOutlined />}>
          New Term
        </Button>
      </div>
      <Card>
        <Space style={{ marginBottom: 16 }}>
          <Input
            placeholder="Search terms..."
            prefix={<SearchOutlined />}
            style={{ width: 300 }}
          />
        </Space>
        <Table
          columns={columns}
          dataSource={[]}
          rowKey="term_id"
          pagination={{ pageSize: 20, showSizeChanger: true }}
        />
      </Card>
    </div>
  );
};

export default GlossaryPage;

import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Alert, Button, Card, Space, Table, Tag, Typography, message } from 'antd';
import { ArrowLeftOutlined, WarningOutlined } from '@ant-design/icons';
import { processesApi } from '../services/processesApi';
import type { CriticalProcessSummary } from '../services/processesApi';

const { Title, Text } = Typography;

const frequencyLabels: Record<string, string> = {
  DAILY: 'Daily',
  WEEKLY: 'Weekly',
  MONTHLY: 'Monthly',
  QUARTERLY: 'Quarterly',
  ANNUAL: 'Annual',
  ON_DEMAND: 'On Demand',
};

const CriticalProcessesPage: React.FC = () => {
  const navigate = useNavigate();
  const [processes, setProcesses] = useState<CriticalProcessSummary[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchCriticalProcesses = useCallback(async () => {
    setLoading(true);
    try {
      const response = await processesApi.listCriticalProcesses();
      setProcesses(response.data);
    } catch {
      message.error('Failed to load critical processes.');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchCriticalProcesses();
  }, [fetchCriticalProcesses]);

  const columns = [
    {
      title: 'Process Name',
      dataIndex: 'process_name',
      key: 'process_name',
      render: (name: string, record: CriticalProcessSummary) => (
        <a onClick={() => navigate(`/processes/${record.process_id}`)}>{name}</a>
      ),
    },
    {
      title: 'Process Code',
      dataIndex: 'process_code',
      key: 'process_code',
      width: 150,
      render: (code: string) => (
        <Text code style={{ fontSize: 12 }}>
          {code}
        </Text>
      ),
    },
    {
      title: 'Category',
      dataIndex: 'category_name',
      key: 'category_name',
      width: 160,
      render: (name: string | null) => name || '-',
    },
    {
      title: 'Owner',
      dataIndex: 'owner_name',
      key: 'owner_name',
      render: (name: string | null) => name || '-',
    },
    {
      title: 'Frequency',
      dataIndex: 'frequency',
      key: 'frequency',
      width: 120,
      render: (freq: string | null) =>
        freq ? (frequencyLabels[freq] || freq) : '-',
    },
    {
      title: 'Data Elements',
      dataIndex: 'data_elements_count',
      key: 'data_elements_count',
      width: 140,
      align: 'center' as const,
      render: (count: number) => (
        <Tag color={count > 0 ? 'blue' : 'default'}>{count}</Tag>
      ),
    },
  ];

  return (
    <div>
      <Space align="center" style={{ marginBottom: 16 }}>
        <Button
          type="text"
          icon={<ArrowLeftOutlined />}
          onClick={() => navigate('/processes')}
        />
        <Title level={3} style={{ margin: 0 }}>
          Critical Business Processes
        </Title>
      </Space>

      <Alert
        type="error"
        showIcon
        icon={<WarningOutlined />}
        message="Critical Business Processes"
        description="These processes have been designated as critical to business operations. All data elements linked to these processes are automatically classified as Critical Data Elements (CDEs), ensuring enhanced data quality monitoring and governance."
        style={{ marginBottom: 24 }}
      />

      <Card>
        <Table
          columns={columns}
          dataSource={processes}
          rowKey="process_id"
          loading={loading}
          pagination={{
            showSizeChanger: true,
            pageSizeOptions: ['10', '20', '50'],
            showTotal: (total) => `${total} critical processes`,
          }}
        />
      </Card>
    </div>
  );
};

export default CriticalProcessesPage;

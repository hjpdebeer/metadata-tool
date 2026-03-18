import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Button, Card, Input, Select, Space, Switch, Table, Tag, Typography, message } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import type { TablePaginationConfig } from 'antd';
import { processesApi } from '../services/processesApi';
import type {
  BusinessProcessListItem,
  ListProcessesParams,
  ProcessCategory,
} from '../services/processesApi';

const { Title, Text } = Typography;

const statusColors: Record<string, string> = {
  DRAFT: 'default',
  PROPOSED: 'processing',
  UNDER_REVIEW: 'warning',
  REVISED: 'orange',
  ACCEPTED: 'success',
  REJECTED: 'error',
  DEPRECATED: 'default',
};

const statusLabels: Record<string, string> = {
  DRAFT: 'Draft',
  PROPOSED: 'Proposed',
  UNDER_REVIEW: 'Under Review',
  REVISED: 'Revised',
  ACCEPTED: 'Accepted',
  REJECTED: 'Rejected',
  DEPRECATED: 'Deprecated',
};

const statusOptions = Object.entries(statusLabels).map(([value, label]) => ({
  value,
  label,
}));

const frequencyLabels: Record<string, string> = {
  DAILY: 'Daily',
  WEEKLY: 'Weekly',
  MONTHLY: 'Monthly',
  QUARTERLY: 'Quarterly',
  ANNUAL: 'Annual',
  ON_DEMAND: 'On Demand',
};

const ProcessesPage: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  const [processes, setProcesses] = useState<BusinessProcessListItem[]>([]);
  const [categories, setCategories] = useState<ProcessCategory[]>([]);
  const [loading, setLoading] = useState(false);
  const [totalCount, setTotalCount] = useState(0);

  const [searchQuery, setSearchQuery] = useState(searchParams.get('query') || '');
  const [selectedCategory, setSelectedCategory] = useState<string | undefined>(
    searchParams.get('category_id') || undefined,
  );
  const [selectedStatus, setSelectedStatus] = useState<string | undefined>(
    searchParams.get('status') || undefined,
  );
  const [criticalOnly, setCriticalOnly] = useState(searchParams.get('critical') === 'true');
  const [currentPage, setCurrentPage] = useState(
    Number(searchParams.get('page')) || 1,
  );
  const [pageSize, setPageSize] = useState(
    Number(searchParams.get('page_size')) || 20,
  );

  const fetchProcesses = useCallback(async () => {
    setLoading(true);
    try {
      const params: ListProcessesParams = {
        page: currentPage,
        page_size: pageSize,
        query: searchQuery || undefined,
        category_id: selectedCategory,
        status: selectedStatus,
        is_critical: criticalOnly || undefined,
      };

      const response = await processesApi.listProcesses(params);
      const data = response.data;

      if (Array.isArray(data)) {
        setProcesses(data);
        setTotalCount(data.length);
      } else {
        const paginated = data as unknown as { data: BusinessProcessListItem[]; total_count: number };
        setProcesses(paginated.data);
        setTotalCount(paginated.total_count);
      }
    } catch {
      message.error('Failed to load business processes.');
    } finally {
      setLoading(false);
    }
  }, [currentPage, pageSize, searchQuery, selectedCategory, selectedStatus, criticalOnly]);

  const fetchCategories = useCallback(async () => {
    try {
      const response = await processesApi.listCategories();
      setCategories(response.data);
    } catch {
      // Categories fetch is non-critical
    }
  }, []);

  useEffect(() => {
    fetchCategories();
  }, [fetchCategories]);

  useEffect(() => {
    fetchProcesses();
  }, [fetchProcesses]);

  // Sync state to URL params
  useEffect(() => {
    const params: Record<string, string> = {};
    if (searchQuery) params.query = searchQuery;
    if (selectedCategory) params.category_id = selectedCategory;
    if (selectedStatus) params.status = selectedStatus;
    if (criticalOnly) params.critical = 'true';
    if (currentPage > 1) params.page = String(currentPage);
    if (pageSize !== 20) params.page_size = String(pageSize);
    setSearchParams(params, { replace: true });
  }, [searchQuery, selectedCategory, selectedStatus, criticalOnly, currentPage, pageSize, setSearchParams]);

  const handleSearch = (value: string) => {
    setSearchQuery(value);
    setCurrentPage(1);
  };

  const handleCategoryChange = (value: string | undefined) => {
    setSelectedCategory(value || undefined);
    setCurrentPage(1);
  };

  const handleStatusChange = (value: string | undefined) => {
    setSelectedStatus(value || undefined);
    setCurrentPage(1);
  };

  const handleCriticalToggle = (checked: boolean) => {
    setCriticalOnly(checked);
    setCurrentPage(1);
  };

  const handleTableChange = (pagination: TablePaginationConfig) => {
    setCurrentPage(pagination.current || 1);
    setPageSize(pagination.pageSize || 20);
  };

  const columns = [
    {
      title: 'Process Name',
      dataIndex: 'process_name',
      key: 'process_name',
      sorter: true,
      render: (name: string, record: BusinessProcessListItem) => (
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
      title: 'Critical',
      dataIndex: 'is_critical',
      key: 'is_critical',
      width: 180,
      align: 'center' as const,
      render: (isCritical: boolean) =>
        isCritical ? (
          <Tag color="red" style={{ fontWeight: 600 }}>
            Critical Business Process
          </Tag>
        ) : null,
    },
    {
      title: 'Status',
      dataIndex: 'status_code',
      key: 'status_code',
      width: 140,
      render: (status: string) => (
        <Tag color={statusColors[status] || 'default'}>
          {statusLabels[status] || status}
        </Tag>
      ),
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
      title: 'Updated',
      dataIndex: 'updated_at',
      key: 'updated_at',
      width: 140,
      render: (date: string) => {
        if (!date) return '-';
        return new Date(date).toLocaleDateString('en-ZA', {
          year: 'numeric',
          month: 'short',
          day: 'numeric',
        });
      },
    },
  ];

  const categoryOptions = categories.map((c) => ({
    value: c.category_id,
    label: c.category_name,
  }));

  return (
    <div>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 16,
        }}
      >
        <Title level={3} style={{ margin: 0 }}>
          Business Process Registry
        </Title>
        <Space>
          <Button onClick={() => navigate('/processes/critical')}>
            Critical Processes
          </Button>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => navigate('/processes/new')}
          >
            New Process
          </Button>
        </Space>
      </div>
      <Card>
        <Space wrap style={{ marginBottom: 16, width: '100%' }}>
          <Input.Search
            placeholder="Search processes..."
            style={{ width: 300 }}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onSearch={handleSearch}
            allowClear
          />
          <Select
            placeholder="Filter by category"
            style={{ width: 200 }}
            value={selectedCategory}
            onChange={handleCategoryChange}
            options={categoryOptions}
            allowClear
          />
          <Select
            placeholder="Filter by status"
            style={{ width: 180 }}
            value={selectedStatus}
            onChange={handleStatusChange}
            options={statusOptions}
            allowClear
          />
          <Space>
            <Text type="secondary">Critical only</Text>
            <Switch
              checked={criticalOnly}
              onChange={handleCriticalToggle}
              size="small"
            />
          </Space>
        </Space>
        <Table
          columns={columns}
          dataSource={processes}
          rowKey="process_id"
          loading={loading}
          onChange={handleTableChange}
          pagination={{
            current: currentPage,
            pageSize: pageSize,
            total: totalCount,
            showSizeChanger: true,
            pageSizeOptions: ['10', '20', '50', '100'],
            showTotal: (total, range) => `${range[0]}-${range[1]} of ${total} processes`,
          }}
        />
      </Card>
    </div>
  );
};

export default ProcessesPage;

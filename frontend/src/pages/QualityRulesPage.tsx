import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Button, Card, Input, Select, Space, Switch, Table, Tag, Typography, message } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import type { TablePaginationConfig } from 'antd';
import { dataQualityApi } from '../services/dataQualityApi';
import type {
  ListRulesParams,
  QualityDimensionSummary,
  QualityRuleListItem,
} from '../services/dataQualityApi';

const { Title, Text } = Typography;

const severityColors: Record<string, string> = {
  LOW: '#52C41A',
  MEDIUM: '#1890FF',
  HIGH: '#FA8C16',
  CRITICAL: '#FF4D4F',
};

const severityOptions = [
  { value: 'LOW', label: 'Low' },
  { value: 'MEDIUM', label: 'Medium' },
  { value: 'HIGH', label: 'High' },
  { value: 'CRITICAL', label: 'Critical' },
];

const QualityRulesPage: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  const [rules, setRules] = useState<QualityRuleListItem[]>([]);
  const [dimensions, setDimensions] = useState<QualityDimensionSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [totalCount, setTotalCount] = useState(0);

  const [searchQuery, setSearchQuery] = useState(searchParams.get('query') || '');
  const [selectedDimension, setSelectedDimension] = useState<string | undefined>(
    searchParams.get('dimension_id') || undefined,
  );
  const [selectedSeverity, setSelectedSeverity] = useState<string | undefined>(
    searchParams.get('severity') || undefined,
  );
  const [activeOnly, setActiveOnly] = useState(searchParams.get('active') === 'true');
  const [currentPage, setCurrentPage] = useState(
    Number(searchParams.get('page')) || 1,
  );
  const [pageSize, setPageSize] = useState(
    Number(searchParams.get('page_size')) || 20,
  );

  const fetchRules = useCallback(async () => {
    setLoading(true);
    try {
      const params: ListRulesParams = {
        page: currentPage,
        page_size: pageSize,
        query: searchQuery || undefined,
        dimension_id: selectedDimension,
        severity: selectedSeverity,
        is_active: activeOnly || undefined,
      };

      const response = await dataQualityApi.listRules(params);
      const data = response.data;

      // Handle both paginated and flat array responses
      if (Array.isArray(data)) {
        setRules(data);
        setTotalCount(data.length);
      } else {
        const paginated = data as unknown as { data: QualityRuleListItem[]; total_count: number };
        setRules(paginated.data);
        setTotalCount(paginated.total_count);
      }
    } catch {
      message.error('Failed to load quality rules.');
    } finally {
      setLoading(false);
    }
  }, [currentPage, pageSize, searchQuery, selectedDimension, selectedSeverity, activeOnly]);

  const fetchDimensions = useCallback(async () => {
    try {
      const response = await dataQualityApi.listDimensions();
      setDimensions(response.data);
    } catch {
      // Dimensions fetch is non-critical
    }
  }, []);

  useEffect(() => {
    fetchDimensions();
  }, [fetchDimensions]);

  useEffect(() => {
    fetchRules();
  }, [fetchRules]);

  // Sync state to URL params
  useEffect(() => {
    const params: Record<string, string> = {};
    if (searchQuery) params.query = searchQuery;
    if (selectedDimension) params.dimension_id = selectedDimension;
    if (selectedSeverity) params.severity = selectedSeverity;
    if (activeOnly) params.active = 'true';
    if (currentPage > 1) params.page = String(currentPage);
    if (pageSize !== 20) params.page_size = String(pageSize);
    setSearchParams(params, { replace: true });
  }, [searchQuery, selectedDimension, selectedSeverity, activeOnly, currentPage, pageSize, setSearchParams]);

  const handleSearch = (value: string) => {
    setSearchQuery(value);
    setCurrentPage(1);
  };

  const handleDimensionChange = (value: string | undefined) => {
    setSelectedDimension(value || undefined);
    setCurrentPage(1);
  };

  const handleSeverityChange = (value: string | undefined) => {
    setSelectedSeverity(value || undefined);
    setCurrentPage(1);
  };

  const handleActiveToggle = (checked: boolean) => {
    setActiveOnly(checked);
    setCurrentPage(1);
  };

  const handleTableChange = (pagination: TablePaginationConfig) => {
    setCurrentPage(pagination.current || 1);
    setPageSize(pagination.pageSize || 20);
  };

  const columns = [
    {
      title: 'Rule Name',
      dataIndex: 'rule_name',
      key: 'rule_name',
      sorter: true,
      render: (name: string, record: QualityRuleListItem) => (
        <a onClick={() => navigate(`/data-quality/rules/${record.rule_id}`)}>{name}</a>
      ),
    },
    {
      title: 'Rule Code',
      dataIndex: 'rule_code',
      key: 'rule_code',
      width: 180,
      render: (code: string) => (
        <Text code style={{ fontSize: 12 }}>
          {code}
        </Text>
      ),
    },
    {
      title: 'Dimension',
      dataIndex: 'dimension_name',
      key: 'dimension_name',
      width: 140,
      render: (name: string, record: QualityRuleListItem) => (
        <Tag color="blue">{name || record.dimension_code}</Tag>
      ),
    },
    {
      title: 'Element',
      dataIndex: 'element_name',
      key: 'element_name',
      render: (name: string | null) => name || '-',
    },
    {
      title: 'Severity',
      dataIndex: 'severity',
      key: 'severity',
      width: 110,
      render: (severity: string) => (
        <Tag color={severityColors[severity] || 'default'} style={{ fontWeight: 600 }}>
          {severity}
        </Tag>
      ),
    },
    {
      title: 'Active',
      dataIndex: 'is_active',
      key: 'is_active',
      width: 80,
      align: 'center' as const,
      render: (isActive: boolean) =>
        isActive ? (
          <Tag color="green">Active</Tag>
        ) : (
          <Tag color="default">Inactive</Tag>
        ),
    },
    {
      title: 'Owner',
      dataIndex: 'owner_name',
      key: 'owner_name',
      render: (name: string | null) => name || '-',
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

  const dimensionOptions = dimensions.map((d) => ({
    value: d.dimension_id,
    label: d.dimension_name,
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
          Quality Rules
        </Title>
        <Space>
          <Button onClick={() => navigate('/data-quality')}>
            Overview
          </Button>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => navigate('/data-quality/rules/new')}
          >
            New Rule
          </Button>
        </Space>
      </div>
      <Card>
        <Space wrap style={{ marginBottom: 16, width: '100%' }}>
          <Input.Search
            placeholder="Search rules..."
            style={{ width: 300 }}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onSearch={handleSearch}
            allowClear
          />
          <Select
            placeholder="Filter by dimension"
            style={{ width: 200 }}
            value={selectedDimension}
            onChange={handleDimensionChange}
            options={dimensionOptions}
            allowClear
          />
          <Select
            placeholder="Filter by severity"
            style={{ width: 160 }}
            value={selectedSeverity}
            onChange={handleSeverityChange}
            options={severityOptions}
            allowClear
          />
          <Space>
            <Text type="secondary">Active only</Text>
            <Switch
              checked={activeOnly}
              onChange={handleActiveToggle}
              size="small"
            />
          </Space>
        </Space>
        <Table
          columns={columns}
          dataSource={rules}
          rowKey="rule_id"
          loading={loading}
          onChange={handleTableChange}
          pagination={{
            current: currentPage,
            pageSize: pageSize,
            total: totalCount,
            showSizeChanger: true,
            pageSizeOptions: ['10', '20', '50', '100'],
            showTotal: (total, range) => `${range[0]}-${range[1]} of ${total} rules`,
          }}
        />
      </Card>
    </div>
  );
};

export default QualityRulesPage;

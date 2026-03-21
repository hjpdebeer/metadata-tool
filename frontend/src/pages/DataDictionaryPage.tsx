import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Button, Card, Input, Select, Space, Switch, Table, Tag, Typography, message } from 'antd';
import { PlusOutlined, UploadOutlined } from '@ant-design/icons';
import type { TablePaginationConfig } from 'antd';
import { dataDictionaryApi } from '../services/dataDictionaryApi';
import { glossaryApi } from '../services/glossaryApi';
import type { DataElementListItem, ListElementsParams } from '../services/dataDictionaryApi';
import type { GlossaryDomain } from '../services/glossaryApi';
import DeBulkUploadModal from '../components/DeBulkUploadModal';

import { statusColors, statusLabels, statusOptions } from '../constants/statusConfig';

const { Title, Text } = Typography;

const DataDictionaryPage: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  const [elements, setElements] = useState<DataElementListItem[]>([]);
  const [domains, setDomains] = useState<GlossaryDomain[]>([]);
  const [loading, setLoading] = useState(false);
  const [totalCount, setTotalCount] = useState(0);
  const [bulkUploadOpen, setBulkUploadOpen] = useState(false);

  const [searchQuery, setSearchQuery] = useState(searchParams.get('query') || '');
  const [selectedDomain, setSelectedDomain] = useState<string | undefined>(
    searchParams.get('domain_id') || undefined,
  );
  const [selectedStatus, setSelectedStatus] = useState<string | undefined>(
    searchParams.get('status') || undefined,
  );
  const [cdeOnly, setCdeOnly] = useState(searchParams.get('cde') === 'true');
  const [currentPage, setCurrentPage] = useState(
    Number(searchParams.get('page')) || 1,
  );
  const [pageSize, setPageSize] = useState(
    Number(searchParams.get('page_size')) || 20,
  );

  const fetchElements = useCallback(async () => {
    setLoading(true);
    try {
      const params: ListElementsParams = {
        page: currentPage,
        page_size: pageSize,
        query: searchQuery || undefined,
        domain_id: selectedDomain,
        status: selectedStatus,
        is_cde: cdeOnly || undefined,
      };

      const response = await dataDictionaryApi.listElements(params);
      const data = response.data;

      // Handle both paginated and flat array responses
      if (Array.isArray(data)) {
        setElements(data);
        setTotalCount(data.length);
      } else {
        const paginated = data as unknown as { data: DataElementListItem[]; total_count: number };
        setElements(paginated.data);
        setTotalCount(paginated.total_count);
      }
    } catch {
      message.error('Failed to load data elements.');
    } finally {
      setLoading(false);
    }
  }, [currentPage, pageSize, searchQuery, selectedDomain, selectedStatus, cdeOnly]);

  const fetchDomains = useCallback(async () => {
    try {
      const response = await glossaryApi.listDomains();
      setDomains(response.data);
    } catch {
      // Domains fetch is non-critical
    }
  }, []);

  useEffect(() => {
    fetchDomains();
  }, [fetchDomains]);

  useEffect(() => {
    fetchElements();
  }, [fetchElements]);

  // Sync state to URL params
  useEffect(() => {
    const params: Record<string, string> = {};
    if (searchQuery) params.query = searchQuery;
    if (selectedDomain) params.domain_id = selectedDomain;
    if (selectedStatus) params.status = selectedStatus;
    if (cdeOnly) params.cde = 'true';
    if (currentPage > 1) params.page = String(currentPage);
    if (pageSize !== 20) params.page_size = String(pageSize);
    setSearchParams(params, { replace: true });
  }, [searchQuery, selectedDomain, selectedStatus, cdeOnly, currentPage, pageSize, setSearchParams]);

  const handleSearch = (value: string) => {
    setSearchQuery(value);
    setCurrentPage(1);
  };

  const handleDomainChange = (value: string | undefined) => {
    setSelectedDomain(value || undefined);
    setCurrentPage(1);
  };

  const handleStatusChange = (value: string | undefined) => {
    setSelectedStatus(value || undefined);
    setCurrentPage(1);
  };

  const handleCdeToggle = (checked: boolean) => {
    setCdeOnly(checked);
    setCurrentPage(1);
  };

  const handleTableChange = (pagination: TablePaginationConfig) => {
    setCurrentPage(pagination.current || 1);
    setPageSize(pagination.pageSize || 20);
  };

  const columns = [
    {
      title: 'Element Name',
      dataIndex: 'element_name',
      key: 'element_name',
      sorter: true,
      render: (name: string, record: DataElementListItem) => (
        <a onClick={() => navigate(`/data-dictionary/${record.element_id}`)}>{name}</a>
      ),
    },
    {
      title: 'Element Code',
      dataIndex: 'element_code',
      key: 'element_code',
      width: 180,
      render: (code: string) => (
        <Text code style={{ fontSize: 12 }}>
          {code}
        </Text>
      ),
    },
    {
      title: 'Data Type',
      dataIndex: 'data_type',
      key: 'data_type',
      width: 120,
    },
    {
      title: 'Domain',
      dataIndex: 'domain_name',
      key: 'domain_name',
      render: (domain: string | null) => domain || '-',
    },
    {
      title: 'Classification',
      dataIndex: 'classification_name',
      key: 'classification_name',
      width: 140,
      render: (name: string | null) => name || '-',
    },
    {
      title: 'CDE',
      dataIndex: 'is_cde',
      key: 'is_cde',
      width: 80,
      align: 'center' as const,
      render: (isCde: boolean) =>
        isCde ? (
          <Tag color="red" style={{ fontWeight: 600 }}>
            CDE
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

  const domainOptions = domains.map((d) => ({
    value: d.domain_id,
    label: d.domain_name,
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
          Data Dictionary
        </Title>
        <Space>
          <Button
            icon={<UploadOutlined />}
            onClick={() => setBulkUploadOpen(true)}
          >
            Bulk Upload
          </Button>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => navigate('/data-dictionary/new')}
          >
            New Element
          </Button>
        </Space>
      </div>
      <Card>
        <Space wrap style={{ marginBottom: 16, width: '100%' }}>
          <Input.Search
            placeholder="Search elements..."
            style={{ width: 300 }}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onSearch={handleSearch}
            allowClear
          />
          <Select
            placeholder="Filter by domain"
            style={{ width: 200 }}
            value={selectedDomain}
            onChange={handleDomainChange}
            options={domainOptions}
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
            <Text type="secondary">CDE only</Text>
            <Switch
              checked={cdeOnly}
              onChange={handleCdeToggle}
              size="small"
            />
          </Space>
        </Space>
        <Table
          columns={columns}
          dataSource={elements}
          rowKey="element_id"
          loading={loading}
          onChange={handleTableChange}
          pagination={{
            current: currentPage,
            pageSize: pageSize,
            total: totalCount,
            showSizeChanger: true,
            pageSizeOptions: ['10', '20', '50', '100'],
            showTotal: (total, range) => `${range[0]}-${range[1]} of ${total} elements`,
          }}
        />
      </Card>

      <DeBulkUploadModal
        open={bulkUploadOpen}
        onClose={() => setBulkUploadOpen(false)}
        onSuccess={fetchElements}
      />
    </div>
  );
};

export default DataDictionaryPage;

import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Button, Card, Input, Select, Space, Switch, Table, Tag, Typography, message } from 'antd';
import { PlusOutlined, UploadOutlined } from '@ant-design/icons';
import AppBulkUploadModal from '../components/AppBulkUploadModal';
import type { TablePaginationConfig } from 'antd';
import { applicationsApi } from '../services/applicationsApi';
import type {
  ApplicationClassification,
  ApplicationListItem,
  ListApplicationsParams,
} from '../services/applicationsApi';

import { statusColors, statusLabels, statusOptions } from '../constants/statusConfig';

const { Title, Text } = Typography;

const deploymentTypeColors: Record<string, string> = {
  ON_PREMISE: 'default',
  CLOUD: 'blue',
  HYBRID: 'purple',
  SAAS: 'cyan',
};

const deploymentTypeLabels: Record<string, string> = {
  ON_PREMISE: 'On-Premise',
  CLOUD: 'Cloud',
  HYBRID: 'Hybrid',
  SAAS: 'SaaS',
};

const deploymentTypeOptions = Object.entries(deploymentTypeLabels).map(([value, label]) => ({
  value,
  label,
}));

const ApplicationsPage: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  const [applications, setApplications] = useState<ApplicationListItem[]>([]);
  const [classifications, setClassifications] = useState<ApplicationClassification[]>([]);
  const [loading, setLoading] = useState(false);
  const [totalCount, setTotalCount] = useState(0);
  const [bulkUploadOpen, setBulkUploadOpen] = useState(false);

  const [searchQuery, setSearchQuery] = useState(searchParams.get('query') || '');
  const [selectedClassification, setSelectedClassification] = useState<string | undefined>(
    searchParams.get('classification_id') || undefined,
  );
  const [selectedStatus, setSelectedStatus] = useState<string | undefined>(
    searchParams.get('status') || undefined,
  );
  const [selectedDeploymentType, setSelectedDeploymentType] = useState<string | undefined>(
    searchParams.get('deployment_type') || undefined,
  );
  const [cbaOnly, setCbaOnly] = useState(searchParams.get('cba') === 'true');
  const [currentPage, setCurrentPage] = useState(
    Number(searchParams.get('page')) || 1,
  );
  const [pageSize, setPageSize] = useState(
    Number(searchParams.get('page_size')) || 20,
  );

  const fetchApplications = useCallback(async () => {
    setLoading(true);
    try {
      const params: ListApplicationsParams = {
        page: currentPage,
        page_size: pageSize,
        query: searchQuery || undefined,
        classification_id: selectedClassification,
        status: selectedStatus,
        deployment_type: selectedDeploymentType,
        is_cba: cbaOnly || undefined,
      };

      const response = await applicationsApi.listApplications(params);
      const data = response.data;

      if (Array.isArray(data)) {
        setApplications(data);
        setTotalCount(data.length);
      } else {
        const paginated = data as unknown as { data: ApplicationListItem[]; total_count: number };
        setApplications(paginated.data);
        setTotalCount(paginated.total_count);
      }
    } catch {
      message.error('Failed to load applications.');
    } finally {
      setLoading(false);
    }
  }, [currentPage, pageSize, searchQuery, selectedClassification, selectedStatus, selectedDeploymentType, cbaOnly]);

  const fetchClassifications = useCallback(async () => {
    try {
      const response = await applicationsApi.listClassifications();
      setClassifications(response.data);
    } catch {
      // Classifications fetch is non-critical
    }
  }, []);

  useEffect(() => {
    fetchClassifications();
  }, [fetchClassifications]);

  useEffect(() => {
    fetchApplications();
  }, [fetchApplications]);

  // Sync state to URL params
  useEffect(() => {
    const params: Record<string, string> = {};
    if (searchQuery) params.query = searchQuery;
    if (selectedClassification) params.classification_id = selectedClassification;
    if (selectedStatus) params.status = selectedStatus;
    if (selectedDeploymentType) params.deployment_type = selectedDeploymentType;
    if (cbaOnly) params.cba = 'true';
    if (currentPage > 1) params.page = String(currentPage);
    if (pageSize !== 20) params.page_size = String(pageSize);
    setSearchParams(params, { replace: true });
  }, [searchQuery, selectedClassification, selectedStatus, selectedDeploymentType, cbaOnly, currentPage, pageSize, setSearchParams]);

  const handleSearch = (value: string) => {
    setSearchQuery(value);
    setCurrentPage(1);
  };

  const handleClassificationChange = (value: string | undefined) => {
    setSelectedClassification(value || undefined);
    setCurrentPage(1);
  };

  const handleStatusChange = (value: string | undefined) => {
    setSelectedStatus(value || undefined);
    setCurrentPage(1);
  };

  const handleDeploymentTypeChange = (value: string | undefined) => {
    setSelectedDeploymentType(value || undefined);
    setCurrentPage(1);
  };

  const handleCbaToggle = (checked: boolean) => {
    setCbaOnly(checked);
    setCurrentPage(1);
  };

  const handleTableChange = (pagination: TablePaginationConfig) => {
    setCurrentPage(pagination.current || 1);
    setPageSize(pagination.pageSize || 20);
  };

  const columns = [
    {
      title: 'App Name',
      dataIndex: 'application_name',
      key: 'application_name',
      sorter: true,
      render: (name: string, record: ApplicationListItem) => (
        <a onClick={() => navigate(`/applications/${record.application_id}`)}>{name}</a>
      ),
    },
    {
      title: 'App Code',
      dataIndex: 'application_code',
      key: 'application_code',
      width: 140,
      render: (code: string) => (
        <Text code style={{ fontSize: 12 }}>
          {code}
        </Text>
      ),
    },
    {
      title: 'Classification',
      dataIndex: 'classification_name',
      key: 'classification_name',
      width: 160,
      render: (name: string | null) =>
        name ? <Tag>{name}</Tag> : '-',
    },
    {
      title: 'Vendor',
      dataIndex: 'vendor',
      key: 'vendor',
      width: 140,
      render: (vendor: string | null) => vendor || '-',
    },
    {
      title: 'Deployment',
      dataIndex: 'deployment_type',
      key: 'deployment_type',
      width: 120,
      render: (type: string | null) =>
        type ? (
          <Tag color={deploymentTypeColors[type] || 'default'}>
            {deploymentTypeLabels[type] || type}
          </Tag>
        ) : (
          '-'
        ),
    },
    {
      title: 'CBA',
      dataIndex: 'is_cba',
      key: 'is_cba',
      width: 90,
      align: 'center' as const,
      render: (isCba: boolean) =>
        isCba ? (
          <Tag color="red" style={{ fontWeight: 600 }}>
            CBA
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
      title: 'Business Owner',
      dataIndex: 'business_owner_name',
      key: 'business_owner_name',
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

  const classificationOptions = classifications.map((c) => ({
    value: c.classification_id,
    label: c.classification_name,
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
          Application Registry
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
            onClick={() => navigate('/applications/new')}
          >
            New Application
          </Button>
        </Space>
      </div>
      <Card>
        <Space wrap style={{ marginBottom: 16, width: '100%' }}>
          <Input.Search
            placeholder="Search applications..."
            style={{ width: 300 }}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onSearch={handleSearch}
            allowClear
          />
          <Select
            placeholder="Filter by classification"
            style={{ width: 200 }}
            value={selectedClassification}
            onChange={handleClassificationChange}
            options={classificationOptions}
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
          <Select
            placeholder="Deployment type"
            style={{ width: 160 }}
            value={selectedDeploymentType}
            onChange={handleDeploymentTypeChange}
            options={deploymentTypeOptions}
            allowClear
          />
          <Space>
            <Text type="secondary">CBA only</Text>
            <Switch
              checked={cbaOnly}
              onChange={handleCbaToggle}
              size="small"
            />
          </Space>
        </Space>
        <Table
          columns={columns}
          dataSource={applications}
          rowKey="application_id"
          loading={loading}
          onChange={handleTableChange}
          pagination={{
            current: currentPage,
            pageSize: pageSize,
            total: totalCount,
            showSizeChanger: true,
            pageSizeOptions: ['10', '20', '50', '100'],
            showTotal: (total, range) => `${range[0]}-${range[1]} of ${total} applications`,
          }}
        />
      </Card>

      <AppBulkUploadModal
        open={bulkUploadOpen}
        onClose={() => setBulkUploadOpen(false)}
        onSuccess={() => fetchApplications()}
      />
    </div>
  );
};

export default ApplicationsPage;

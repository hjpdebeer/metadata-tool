import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Button, Card, Input, Select, Space, Table, Tag, Typography, message } from 'antd';
import { PlusOutlined, SafetyCertificateOutlined, UploadOutlined } from '@ant-design/icons';
import BulkUploadModal from '../components/BulkUploadModal';
import type { TablePaginationConfig } from 'antd';
import { glossaryApi } from '../services/glossaryApi';
import type {
  GlossaryTermListItem,
  GlossaryDomain,
  GlossaryTermType,
  ListTermsParams,
} from '../services/glossaryApi';

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

const statusLabels: Record<string, string> = {
  DRAFT: 'Draft',
  PROPOSED: 'Proposed',
  UNDER_REVIEW: 'Under Review',
  PENDING_APPROVAL: 'Pending Approval',
  REVISED: 'Revised',
  ACCEPTED: 'Accepted',
  REJECTED: 'Rejected',
  DEPRECATED: 'Deprecated',
  SUPERSEDED: 'Superseded',
};

const statusOptions = Object.entries(statusLabels).map(([value, label]) => ({
  value,
  label,
}));

const GlossaryPage: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  const [terms, setTerms] = useState<GlossaryTermListItem[]>([]);
  const [domains, setDomains] = useState<GlossaryDomain[]>([]);
  const [termTypes, setTermTypes] = useState<GlossaryTermType[]>([]);
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
  const [selectedTermType, setSelectedTermType] = useState<string | undefined>(
    searchParams.get('term_type_id') || undefined,
  );
  const [currentPage, setCurrentPage] = useState(
    Number(searchParams.get('page')) || 1,
  );
  const [pageSize, setPageSize] = useState(
    Number(searchParams.get('page_size')) || 20,
  );

  const fetchTerms = useCallback(async () => {
    setLoading(true);
    try {
      const params: ListTermsParams = {
        page: currentPage,
        page_size: pageSize,
        query: searchQuery || undefined,
        domain_id: selectedDomain,
        status: selectedStatus,
        term_type_id: selectedTermType,
      };

      const response = await glossaryApi.listTerms(params);
      const data = response.data;

      // Handle both paginated and flat array responses
      if (Array.isArray(data)) {
        setTerms(data);
        setTotalCount(data.length);
      } else {
        const paginated = data as unknown as { data: GlossaryTermListItem[]; total_count: number };
        setTerms(paginated.data);
        setTotalCount(paginated.total_count);
      }
    } catch {
      message.error('Failed to load glossary terms.');
    } finally {
      setLoading(false);
    }
  }, [currentPage, pageSize, searchQuery, selectedDomain, selectedStatus, selectedTermType]);

  const fetchReferenceData = useCallback(async () => {
    const [domainsRes, typesRes] = await Promise.allSettled([
      glossaryApi.listDomains(),
      glossaryApi.listTermTypes(),
    ]);
    if (domainsRes.status === 'fulfilled') setDomains(domainsRes.value.data);
    if (typesRes.status === 'fulfilled') setTermTypes(typesRes.value.data);
  }, []);

  useEffect(() => {
    fetchReferenceData();
  }, [fetchReferenceData]);

  useEffect(() => {
    fetchTerms();
  }, [fetchTerms]);

  // Sync state to URL params
  useEffect(() => {
    const params: Record<string, string> = {};
    if (searchQuery) params.query = searchQuery;
    if (selectedDomain) params.domain_id = selectedDomain;
    if (selectedStatus) params.status = selectedStatus;
    if (selectedTermType) params.term_type_id = selectedTermType;
    if (currentPage > 1) params.page = String(currentPage);
    if (pageSize !== 20) params.page_size = String(pageSize);
    setSearchParams(params, { replace: true });
  }, [searchQuery, selectedDomain, selectedStatus, selectedTermType, currentPage, pageSize, setSearchParams]);

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

  const handleTermTypeChange = (value: string | undefined) => {
    setSelectedTermType(value || undefined);
    setCurrentPage(1);
  };

  const handleTableChange = (pagination: TablePaginationConfig) => {
    setCurrentPage(pagination.current || 1);
    setPageSize(pagination.pageSize || 20);
  };

  const columns = [
    {
      title: 'Term Name',
      dataIndex: 'term_name',
      key: 'term_name',
      sorter: true,
      render: (name: string, record: GlossaryTermListItem) => (
        <Space size={4}>
          <a onClick={() => navigate(`/glossary/${record.term_id}`)}>{name}</a>
          {record.is_cbt && (
            <Tag color="red" style={{ fontSize: 10, lineHeight: '16px', padding: '0 4px' }}>
              <SafetyCertificateOutlined /> CBT
            </Tag>
          )}
        </Space>
      ),
    },
    {
      title: 'Term Code',
      dataIndex: 'term_code',
      key: 'term_code',
      width: 150,
      render: (code: string | null) =>
        code ? (
          <Tag color="geekblue" style={{ fontFamily: 'monospace', fontSize: 11 }}>
            {code}
          </Tag>
        ) : (
          <span style={{ color: '#9CA3AF' }}>-</span>
        ),
    },
    {
      title: 'Domain',
      dataIndex: 'domain_name',
      key: 'domain_name',
      render: (domain: string | null) => domain || '-',
    },
    {
      title: 'Term Type',
      dataIndex: 'term_type_name',
      key: 'term_type_name',
      width: 160,
      render: (typeName: string | null) =>
        typeName ? <Tag color="purple">{typeName}</Tag> : '-',
    },
    {
      title: 'Definition',
      dataIndex: 'definition',
      key: 'definition',
      ellipsis: true,
      width: '22%',
    },
    {
      title: 'Status',
      dataIndex: 'status_code',
      key: 'status_code',
      width: 130,
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
      width: 120,
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

  const termTypeOptions = termTypes.map((t) => ({
    value: t.term_type_id,
    label: t.type_name,
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
          Business Glossary
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
            onClick={() => navigate('/glossary/new')}
          >
            New Term
          </Button>
        </Space>
      </div>
      <Card>
        <Space wrap style={{ marginBottom: 16, width: '100%' }}>
          <Input.Search
            placeholder="Search terms..."
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
            placeholder="Filter by type"
            style={{ width: 200 }}
            value={selectedTermType}
            onChange={handleTermTypeChange}
            options={termTypeOptions}
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
        </Space>
        <Table
          columns={columns}
          dataSource={terms}
          rowKey="term_id"
          loading={loading}
          onChange={handleTableChange}
          pagination={{
            current: currentPage,
            pageSize: pageSize,
            total: totalCount,
            showSizeChanger: true,
            pageSizeOptions: ['10', '20', '50', '100'],
            showTotal: (total, range) => `${range[0]}-${range[1]} of ${total} terms`,
          }}
          scroll={{ x: 1100 }}
        />
      </Card>

      <BulkUploadModal
        open={bulkUploadOpen}
        onClose={() => setBulkUploadOpen(false)}
        onSuccess={() => fetchTerms()}
      />
    </div>
  );
};

export default GlossaryPage;

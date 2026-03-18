import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import {
  Button,
  Card,
  Form,
  Input,
  Modal,
  Select,
  Space,
  Table,
  Tag,
  Typography,
  message,
} from 'antd';
import { PlusOutlined, ApartmentOutlined } from '@ant-design/icons';
import type { TablePaginationConfig } from 'antd';
import { lineageApi } from '../services/lineageApi';
import type {
  LineageGraphListItem,
  CreateLineageGraphRequest,
  ListGraphsParams,
} from '../services/lineageApi';

const { Title, Text } = Typography;
const { TextArea } = Input;

const graphTypeColors: Record<string, string> = {
  BUSINESS: 'blue',
  TECHNICAL: 'green',
};

const graphTypeLabels: Record<string, string> = {
  BUSINESS: 'Business',
  TECHNICAL: 'Technical',
};

const graphTypeOptions = [
  { value: 'BUSINESS', label: 'Business' },
  { value: 'TECHNICAL', label: 'Technical' },
];

const LineageGraphList: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  const [graphs, setGraphs] = useState<LineageGraphListItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [totalCount, setTotalCount] = useState(0);
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [form] = Form.useForm();

  const [selectedType, setSelectedType] = useState<string | undefined>(
    searchParams.get('graph_type') || undefined,
  );
  const [searchQuery, setSearchQuery] = useState(searchParams.get('query') || '');
  const [currentPage, setCurrentPage] = useState(
    Number(searchParams.get('page')) || 1,
  );
  const [pageSize, setPageSize] = useState(
    Number(searchParams.get('page_size')) || 20,
  );

  const fetchGraphs = useCallback(async () => {
    setLoading(true);
    try {
      const params: ListGraphsParams = {
        page: currentPage,
        page_size: pageSize,
        query: searchQuery || undefined,
        graph_type: selectedType,
      };

      const response = await lineageApi.listGraphs(params);
      const data = response.data;

      if (Array.isArray(data)) {
        setGraphs(data);
        setTotalCount(data.length);
      } else {
        const paginated = data as unknown as {
          data: LineageGraphListItem[];
          total_count: number;
        };
        setGraphs(paginated.data);
        setTotalCount(paginated.total_count);
      }
    } catch {
      message.error('Failed to load lineage graphs.');
    } finally {
      setLoading(false);
    }
  }, [currentPage, pageSize, searchQuery, selectedType]);

  useEffect(() => {
    fetchGraphs();
  }, [fetchGraphs]);

  // Sync state to URL params
  useEffect(() => {
    const params: Record<string, string> = {};
    if (searchQuery) params.query = searchQuery;
    if (selectedType) params.graph_type = selectedType;
    if (currentPage > 1) params.page = String(currentPage);
    if (pageSize !== 20) params.page_size = String(pageSize);
    setSearchParams(params, { replace: true });
  }, [searchQuery, selectedType, currentPage, pageSize, setSearchParams]);

  const handleSearch = (value: string) => {
    setSearchQuery(value);
    setCurrentPage(1);
  };

  const handleTypeChange = (value: string | undefined) => {
    setSelectedType(value || undefined);
    setCurrentPage(1);
  };

  const handleTableChange = (pagination: TablePaginationConfig) => {
    setCurrentPage(pagination.current || 1);
    setPageSize(pagination.pageSize || 20);
  };

  const handleCreateGraph = async (values: CreateLineageGraphRequest) => {
    setCreating(true);
    try {
      const response = await lineageApi.createGraph({
        graph_name: values.graph_name,
        graph_type: values.graph_type,
        description: values.description || undefined,
      });
      message.success('Lineage graph created successfully.');
      setCreateModalOpen(false);
      form.resetFields();
      navigate(`/lineage/${response.data.graph_id}`);
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { message?: string }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.message || 'Validation error.');
      } else {
        message.error('Failed to create lineage graph.');
      }
    } finally {
      setCreating(false);
    }
  };

  const columns = [
    {
      title: 'Graph Name',
      dataIndex: 'graph_name',
      key: 'graph_name',
      render: (name: string, record: LineageGraphListItem) => (
        <a onClick={() => navigate(`/lineage/${record.graph_id}`)}>
          <Space>
            <ApartmentOutlined />
            {name}
          </Space>
        </a>
      ),
    },
    {
      title: 'Type',
      dataIndex: 'graph_type',
      key: 'graph_type',
      width: 130,
      render: (type: string) => (
        <Tag color={graphTypeColors[type] || 'default'}>
          {graphTypeLabels[type] || type}
        </Tag>
      ),
    },
    {
      title: 'Nodes',
      dataIndex: 'node_count',
      key: 'node_count',
      width: 90,
      align: 'center' as const,
      render: (count: number) => count || 0,
    },
    {
      title: 'Edges',
      dataIndex: 'edge_count',
      key: 'edge_count',
      width: 90,
      align: 'center' as const,
      render: (count: number) => count || 0,
    },
    {
      title: 'Current',
      dataIndex: 'is_current',
      key: 'is_current',
      width: 90,
      align: 'center' as const,
      render: (isCurrent: boolean) =>
        isCurrent ? (
          <Tag color="success">Yes</Tag>
        ) : (
          <Tag color="default">No</Tag>
        ),
    },
    {
      title: 'Description',
      dataIndex: 'description',
      key: 'description',
      ellipsis: true,
      render: (desc: string | null) => (
        <Text type="secondary" ellipsis>
          {desc || '-'}
        </Text>
      ),
    },
    {
      title: 'Created By',
      dataIndex: 'created_by_name',
      key: 'created_by_name',
      width: 150,
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
          Data Lineage
        </Title>
        <Button
          type="primary"
          icon={<PlusOutlined />}
          onClick={() => setCreateModalOpen(true)}
        >
          New Graph
        </Button>
      </div>

      <Card>
        <Space wrap style={{ marginBottom: 16, width: '100%' }}>
          <Input.Search
            placeholder="Search graphs..."
            style={{ width: 300 }}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onSearch={handleSearch}
            allowClear
          />
          <Select
            placeholder="Filter by type"
            style={{ width: 180 }}
            value={selectedType}
            onChange={handleTypeChange}
            options={graphTypeOptions}
            allowClear
          />
        </Space>
        <Table
          columns={columns}
          dataSource={graphs}
          rowKey="graph_id"
          loading={loading}
          onChange={handleTableChange}
          pagination={{
            current: currentPage,
            pageSize: pageSize,
            total: totalCount,
            showSizeChanger: true,
            pageSizeOptions: ['10', '20', '50'],
            showTotal: (total, range) => `${range[0]}-${range[1]} of ${total} graphs`,
          }}
        />
      </Card>

      <Modal
        title="Create Lineage Graph"
        open={createModalOpen}
        onCancel={() => {
          setCreateModalOpen(false);
          form.resetFields();
        }}
        footer={null}
        destroyOnClose
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleCreateGraph}
          style={{ marginTop: 16 }}
        >
          <Form.Item
            name="graph_name"
            label="Graph Name"
            rules={[
              { required: true, message: 'Graph name is required' },
              { max: 256, message: 'Graph name cannot exceed 256 characters' },
            ]}
          >
            <Input placeholder="e.g., Customer Data Flow" />
          </Form.Item>
          <Form.Item
            name="graph_type"
            label="Graph Type"
            rules={[{ required: true, message: 'Graph type is required' }]}
          >
            <Select placeholder="Select type" options={graphTypeOptions} />
          </Form.Item>
          <Form.Item name="description" label="Description">
            <TextArea
              rows={3}
              placeholder="Describe the scope and purpose of this lineage graph"
            />
          </Form.Item>
          <Form.Item style={{ marginBottom: 0, textAlign: 'right' }}>
            <Space>
              <Button
                onClick={() => {
                  setCreateModalOpen(false);
                  form.resetFields();
                }}
              >
                Cancel
              </Button>
              <Button type="primary" htmlType="submit" loading={creating}>
                Create Graph
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default LineageGraphList;

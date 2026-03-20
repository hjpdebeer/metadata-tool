import React, { useState, useEffect, useCallback } from 'react';
import {
  Alert,
  Button,
  DatePicker,
  Form,
  Input,
  Modal,
  Popconfirm,
  Select,
  Space,
  Table,
  Tag,
  Tooltip,
  Typography,
  message,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import {
  CopyOutlined,
  KeyOutlined,
  PlusOutlined,
  StopOutlined,
} from '@ant-design/icons';
import type { Dayjs } from 'dayjs';
import api from '../../services/api';

const { Title, Text } = Typography;

// ---------------------------------------------------------------------------
// Types matching backend responses
// ---------------------------------------------------------------------------

interface ApiKeyListItem {
  key_id: string;
  key_name: string;
  key_prefix: string;
  scopes: string[];
  is_active: boolean;
  last_used_at: string | null;
  created_at: string;
  expires_at: string | null;
  created_by_name: string | null;
}

interface ApiKeyListResponse {
  api_keys: ApiKeyListItem[];
}

interface CreateApiKeyResponse {
  key_id: string;
  key_name: string;
  api_key: string;
  key_prefix: string;
  scopes: string[];
  expires_at: string | null;
}

// ---------------------------------------------------------------------------
// Scope options
// ---------------------------------------------------------------------------

const SCOPE_OPTIONS = [
  { label: 'Ingest: Technical Metadata', value: 'ingest:technical' },
  { label: 'Ingest: Data Elements', value: 'ingest:elements' },
  { label: 'Read: All', value: 'read:all' },
  { label: 'Read: Technical Metadata', value: 'read:technical' },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function relativeTime(dateStr: string | null): string {
  if (!dateStr) return 'Never';
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();

  if (diffMs < 0) return 'In the future';

  const seconds = Math.floor(diffMs / 1000);
  if (seconds < 60) return 'Just now';

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} minute${minutes === 1 ? '' : 's'} ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hour${hours === 1 ? '' : 's'} ago`;

  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} day${days === 1 ? '' : 's'} ago`;

  const months = Math.floor(days / 30);
  if (months < 12) return `${months} month${months === 1 ? '' : 's'} ago`;

  const years = Math.floor(months / 12);
  return `${years} year${years === 1 ? '' : 's'} ago`;
}

function scopeColor(scope: string): string {
  if (scope.startsWith('ingest:')) return 'orange';
  if (scope === 'read:all') return 'blue';
  if (scope.startsWith('read:')) return 'green';
  return 'default';
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const AdminApiKeys: React.FC = () => {
  const [keys, setKeys] = useState<ApiKeyListItem[]>([]);
  const [loading, setLoading] = useState(false);

  // Create modal
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [createLoading, setCreateLoading] = useState(false);
  const [form] = Form.useForm();

  // Generated key display
  const [generatedKey, setGeneratedKey] = useState<string | null>(null);

  const fetchKeys = useCallback(async () => {
    setLoading(true);
    try {
      const response = await api.get<ApiKeyListResponse>('/admin/api-keys');
      setKeys(response.data.api_keys);
    } catch {
      message.error('Failed to load API keys');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchKeys();
  }, [fetchKeys]);

  const handleCreate = async () => {
    try {
      const values = await form.validateFields();
      setCreateLoading(true);

      const payload: {
        key_name: string;
        scopes: string[];
        expires_at?: string;
      } = {
        key_name: values.key_name.trim(),
        scopes: values.scopes,
      };

      if (values.expires_at) {
        payload.expires_at = (values.expires_at as Dayjs).toISOString();
      }

      const response = await api.post<CreateApiKeyResponse>('/admin/api-keys', payload);
      setGeneratedKey(response.data.api_key);
      form.resetFields();
      fetchKeys();
    } catch (err: unknown) {
      const error = err as { response?: { data?: { error?: { message?: string } } } };
      if (error?.response?.data?.error?.message) {
        message.error(error.response.data.error.message);
      }
    } finally {
      setCreateLoading(false);
    }
  };

  const handleDeactivate = async (keyId: string) => {
    try {
      await api.delete(`/admin/api-keys/${keyId}`);
      message.success('API key deactivated');
      fetchKeys();
    } catch (err: unknown) {
      const error = err as { response?: { data?: { error?: { message?: string } } } };
      message.error(error?.response?.data?.error?.message || 'Failed to deactivate key');
    }
  };

  const handleCopyKey = async () => {
    if (!generatedKey) return;
    try {
      await navigator.clipboard.writeText(generatedKey);
      message.success('API key copied to clipboard');
    } catch {
      message.error('Failed to copy to clipboard');
    }
  };

  const handleCloseCreateModal = () => {
    setCreateModalOpen(false);
    setGeneratedKey(null);
    form.resetFields();
  };

  const columns: ColumnsType<ApiKeyListItem> = [
    {
      title: 'Name',
      dataIndex: 'key_name',
      key: 'key_name',
      render: (name: string) => <Text strong>{name}</Text>,
    },
    {
      title: 'Prefix',
      dataIndex: 'key_prefix',
      key: 'key_prefix',
      width: 120,
      render: (prefix: string) => (
        <Text code style={{ fontSize: 12 }}>
          {prefix}...
        </Text>
      ),
    },
    {
      title: 'Scopes',
      dataIndex: 'scopes',
      key: 'scopes',
      width: 240,
      render: (scopes: string[]) => (
        <Space size={[0, 4]} wrap>
          {scopes.map((scope) => (
            <Tag key={scope} color={scopeColor(scope)} style={{ fontSize: 11 }}>
              {scope}
            </Tag>
          ))}
        </Space>
      ),
    },
    {
      title: 'Status',
      dataIndex: 'is_active',
      key: 'is_active',
      width: 100,
      render: (active: boolean, record: ApiKeyListItem) => {
        if (!active) {
          return <Tag color="default">Inactive</Tag>;
        }
        if (record.expires_at && new Date(record.expires_at) < new Date()) {
          return <Tag color="red">Expired</Tag>;
        }
        return <Tag color="green">Active</Tag>;
      },
    },
    {
      title: 'Last Used',
      dataIndex: 'last_used_at',
      key: 'last_used_at',
      width: 150,
      render: (val: string | null) => (
        <Tooltip title={val ? new Date(val).toLocaleString() : undefined}>
          <Text type="secondary">{relativeTime(val)}</Text>
        </Tooltip>
      ),
    },
    {
      title: 'Created',
      dataIndex: 'created_at',
      key: 'created_at',
      width: 150,
      render: (val: string, record: ApiKeyListItem) => (
        <Tooltip title={record.created_by_name ? `By ${record.created_by_name}` : undefined}>
          <Text type="secondary">{new Date(val).toLocaleDateString()}</Text>
        </Tooltip>
      ),
    },
    {
      title: 'Expires',
      dataIndex: 'expires_at',
      key: 'expires_at',
      width: 120,
      render: (val: string | null) => {
        if (!val) return <Text type="secondary">Never</Text>;
        const date = new Date(val);
        const isExpired = date < new Date();
        return (
          <Text type={isExpired ? 'danger' : 'secondary'}>
            {date.toLocaleDateString()}
          </Text>
        );
      },
    },
    {
      title: 'Actions',
      key: 'actions',
      width: 120,
      render: (_: unknown, record: ApiKeyListItem) =>
        record.is_active ? (
          <Popconfirm
            title="Deactivate this API key?"
            description="This action cannot be undone. Services using this key will lose access."
            onConfirm={() => handleDeactivate(record.key_id)}
            okText="Deactivate"
            okButtonProps={{ danger: true }}
          >
            <Button type="text" size="small" danger icon={<StopOutlined />}>
              Deactivate
            </Button>
          </Popconfirm>
        ) : (
          <Text type="secondary">-</Text>
        ),
    },
  ];

  return (
    <div>
      <Space style={{ marginBottom: 16, width: '100%', justifyContent: 'space-between' }}>
        <Title level={5} style={{ margin: 0 }}>
          API Keys
        </Title>
        <Button
          type="primary"
          icon={<PlusOutlined />}
          onClick={() => setCreateModalOpen(true)}
        >
          Generate New Key
        </Button>
      </Space>

      <Table<ApiKeyListItem>
        columns={columns}
        dataSource={keys}
        rowKey="key_id"
        loading={loading}
        size="small"
        pagination={false}
      />

      {/* Create API Key Modal */}
      <Modal
        title={
          <Space>
            <KeyOutlined />
            Generate New API Key
          </Space>
        }
        open={createModalOpen}
        onCancel={handleCloseCreateModal}
        footer={
          generatedKey
            ? [
                <Button key="close" onClick={handleCloseCreateModal}>
                  Close
                </Button>,
              ]
            : [
                <Button key="cancel" onClick={handleCloseCreateModal}>
                  Cancel
                </Button>,
                <Button
                  key="create"
                  type="primary"
                  loading={createLoading}
                  onClick={handleCreate}
                >
                  Generate Key
                </Button>,
              ]
        }
        destroyOnClose
        width={520}
      >
        {generatedKey ? (
          <div style={{ marginTop: 16 }}>
            <Alert
              message="API Key Generated"
              description="Copy this key now. It will not be shown again."
              type="warning"
              showIcon
              style={{ marginBottom: 16 }}
            />
            <div
              style={{
                background: '#f5f5f5',
                borderRadius: 6,
                padding: '12px 16px',
                display: 'flex',
                alignItems: 'center',
                gap: 8,
              }}
            >
              <Text
                code
                copyable={false}
                style={{
                  flex: 1,
                  wordBreak: 'break-all',
                  fontSize: 13,
                }}
              >
                {generatedKey}
              </Text>
              <Button
                type="primary"
                icon={<CopyOutlined />}
                onClick={handleCopyKey}
              >
                Copy
              </Button>
            </div>
          </div>
        ) : (
          <Form form={form} layout="vertical" style={{ marginTop: 16 }}>
            <Form.Item
              name="key_name"
              label="Key Name"
              rules={[
                { required: true, message: 'Please enter a name for this API key' },
                { max: 100, message: 'Name must be 100 characters or fewer' },
              ]}
            >
              <Input placeholder="e.g., Data Warehouse ETL" maxLength={100} />
            </Form.Item>

            <Form.Item
              name="scopes"
              label="Scopes"
              rules={[
                { required: true, message: 'Select at least one scope' },
              ]}
            >
              <Select
                mode="multiple"
                placeholder="Select permissions for this key"
                options={SCOPE_OPTIONS}
              />
            </Form.Item>

            <Form.Item
              name="expires_at"
              label="Expiry Date"
              help="Leave empty for a key that does not expire"
            >
              <DatePicker
                style={{ width: '100%' }}
                disabledDate={(current) => current && current.valueOf() < Date.now()}
              />
            </Form.Item>
          </Form>
        )}
      </Modal>
    </div>
  );
};

export default AdminApiKeys;

import React, { useState, useEffect, useCallback } from 'react';
import {
  Card,
  Table,
  Button,
  Modal,
  Input,
  Form,
  Space,
  Typography,
  Menu,
  message,
  Popconfirm,
  Spin,
} from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined, SearchOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import {
  adminApi,
  LOOKUP_TABLES,
  type LookupRow,
  type LookupTableMeta,
} from '../../services/adminApi';

const { Title, Text } = Typography;

const AdminLookupTables: React.FC = () => {
  const [selectedTable, setSelectedTable] = useState<LookupTableMeta>(LOOKUP_TABLES[0]);
  const [rows, setRows] = useState<LookupRow[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(50);

  // Modal state
  const [modalOpen, setModalOpen] = useState(false);
  const [editingRow, setEditingRow] = useState<LookupRow | null>(null);
  const [modalLoading, setModalLoading] = useState(false);
  const [form] = Form.useForm();

  const fetchRows = useCallback(async () => {
    setLoading(true);
    try {
      const response = await adminApi.listLookup(selectedTable.key, {
        search: search || undefined,
        page,
        page_size: pageSize,
      });
      setRows(response.data.data);
      setTotalCount(response.data.total_count);
    } catch {
      message.error('Failed to load lookup table data');
    } finally {
      setLoading(false);
    }
  }, [selectedTable.key, search, page, pageSize]);

  useEffect(() => {
    fetchRows();
  }, [fetchRows]);

  const handleTableSelect = (table: LookupTableMeta) => {
    setSelectedTable(table);
    setSearch('');
    setPage(1);
  };

  const handleAdd = () => {
    setEditingRow(null);
    form.resetFields();
    setModalOpen(true);
  };

  const handleEdit = (row: LookupRow) => {
    setEditingRow(row);
    form.setFieldsValue({
      code: row.code || '',
      name: row.name,
      description: row.description || '',
    });
    setModalOpen(true);
  };

  const handleDelete = async (row: LookupRow) => {
    try {
      // Check usage count first
      const usageResponse = await adminApi.getUsageCount(selectedTable.key, row.id);
      const count = usageResponse.data.usage_count;
      if (count > 0) {
        message.error(
          `Cannot delete "${row.name}": it is referenced by ${count} ${count === 1 ? 'entity' : 'entities'}.`,
        );
        return;
      }
      await adminApi.deleteLookup(selectedTable.key, row.id);
      message.success(`"${row.name}" deleted`);
      fetchRows();
    } catch (err: unknown) {
      const error = err as { response?: { data?: { error?: { message?: string } } } };
      message.error(error?.response?.data?.error?.message || 'Failed to delete');
    }
  };

  const handleModalOk = async () => {
    try {
      const values = await form.validateFields();
      setModalLoading(true);

      const data = {
        code: values.code || undefined,
        name: values.name,
        description: values.description || undefined,
      };

      if (editingRow) {
        await adminApi.updateLookup(selectedTable.key, editingRow.id, data);
        message.success(`"${values.name}" updated`);
      } else {
        await adminApi.createLookup(selectedTable.key, data);
        message.success(`"${values.name}" created`);
      }

      setModalOpen(false);
      form.resetFields();
      fetchRows();
    } catch (err: unknown) {
      const error = err as { response?: { data?: { error?: { message?: string } } } };
      if (error?.response?.data?.error?.message) {
        message.error(error.response.data.error.message);
      }
      // If it's a form validation error, the form will show inline errors
    } finally {
      setModalLoading(false);
    }
  };

  const columns: ColumnsType<LookupRow> = [
    ...(selectedTable.hasCode
      ? [
          {
            title: 'Code',
            dataIndex: 'code' as const,
            key: 'code',
            width: 150,
            render: (text: string | null) => (
              <Text code style={{ fontSize: 13 }}>
                {text || '-'}
              </Text>
            ),
          },
        ]
      : []),
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
    },
    {
      title: 'Description',
      dataIndex: 'description',
      key: 'description',
      ellipsis: true,
      render: (text: string | null) => text || '-',
    },
    {
      title: 'Actions',
      key: 'actions',
      width: 120,
      render: (_: unknown, record: LookupRow) => (
        <Space>
          <Button
            type="text"
            size="small"
            icon={<EditOutlined />}
            onClick={() => handleEdit(record)}
          />
          <Popconfirm
            title="Delete this item?"
            description="This action cannot be undone."
            onConfirm={() => handleDelete(record)}
            okText="Delete"
            okButtonProps={{ danger: true }}
          >
            <Button type="text" size="small" danger icon={<DeleteOutlined />} />
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const menuItems = LOOKUP_TABLES.map((t) => ({
    key: t.key,
    label: t.label,
  }));

  return (
    <div style={{ display: 'flex', gap: 16 }}>
      {/* Left sidebar */}
      <Card
        size="small"
        style={{ width: 220, flexShrink: 0 }}
        styles={{ body: { padding: 0 } }}
      >
        <Menu
          selectedKeys={[selectedTable.key]}
          items={menuItems}
          onClick={({ key }) => {
            const table = LOOKUP_TABLES.find((t) => t.key === key);
            if (table) handleTableSelect(table);
          }}
          style={{ border: 'none' }}
        />
      </Card>

      {/* Right content */}
      <Card style={{ flex: 1 }}>
        <Space
          style={{ marginBottom: 16, width: '100%', justifyContent: 'space-between' }}
        >
          <Title level={5} style={{ margin: 0 }}>
            {selectedTable.label}
          </Title>
          <Space>
            <Input
              placeholder="Search..."
              prefix={<SearchOutlined />}
              allowClear
              style={{ width: 250 }}
              value={search}
              onChange={(e) => {
                setSearch(e.target.value);
                setPage(1);
              }}
            />
            <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>
              Add New
            </Button>
          </Space>
        </Space>

        <Spin spinning={loading}>
          <Table<LookupRow>
            columns={columns}
            dataSource={rows}
            rowKey="id"
            size="small"
            pagination={{
              current: page,
              pageSize,
              total: totalCount,
              showSizeChanger: true,
              showTotal: (total) => `${total} items`,
              onChange: (p, ps) => {
                setPage(p);
                setPageSize(ps);
              },
            }}
          />
        </Spin>
      </Card>

      {/* Add/Edit Modal */}
      <Modal
        title={editingRow ? `Edit ${selectedTable.label.replace(/s$/, '')}` : `Add ${selectedTable.label.replace(/s$/, '')}`}
        open={modalOpen}
        onOk={handleModalOk}
        onCancel={() => {
          setModalOpen(false);
          form.resetFields();
        }}
        confirmLoading={modalLoading}
        destroyOnClose
      >
        <Form form={form} layout="vertical" style={{ marginTop: 16 }}>
          {selectedTable.hasCode && (
            <Form.Item
              name="code"
              label="Code"
              rules={[{ required: true, message: 'Code is required' }]}
            >
              <Input
                placeholder="e.g., ANALYTICS"
                maxLength={50}
                disabled={!!editingRow}
                style={{ textTransform: 'uppercase' }}
              />
            </Form.Item>
          )}
          <Form.Item
            name="name"
            label="Name"
            rules={[{ required: true, message: 'Name is required' }]}
          >
            <Input placeholder="Display name" maxLength={256} />
          </Form.Item>
          <Form.Item name="description" label="Description">
            <Input.TextArea rows={3} placeholder="Optional description" maxLength={2000} />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default AdminLookupTables;

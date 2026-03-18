import React, { useState, useEffect, useCallback } from 'react';
import {
  Card,
  Table,
  Input,
  Tag,
  Switch,
  Button,
  Modal,
  Select,
  Space,
  Typography,
  message,
} from 'antd';
import { SearchOutlined, PlusOutlined, DeleteOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { usersApi, type UserListItem, type Role, type UserWithRoles } from '../services/usersApi';

const { Title } = Typography;

const roleTagColors: Record<string, string> = {
  ADMIN: '#1B3A5C',
  DATA_STEWARD: '#2E7D32',
  DATA_OWNER: '#1565C0',
  ANALYST: '#6A1B9A',
  VIEWER: '#757575',
};

const UserManagementPage: React.FC = () => {
  const [users, setUsers] = useState<UserListItem[]>([]);
  const [roles, setRoles] = useState<Role[]>([]);
  const [loading, setLoading] = useState(false);
  const [totalCount, setTotalCount] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [searchQuery, setSearchQuery] = useState('');
  const [roleFilter, setRoleFilter] = useState<string | undefined>(undefined);

  // Detail modal state
  const [detailModalOpen, setDetailModalOpen] = useState(false);
  const [selectedUser, setSelectedUser] = useState<UserWithRoles | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [addRoleId, setAddRoleId] = useState<string | undefined>(undefined);

  const fetchUsers = useCallback(async () => {
    setLoading(true);
    try {
      const response = await usersApi.listUsers({
        query: searchQuery || undefined,
        role_code: roleFilter,
        page,
        page_size: pageSize,
      });
      setUsers(response.data.data);
      setTotalCount(response.data.total_count);
    } catch {
      message.error('Failed to load users');
    } finally {
      setLoading(false);
    }
  }, [searchQuery, roleFilter, page, pageSize]);

  const fetchRoles = useCallback(async () => {
    try {
      const response = await usersApi.listRoles();
      setRoles(response.data);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    fetchUsers();
  }, [fetchUsers]);

  useEffect(() => {
    fetchRoles();
  }, [fetchRoles]);

  const handleUserClick = async (userId: string) => {
    setDetailModalOpen(true);
    setDetailLoading(true);
    try {
      const response = await usersApi.getUser(userId);
      setSelectedUser(response.data);
    } catch {
      message.error('Failed to load user details');
      setDetailModalOpen(false);
    } finally {
      setDetailLoading(false);
    }
  };

  const handleToggleActive = async (userId: string, isActive: boolean) => {
    try {
      await usersApi.updateUser(userId, { is_active: isActive });
      message.success(`User ${isActive ? 'activated' : 'deactivated'}`);
      fetchUsers();
      if (selectedUser && selectedUser.user_id === userId) {
        setSelectedUser((prev) => (prev ? { ...prev, is_active: isActive } : prev));
      }
    } catch {
      message.error('Failed to update user');
    }
  };

  const handleAssignRole = async () => {
    if (!selectedUser || !addRoleId) return;
    try {
      await usersApi.assignRole(selectedUser.user_id, addRoleId);
      message.success('Role assigned');
      setAddRoleId(undefined);
      // Refresh user detail
      const response = await usersApi.getUser(selectedUser.user_id);
      setSelectedUser(response.data);
      fetchUsers();
    } catch (err: unknown) {
      const error = err as { response?: { data?: { error?: { message?: string } } } };
      message.error(error?.response?.data?.error?.message || 'Failed to assign role');
    }
  };

  const handleRemoveRole = async (roleId: string) => {
    if (!selectedUser) return;
    try {
      await usersApi.removeRole(selectedUser.user_id, roleId);
      message.success('Role removed');
      // Refresh user detail
      const response = await usersApi.getUser(selectedUser.user_id);
      setSelectedUser(response.data);
      fetchUsers();
    } catch {
      message.error('Failed to remove role');
    }
  };

  const formatDate = (dateStr: string | null): string => {
    if (!dateStr) return 'Never';
    return new Date(dateStr).toLocaleString();
  };

  const columns: ColumnsType<UserListItem> = [
    {
      title: 'Display Name',
      dataIndex: 'display_name',
      key: 'display_name',
      render: (text: string, record: UserListItem) => (
        <Button type="link" onClick={() => handleUserClick(record.user_id)} style={{ padding: 0 }}>
          {text}
        </Button>
      ),
    },
    {
      title: 'Email',
      dataIndex: 'email',
      key: 'email',
    },
    {
      title: 'Department',
      dataIndex: 'department',
      key: 'department',
      render: (text: string | null) => text || '-',
    },
    {
      title: 'Job Title',
      dataIndex: 'job_title',
      key: 'job_title',
      render: (text: string | null) => text || '-',
    },
    {
      title: 'Active',
      dataIndex: 'is_active',
      key: 'is_active',
      width: 80,
      render: (isActive: boolean, record: UserListItem) => (
        <Switch
          checked={isActive}
          size="small"
          onChange={(checked) => handleToggleActive(record.user_id, checked)}
        />
      ),
    },
    {
      title: 'Last Login',
      dataIndex: 'last_login_at',
      key: 'last_login_at',
      render: (text: string | null) => formatDate(text),
    },
  ];

  // Compute which roles can still be added to the selected user
  const availableRoles = selectedUser
    ? roles.filter((r) => !selectedUser.roles.some((ur) => ur.role_id === r.role_id))
    : [];

  return (
    <div>
      <Title level={3}>User Management</Title>

      <Card>
        <Space style={{ marginBottom: 16, width: '100%', justifyContent: 'space-between' }}>
          <Space>
            <Input
              placeholder="Search by name or email"
              prefix={<SearchOutlined />}
              allowClear
              style={{ width: 300 }}
              value={searchQuery}
              onChange={(e) => {
                setSearchQuery(e.target.value);
                setPage(1);
              }}
            />
            <Select
              placeholder="Filter by role"
              allowClear
              style={{ width: 200 }}
              value={roleFilter}
              onChange={(value) => {
                setRoleFilter(value);
                setPage(1);
              }}
              options={roles.map((r) => ({ value: r.role_code, label: r.role_name }))}
            />
          </Space>
        </Space>

        <Table<UserListItem>
          columns={columns}
          dataSource={users}
          rowKey="user_id"
          loading={loading}
          pagination={{
            current: page,
            pageSize,
            total: totalCount,
            showSizeChanger: true,
            showTotal: (total) => `${total} users`,
            onChange: (p, ps) => {
              setPage(p);
              setPageSize(ps);
            },
          }}
        />
      </Card>

      <Modal
        title={selectedUser ? `User: ${selectedUser.display_name}` : 'User Details'}
        open={detailModalOpen}
        onCancel={() => {
          setDetailModalOpen(false);
          setSelectedUser(null);
          setAddRoleId(undefined);
        }}
        footer={null}
        width={600}
        loading={detailLoading}
      >
        {selectedUser && (
          <div>
            <div style={{ marginBottom: 16 }}>
              <p>
                <strong>Email:</strong> {selectedUser.email}
              </p>
              <p>
                <strong>Username:</strong> {selectedUser.username}
              </p>
              <p>
                <strong>Department:</strong> {selectedUser.department || '-'}
              </p>
              <p>
                <strong>Job Title:</strong> {selectedUser.job_title || '-'}
              </p>
              <p>
                <strong>Status:</strong>{' '}
                <Switch
                  checked={selectedUser.is_active}
                  checkedChildren="Active"
                  unCheckedChildren="Inactive"
                  onChange={(checked) => handleToggleActive(selectedUser.user_id, checked)}
                />
              </p>
              <p>
                <strong>Last Login:</strong> {formatDate(selectedUser.last_login_at)}
              </p>
            </div>

            <div style={{ marginBottom: 16 }}>
              <Title level={5}>Roles</Title>
              <Space wrap style={{ marginBottom: 12 }}>
                {selectedUser.roles.length === 0 ? (
                  <span style={{ color: '#9CA3AF' }}>No roles assigned</span>
                ) : (
                  selectedUser.roles.map((role) => (
                    <Tag
                      key={role.role_id}
                      color={roleTagColors[role.role_code] || '#1B3A5C'}
                      closable
                      onClose={(e) => {
                        e.preventDefault();
                        handleRemoveRole(role.role_id);
                      }}
                      closeIcon={<DeleteOutlined />}
                    >
                      {role.role_name}
                    </Tag>
                  ))
                )}
              </Space>

              {availableRoles.length > 0 && (
                <Space>
                  <Select
                    placeholder="Select role to add"
                    style={{ width: 200 }}
                    value={addRoleId}
                    onChange={setAddRoleId}
                    options={availableRoles.map((r) => ({
                      value: r.role_id,
                      label: r.role_name,
                    }))}
                  />
                  <Button
                    type="primary"
                    icon={<PlusOutlined />}
                    onClick={handleAssignRole}
                    disabled={!addRoleId}
                  >
                    Assign
                  </Button>
                </Space>
              )}
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
};

export default UserManagementPage;

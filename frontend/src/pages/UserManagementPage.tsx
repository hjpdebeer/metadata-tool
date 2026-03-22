import React, { useState, useEffect, useCallback } from 'react';
import {
  Avatar,
  Card,
  Descriptions,
  Divider,
  Table,
  Input,
  Tag,
  Switch,
  Button,
  Modal,
  Select,
  Space,
  Typography,
  Alert,
  Badge,
  Tooltip,
  message,
} from 'antd';
import {
  SearchOutlined,
  PlusOutlined,
  DeleteOutlined,
  CheckCircleOutlined,
  SafetyCertificateOutlined,
  UserOutlined,
  MailOutlined,
  TeamOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import {
  usersApi,
  type UserListItem,
  type Role,
  type UserWithRoles,
} from '../services/usersApi';

const { Title, Text } = Typography;

const roleTagColors: Record<string, string> = {
  ADMIN: '#1B3A5C',
  DATA_STEWARD: '#2E7D32',
  DATA_OWNER: '#1565C0',
  DATA_PRODUCER: '#7B1FA2',
  DATA_CONSUMER: '#757575',
  APP_BUSINESS_OWNER: '#E65100',
  APP_TECHNICAL_OWNER: '#00838F',
  BUSINESS_PROCESS_OWNER: '#AD1457',
  VIEWER: '#9E9E9E',
};

/** Generate initials from display name */
const getInitials = (name: string): string => {
  const parts = name.trim().split(/\s+/);
  if (parts.length >= 2) return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
  return name.substring(0, 2).toUpperCase();
};

/** Generate a consistent color from a string */
const getAvatarColor = (name: string): string => {
  const colors = ['#1B3A5C', '#2E7D32', '#1565C0', '#7B1FA2', '#E65100', '#00838F', '#AD1457'];
  let hash = 0;
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
  return colors[Math.abs(hash) % colors.length];
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
  const [needsRoleFilter, setNeedsRoleFilter] = useState(false);

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
        needs_role_assignment: needsRoleFilter || undefined,
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
  }, [searchQuery, roleFilter, needsRoleFilter, page, pageSize]);

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
      const response = await usersApi.getUser(selectedUser.user_id);
      setSelectedUser(response.data);
      fetchUsers();
    } catch {
      message.error('Failed to remove role');
    }
  };

  const handleConfirmRoles = async () => {
    if (!selectedUser) return;
    try {
      await usersApi.confirmRoles(selectedUser.user_id);
      message.success('Roles confirmed');
      const response = await usersApi.getUser(selectedUser.user_id);
      setSelectedUser(response.data);
      fetchUsers();
    } catch {
      message.error('Failed to confirm roles');
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
        <Space>
          <Button type="link" onClick={() => handleUserClick(record.user_id)} style={{ padding: 0 }}>
            {text}
          </Button>
          {record.is_sso_user && (
            <Tooltip title="SSO User (Entra ID)">
              <SafetyCertificateOutlined style={{ color: '#1565C0', fontSize: 14 }} />
            </Tooltip>
          )}
          {!record.is_sso_user && (
            <Tooltip title="Dev-mode User">
              <UserOutlined style={{ color: '#9E9E9E', fontSize: 14 }} />
            </Tooltip>
          )}
        </Space>
      ),
    },
    {
      title: 'Email',
      dataIndex: 'email',
      key: 'email',
    },
    {
      title: 'Roles',
      key: 'roles',
      width: 280,
      render: (_: unknown, record: UserListItem) => {
        if (record.roles.length === 0) {
          return <Text type="secondary">No roles</Text>;
        }
        return (
          <Space wrap size={[4, 4]}>
            {record.roles.map((role) => (
              <Tag
                key={role.role_id}
                color={roleTagColors[role.role_code] || '#1B3A5C'}
                style={{ margin: 0 }}
              >
                {role.role_name}
              </Tag>
            ))}
          </Space>
        );
      },
    },
    {
      title: 'Department',
      dataIndex: 'department',
      key: 'department',
      responsive: ['lg'],
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
      responsive: ['xl'],
      render: (text: string | null) => formatDate(text),
    },
  ];

  // Compute which roles can still be added to the selected user
  const availableRoles = selectedUser
    ? roles.filter((r) => !selectedUser.roles.some((ur) => ur.role_id === r.role_id))
    : [];

  // Count users needing role assignment from current data (for badge)
  const needsAttentionCount = needsRoleFilter ? totalCount : undefined;

  return (
    <div>
      <Title level={3}>User Management</Title>

      <Card>
        <Space style={{ marginBottom: 16, width: '100%', justifyContent: 'space-between' }} wrap>
          <Space wrap>
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
            <Badge count={needsAttentionCount} size="small" offset={[-4, 0]}>
              <Button
                type={needsRoleFilter ? 'primary' : 'default'}
                danger={needsRoleFilter}
                onClick={() => {
                  setNeedsRoleFilter((prev) => !prev);
                  setRoleFilter(undefined);
                  setPage(1);
                }}
              >
                Needs Role Assignment
              </Button>
            </Badge>
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
        title={null}
        open={detailModalOpen}
        onCancel={() => {
          setDetailModalOpen(false);
          setSelectedUser(null);
          setAddRoleId(undefined);
        }}
        footer={null}
        width={640}
        loading={detailLoading}
        styles={{ body: { paddingTop: 8 } }}
      >
        {selectedUser && (
          <div>
            {/* Header: Avatar + Name + Status */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 16, marginBottom: 20 }}>
              <Avatar
                size={56}
                style={{ backgroundColor: getAvatarColor(selectedUser.display_name), flexShrink: 0 }}
              >
                {getInitials(selectedUser.display_name)}
              </Avatar>
              <div style={{ flex: 1 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Title level={4} style={{ margin: 0 }}>
                    {selectedUser.display_name}
                  </Title>
                  <Tag color={selectedUser.is_active ? 'success' : 'default'}>
                    {selectedUser.is_active ? 'Active' : 'Inactive'}
                  </Tag>
                </div>
                <Space size={16} style={{ marginTop: 4, color: '#6B7280' }}>
                  <span><MailOutlined style={{ marginRight: 4 }} />{selectedUser.email}</span>
                </Space>
              </div>
            </div>

            {/* Alert for unreviewed roles */}
            {!selectedUser.roles_reviewed && (
              <Alert
                message="Role assignment needs review"
                description="This user was auto-provisioned via SSO with the default Data Consumer role. Please review and confirm or update their roles."
                type="warning"
                showIcon
                style={{ marginBottom: 20 }}
              />
            )}

            {/* Profile Details */}
            <Descriptions
              column={2}
              size="small"
              bordered
              style={{ marginBottom: 20 }}
            >
              <Descriptions.Item label={<><TeamOutlined style={{ marginRight: 4 }} />Department</>}>
                {selectedUser.department || <Text type="secondary">Not set</Text>}
              </Descriptions.Item>
              <Descriptions.Item label="Job Title">
                {selectedUser.job_title || <Text type="secondary">Not set</Text>}
              </Descriptions.Item>
              <Descriptions.Item label={<><ClockCircleOutlined style={{ marginRight: 4 }} />Last Login</>}>
                {formatDate(selectedUser.last_login_at)}
              </Descriptions.Item>
              <Descriptions.Item label="Account Status">
                <Switch
                  checked={selectedUser.is_active}
                  checkedChildren="Active"
                  unCheckedChildren="Inactive"
                  onChange={(checked) => handleToggleActive(selectedUser.user_id, checked)}
                />
              </Descriptions.Item>
            </Descriptions>

            {/* Roles Section */}
            <Divider orientation="left" orientationMargin={0} style={{ marginBottom: 12, marginTop: 0 }}>
              Roles
            </Divider>

            <div style={{ marginBottom: 12 }}>
              {selectedUser.roles.length === 0 ? (
                <Text type="secondary">No roles assigned</Text>
              ) : (
                <Space wrap size={[6, 6]}>
                  {selectedUser.roles.map((role) => (
                    <Tag
                      key={role.role_id}
                      color={roleTagColors[role.role_code] || '#1B3A5C'}
                      closable
                      onClose={(e) => {
                        e.preventDefault();
                        handleRemoveRole(role.role_id);
                      }}
                      closeIcon={<DeleteOutlined />}
                      style={{ padding: '2px 8px' }}
                    >
                      {role.role_name}
                    </Tag>
                  ))}
                </Space>
              )}
            </div>

            {availableRoles.length > 0 && (
              <Space style={{ marginBottom: 12 }}>
                <Select
                  placeholder="Add a role..."
                  style={{ width: 220 }}
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

            {!selectedUser.roles_reviewed && (
              <div style={{ marginTop: 8 }}>
                <Button
                  icon={<CheckCircleOutlined />}
                  onClick={handleConfirmRoles}
                  style={{ borderColor: '#2E7D32', color: '#2E7D32' }}
                >
                  Confirm Roles as Correct
                </Button>
              </div>
            )}
          </div>
        )}
      </Modal>
    </div>
  );
};

export default UserManagementPage;

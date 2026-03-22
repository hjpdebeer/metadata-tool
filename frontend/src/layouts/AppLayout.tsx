import React, { useState, useEffect, useCallback } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import {
  Layout,
  Menu,
  Typography,
  Avatar,
  Descriptions,
  Divider,
  Dropdown,
  Modal,
  Space,
  Badge,
  Tag,
  Drawer,
  List,
  Button,
  Empty,
  message,
} from 'antd';
import {
  BookOutlined,
  DatabaseOutlined,
  FolderOpenOutlined,
  SafetyCertificateOutlined,
  ApartmentOutlined,
  AppstoreOutlined,
  PartitionOutlined,
  CheckSquareOutlined,
  UserOutlined,
  BellOutlined,
  SettingOutlined,
  LogoutOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  MailOutlined,
  TeamOutlined,
  ClockCircleOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import { useAuth } from '../hooks/useAuth';
import {
  notificationsApi,
  type InAppNotification,
} from '../services/notificationsApi';
import { workflowApi } from '../services/glossaryApi';
import { usersApi, type UserWithRoles } from '../services/usersApi';

const { Header, Sider, Content } = Layout;

const roleColors: Record<string, string> = {
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

const getInitials = (name: string): string => {
  const parts = name.trim().split(/\s+/);
  if (parts.length >= 2) return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
  return name.substring(0, 2).toUpperCase();
};

const getAvatarColor = (name: string): string => {
  const colors = ['#1B3A5C', '#2E7D32', '#1565C0', '#7B1FA2', '#E65100', '#00838F', '#AD1457'];
  let hash = 0;
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
  return colors[Math.abs(hash) % colors.length];
};

const AppLayout: React.FC = () => {
  const [collapsed, setCollapsed] = useState(false);
  const [notifDrawerOpen, setNotifDrawerOpen] = useState(false);
  const [unreadCount, setUnreadCount] = useState(0);
  const [pendingTaskCount, setPendingTaskCount] = useState(0);
  const [notifications, setNotifications] = useState<InAppNotification[]>([]);
  const [notifLoading, setNotifLoading] = useState(false);
  const [profileModalOpen, setProfileModalOpen] = useState(false);
  const [profileData, setProfileData] = useState<UserWithRoles | null>(null);
  const [profileLoading, setProfileLoading] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  const { user, logout } = useAuth();

  const isAdmin = user?.roles?.some((r) => r === 'ADMIN') ?? false;

  // Build menu items dynamically to include admin section when applicable
  const menuItems = [
    { key: '/glossary', icon: <BookOutlined />, label: 'Business Glossary' },
    {
      key: 'data-dictionary-group',
      icon: <DatabaseOutlined />,
      label: 'Data Dictionary',
      children: [
        { key: '/data-dictionary', icon: <DatabaseOutlined />, label: 'All Data Elements' },
        {
          key: '/data-dictionary/cde',
          icon: <WarningOutlined />,
          label: 'Critical Data Elements',
        },
        {
          key: '/data-quality/rules',
          icon: <CheckSquareOutlined />,
          label: 'Quality Rules',
        },
        {
          key: '/data-dictionary/technical',
          icon: <FolderOpenOutlined />,
          label: 'Technical Metadata',
        },
      ],
    },
    { key: '/data-quality', icon: <SafetyCertificateOutlined />, label: 'Data Quality Dashboard' },
    { key: '/lineage', icon: <ApartmentOutlined />, label: 'Data Lineage' },
    { key: '/applications', icon: <AppstoreOutlined />, label: 'Applications' },
    { key: '/processes', icon: <PartitionOutlined />, label: 'Business Processes' },
    { key: '/workflow', icon: <CheckSquareOutlined />, label: <span>My Tasks{pendingTaskCount > 0 && <Badge count={pendingTaskCount} size="small" offset={[6, -2]} />}</span> },
    ...(isAdmin
      ? [{ key: '/admin', icon: <SettingOutlined />, label: 'Admin Panel' }]
      : []),
  ];

  // Fetch unread count
  const fetchUnreadCount = useCallback(async () => {
    try {
      const response = await notificationsApi.getUnreadCount();
      setUnreadCount(response.data.count);
    } catch {
      // Silently ignore — user might not have notifications table yet
    }
  }, []);

  // Fetch pending task count
  const fetchPendingTaskCount = useCallback(async () => {
    try {
      const response = await workflowApi.getPendingTasks();
      setPendingTaskCount(response.data.length);
    } catch {
      // Silently ignore
    }
  }, []);

  // Fetch counts on mount and on navigation
  useEffect(() => {
    fetchUnreadCount();
    fetchPendingTaskCount();
  }, [fetchUnreadCount, fetchPendingTaskCount, location.pathname]);

  // Poll every 30 seconds
  useEffect(() => {
    const interval = setInterval(() => {
      fetchUnreadCount();
      fetchPendingTaskCount();
    }, 30000);
    return () => clearInterval(interval);
  }, [fetchUnreadCount, fetchPendingTaskCount]);

  // Fetch notifications when drawer opens
  const handleOpenNotifDrawer = async () => {
    setNotifDrawerOpen(true);
    setNotifLoading(true);
    try {
      const response = await notificationsApi.listNotifications({ page: 1, page_size: 50 });
      setNotifications(response.data.data);
    } catch {
      setNotifications([]);
    } finally {
      setNotifLoading(false);
    }
  };

  const handleMarkAllRead = async () => {
    try {
      await notificationsApi.markAllRead();
      setNotifications((prev) => prev.map((n) => ({ ...n, is_read: true })));
      setUnreadCount(0);
    } catch {
      // ignore
    }
  };

  const handleNotificationClick = async (notification: InAppNotification) => {
    if (!notification.is_read) {
      try {
        await notificationsApi.markRead(notification.notification_id);
        setNotifications((prev) =>
          prev.map((n) =>
            n.notification_id === notification.notification_id ? { ...n, is_read: true } : n,
          ),
        );
        setUnreadCount((prev) => Math.max(0, prev - 1));
      } catch {
        // ignore
      }
    }
    if (notification.link_url) {
      setNotifDrawerOpen(false);
      navigate(notification.link_url);
    }
  };

  const formatTimeAgo = (dateStr: string): string => {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return 'Just now';
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHours = Math.floor(diffMin / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    const diffDays = Math.floor(diffHours / 24);
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  // Derive selected key from current path
  const getSelectedKey = () => {
    const path = location.pathname;
    if (path === '/data-dictionary/cde') return '/data-dictionary/cde';
    if (path === '/data-dictionary/technical') return '/data-dictionary/technical';
    if (path.startsWith('/data-quality/rules')) return '/data-quality/rules';
    if (path.startsWith('/data-dictionary')) return '/data-dictionary';
    if (path.startsWith('/data-quality')) return '/data-quality';
    if (path.startsWith('/lineage')) return '/lineage';
    if (path.startsWith('/applications')) return '/applications';
    if (path.startsWith('/processes')) return '/processes';
    if (path.startsWith('/admin')) return '/admin';
    return path;
  };

  // Open sub-menus when on matching routes
  const getOpenKeys = () => {
    const keys: string[] = [];
    if (location.pathname.startsWith('/data-dictionary') || location.pathname.startsWith('/data-quality/rules')) keys.push('data-dictionary-group');
    // Admin is a single item now, no sub-menu to open
    return keys;
  };

  const handleOpenProfile = async () => {
    setProfileModalOpen(true);
    setProfileLoading(true);
    try {
      const response = await usersApi.getMyProfile();
      setProfileData(response.data);
    } catch {
      message.error('Failed to load profile');
      setProfileModalOpen(false);
    } finally {
      setProfileLoading(false);
    }
  };

  const handleMenuClick = ({ key }: { key: string }) => {
    if (key === 'logout') {
      logout();
      navigate('/login', { replace: true });
    } else if (key === 'profile') {
      handleOpenProfile();
    }
  };

  const formatProfileDate = (dateStr: string | null): string => {
    if (!dateStr) return 'Never';
    return new Date(dateStr).toLocaleString();
  };

  const userMenu = {
    items: [
      {
        key: 'user-info',
        label: (
          <div style={{ padding: '4px 0' }}>
            <div style={{ fontWeight: 500, color: '#1F2937' }}>
              {user?.display_name || 'User'}
            </div>
            <div style={{ fontSize: 12, color: '#6B7280' }}>{user?.email}</div>
            {user?.roles && user.roles.length > 0 && (
              <div style={{ marginTop: 6 }}>
                {user.roles.map((role) => (
                  <Tag
                    key={role}
                    color={roleColors[role] || '#1B3A5C'}
                    style={{ fontSize: 11, marginRight: 4 }}
                  >
                    {role.replace(/_/g, ' ')}
                  </Tag>
                ))}
              </div>
            )}
          </div>
        ),
        disabled: true,
      },
      { type: 'divider' as const },
      { key: 'profile', icon: <UserOutlined />, label: 'My Profile' },
      { type: 'divider' as const },
      { key: 'logout', icon: <LogoutOutlined />, label: 'Sign Out', danger: true },
    ],
    onClick: handleMenuClick,
  };

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider
        collapsible
        collapsed={collapsed}
        trigger={null}
        width={260}
        style={{
          borderRight: '1px solid #E5E7EB',
          overflow: 'auto',
          height: '100vh',
          position: 'fixed',
          left: 0,
          top: 0,
          bottom: 0,
        }}
      >
        <div
          style={{
            height: 64,
            display: 'flex',
            alignItems: 'center',
            justifyContent: collapsed ? 'center' : 'flex-start',
            padding: collapsed ? '0' : '0 20px',
            borderBottom: '1px solid #E5E7EB',
          }}
        >
          <DatabaseOutlined style={{ fontSize: 24, color: '#1B3A5C' }} />
          {!collapsed && (
            <Typography.Title
              level={5}
              style={{ margin: '0 0 0 12px', color: '#1B3A5C', whiteSpace: 'nowrap' }}
            >
              Metadata Tool
            </Typography.Title>
          )}
        </div>
        <Menu
          mode="inline"
          selectedKeys={[getSelectedKey()]}
          defaultOpenKeys={getOpenKeys()}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
          style={{ borderRight: 0, marginTop: 8 }}
        />
      </Sider>
      <Layout style={{ marginLeft: collapsed ? 80 : 260, transition: 'margin-left 0.2s' }}>
        <Header
          style={{
            padding: '0 24px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            borderBottom: '1px solid #E5E7EB',
            background: '#FFFFFF',
          }}
        >
          <Space>
            {React.createElement(collapsed ? MenuUnfoldOutlined : MenuFoldOutlined, {
              onClick: () => setCollapsed(!collapsed),
              style: { fontSize: 18, cursor: 'pointer' },
            })}
          </Space>
          <Space size="large">
            <Badge count={unreadCount} showZero={false}>
              <BellOutlined
                style={{ fontSize: 18, cursor: 'pointer' }}
                onClick={handleOpenNotifDrawer}
              />
            </Badge>
            <Dropdown menu={userMenu} placement="bottomRight" trigger={['click']}>
              <Space style={{ cursor: 'pointer' }}>
                <Avatar
                  size="small"
                  icon={<UserOutlined />}
                  style={{ backgroundColor: '#1B3A5C' }}
                />
                <span style={{ color: '#1F2937', fontSize: 14 }}>
                  {user?.display_name || 'User'}
                </span>
              </Space>
            </Dropdown>
          </Space>
        </Header>
        <Content style={{ margin: 24 }}>
          <Outlet />
        </Content>
      </Layout>

      <Drawer
        title="Notifications"
        placement="right"
        onClose={() => setNotifDrawerOpen(false)}
        open={notifDrawerOpen}
        width={400}
        extra={
          unreadCount > 0 ? (
            <Button type="link" size="small" onClick={handleMarkAllRead}>
              Mark all as read
            </Button>
          ) : null
        }
      >
        {notifications.length === 0 && !notifLoading ? (
          <Empty description="No notifications" />
        ) : (
          <List
            loading={notifLoading}
            dataSource={notifications}
            renderItem={(item) => (
              <List.Item
                onClick={() => handleNotificationClick(item)}
                style={{
                  cursor: item.link_url ? 'pointer' : 'default',
                  backgroundColor: item.is_read ? 'transparent' : '#F0F5FF',
                  padding: '12px 16px',
                  borderRadius: 6,
                  marginBottom: 4,
                }}
              >
                <List.Item.Meta
                  title={
                    <span
                      style={{
                        fontWeight: item.is_read ? 400 : 600,
                        fontSize: 14,
                      }}
                    >
                      {item.title}
                    </span>
                  }
                  description={
                    <div>
                      <div style={{ fontSize: 13, color: '#4B5563', marginBottom: 4 }}>
                        {item.message}
                      </div>
                      <div style={{ fontSize: 12, color: '#9CA3AF' }}>
                        {formatTimeAgo(item.created_at)}
                      </div>
                    </div>
                  }
                />
              </List.Item>
            )}
          />
        )}
      </Drawer>

      {/* Read-only profile modal */}
      <Modal
        title={null}
        open={profileModalOpen}
        onCancel={() => {
          setProfileModalOpen(false);
          setProfileData(null);
        }}
        footer={null}
        width={580}
        loading={profileLoading}
        styles={{ body: { paddingTop: 8 } }}
      >
        {profileData && (
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 16, marginBottom: 20 }}>
              <Avatar
                size={56}
                style={{ backgroundColor: getAvatarColor(profileData.display_name), flexShrink: 0 }}
              >
                {getInitials(profileData.display_name)}
              </Avatar>
              <div style={{ flex: 1 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Typography.Title level={4} style={{ margin: 0 }}>
                    {profileData.display_name}
                  </Typography.Title>
                  <Tag color={profileData.is_active ? 'success' : 'default'}>
                    {profileData.is_active ? 'Active' : 'Inactive'}
                  </Tag>
                </div>
                <Space size={16} style={{ marginTop: 4, color: '#6B7280' }}>
                  <span><MailOutlined style={{ marginRight: 4 }} />{profileData.email}</span>
                </Space>
              </div>
            </div>

            <Descriptions column={2} size="small" bordered style={{ marginBottom: 20 }}>
              <Descriptions.Item label={<><TeamOutlined style={{ marginRight: 4 }} />Department</>}>
                {profileData.department || <Typography.Text type="secondary">Not set</Typography.Text>}
              </Descriptions.Item>
              <Descriptions.Item label="Job Title">
                {profileData.job_title || <Typography.Text type="secondary">Not set</Typography.Text>}
              </Descriptions.Item>
              <Descriptions.Item label={<><ClockCircleOutlined style={{ marginRight: 4 }} />Last Login</>}>
                {formatProfileDate(profileData.last_login_at)}
              </Descriptions.Item>
              <Descriptions.Item label="Member Since">
                {new Date(profileData.created_at).toLocaleDateString()}
              </Descriptions.Item>
            </Descriptions>

            <Divider orientation="left" orientationMargin={0} style={{ marginBottom: 12, marginTop: 0 }}>
              Roles
            </Divider>
            <div>
              {profileData.roles.length === 0 ? (
                <Typography.Text type="secondary">No roles assigned</Typography.Text>
              ) : (
                <Space wrap size={[6, 6]}>
                  {profileData.roles.map((role) => (
                    <Tag
                      key={role.role_id}
                      color={roleColors[role.role_code] || '#1B3A5C'}
                      style={{ padding: '2px 8px' }}
                    >
                      {role.role_name}
                    </Tag>
                  ))}
                </Space>
              )}
            </div>

            <div style={{ marginTop: 20, padding: '12px 16px', background: '#F9FAFB', borderRadius: 6, fontSize: 13, color: '#6B7280' }}>
              To update your profile or roles, please contact your administrator.
            </div>
          </div>
        )}
      </Modal>
    </Layout>
  );
};

export default AppLayout;

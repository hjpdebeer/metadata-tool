import React, { useState, useEffect, useCallback } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import {
  Layout,
  Menu,
  Typography,
  Avatar,
  Dropdown,
  Space,
  Badge,
  Tag,
  Drawer,
  List,
  Button,
  Empty,
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
  WarningOutlined,
} from '@ant-design/icons';
import { useAuth } from '../hooks/useAuth';
import {
  notificationsApi,
  type InAppNotification,
} from '../services/notificationsApi';
import { workflowApi } from '../services/glossaryApi';

const { Header, Sider, Content } = Layout;

const roleColors: Record<string, string> = {
  admin: '#1B3A5C',
  data_steward: '#2E7D32',
  data_owner: '#1565C0',
  analyst: '#6A1B9A',
  viewer: '#757575',
};

const AppLayout: React.FC = () => {
  const [collapsed, setCollapsed] = useState(false);
  const [notifDrawerOpen, setNotifDrawerOpen] = useState(false);
  const [unreadCount, setUnreadCount] = useState(0);
  const [pendingTaskCount, setPendingTaskCount] = useState(0);
  const [notifications, setNotifications] = useState<InAppNotification[]>([]);
  const [notifLoading, setNotifLoading] = useState(false);
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

  const handleMenuClick = ({ key }: { key: string }) => {
    if (key === 'logout') {
      logout();
      navigate('/login', { replace: true });
    } else if (key === 'profile') {
      // Profile page not yet implemented
    } else if (key === 'settings') {
      // Settings page not yet implemented
    }
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
      { key: 'profile', icon: <UserOutlined />, label: 'Profile' },
      { key: 'settings', icon: <SettingOutlined />, label: 'Settings' },
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
    </Layout>
  );
};

export default AppLayout;

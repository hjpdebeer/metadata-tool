import React, { useState } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, Typography, Avatar, Dropdown, Space, Badge, Tag } from 'antd';
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

const { Header, Sider, Content } = Layout;

const menuItems = [
  { key: '/glossary', icon: <BookOutlined />, label: 'Business Glossary' },
  {
    key: 'data-dictionary-group',
    icon: <DatabaseOutlined />,
    label: 'Data Dictionary',
    children: [
      { key: '/data-dictionary', icon: <DatabaseOutlined />, label: 'All Elements' },
      { key: '/data-dictionary/cde', icon: <WarningOutlined />, label: 'Critical Data Elements' },
      { key: '/data-dictionary/technical', icon: <FolderOpenOutlined />, label: 'Technical Metadata' },
    ],
  },
  {
    key: 'data-quality-group',
    icon: <SafetyCertificateOutlined />,
    label: 'Data Quality',
    children: [
      { key: '/data-quality', icon: <SafetyCertificateOutlined />, label: 'Overview' },
      { key: '/data-quality/rules', icon: <CheckSquareOutlined />, label: 'Quality Rules' },
    ],
  },
  { key: '/lineage', icon: <ApartmentOutlined />, label: 'Data Lineage' },
  { key: '/applications', icon: <AppstoreOutlined />, label: 'Applications' },
  { key: '/processes', icon: <PartitionOutlined />, label: 'Business Processes' },
  { key: '/workflow', icon: <CheckSquareOutlined />, label: 'My Tasks' },
];

const roleColors: Record<string, string> = {
  admin: '#1B3A5C',
  data_steward: '#2E7D32',
  data_owner: '#1565C0',
  analyst: '#6A1B9A',
  viewer: '#757575',
};

const AppLayout: React.FC = () => {
  const [collapsed, setCollapsed] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  const { user, logout } = useAuth();

  // Derive selected key from current path
  const getSelectedKey = () => {
    const path = location.pathname;
    if (path === '/data-dictionary/cde') return '/data-dictionary/cde';
    if (path === '/data-dictionary/technical') return '/data-dictionary/technical';
    if (path.startsWith('/data-dictionary')) return '/data-dictionary';
    if (path.startsWith('/data-quality/rules')) return '/data-quality/rules';
    if (path.startsWith('/data-quality')) return '/data-quality';
    if (path.startsWith('/lineage')) return '/lineage';
    if (path.startsWith('/applications')) return '/applications';
    if (path.startsWith('/processes')) return '/processes';
    return path;
  };

  // Open sub-menus when on matching routes
  const getOpenKeys = () => {
    const keys: string[] = [];
    if (location.pathname.startsWith('/data-dictionary')) keys.push('data-dictionary-group');
    if (location.pathname.startsWith('/data-quality')) keys.push('data-quality-group');
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
                {user.roles.map(role => (
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
        width={240}
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
      <Layout style={{ marginLeft: collapsed ? 80 : 240, transition: 'margin-left 0.2s' }}>
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
            <Badge count={0} showZero={false}>
              <BellOutlined style={{ fontSize: 18, cursor: 'pointer' }} />
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
    </Layout>
  );
};

export default AppLayout;

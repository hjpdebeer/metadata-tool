import React, { useState } from 'react';
import { Tabs, Typography } from 'antd';
import {
  TableOutlined,
  SettingOutlined,
  TeamOutlined,
  BellOutlined,
} from '@ant-design/icons';
import AdminLookupTables from './admin/AdminLookupTables';
import AdminSystemConfig from './admin/AdminSystemConfig';
import UserManagementPage from './UserManagementPage';
import NotificationPreferencesPage from './NotificationPreferencesPage';

const { Title } = Typography;

const AdminPanel: React.FC = () => {
  const [activeKey, setActiveKey] = useState('lookup-tables');

  const tabItems = [
    {
      key: 'lookup-tables',
      label: (
        <span>
          <TableOutlined /> Lookup Tables
        </span>
      ),
      children: <AdminLookupTables />,
    },
    {
      key: 'system-config',
      label: (
        <span>
          <SettingOutlined /> System Configuration
        </span>
      ),
      children: <AdminSystemConfig />,
    },
    {
      key: 'users',
      label: (
        <span>
          <TeamOutlined /> Users
        </span>
      ),
      children: <UserManagementPage />,
    },
    {
      key: 'notifications',
      label: (
        <span>
          <BellOutlined /> Notifications
        </span>
      ),
      children: <NotificationPreferencesPage />,
    },
  ];

  return (
    <div>
      <Title level={3} style={{ marginBottom: 24 }}>
        Admin Panel
      </Title>
      <Tabs
        activeKey={activeKey}
        onChange={setActiveKey}
        tabPosition="left"
        items={tabItems}
        style={{ minHeight: 600 }}
      />
    </div>
  );
};

export default AdminPanel;

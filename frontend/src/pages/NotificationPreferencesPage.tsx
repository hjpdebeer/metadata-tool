import React, { useState, useEffect, useCallback } from 'react';
import { Card, Table, Switch, Button, Typography, message, Space } from 'antd';
import { SaveOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import {
  notificationsApi,
  type NotificationPreference,
  type UpdatePreferenceItem,
} from '../services/notificationsApi';

const { Title, Text } = Typography;

// Known event types that match the notification templates seeded in migration 010
const DEFAULT_EVENT_TYPES = [
  {
    event_type: 'WORKFLOW_TASK_ASSIGNED',
    label: 'Task Assigned',
    description: 'When a new review task is assigned to you',
  },
  {
    event_type: 'WORKFLOW_STATE_CHANGED',
    label: 'Status Changed',
    description: 'When an entity you submitted changes status',
  },
  {
    event_type: 'WORKFLOW_SLA_WARNING',
    label: 'SLA Warning',
    description: 'When a review task is overdue',
  },
];

interface PreferenceRow {
  event_type: string;
  label: string;
  description: string;
  email_enabled: boolean;
  in_app_enabled: boolean;
}

const NotificationPreferencesPage: React.FC = () => {
  const [preferences, setPreferences] = useState<PreferenceRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);

  const loadPreferences = useCallback(async () => {
    setLoading(true);
    try {
      const response = await notificationsApi.getPreferences();
      const existing = response.data;

      // Merge existing preferences with defaults
      const rows: PreferenceRow[] = DEFAULT_EVENT_TYPES.map((def) => {
        const pref = existing.find(
          (p: NotificationPreference) => p.event_type === def.event_type,
        );
        return {
          event_type: def.event_type,
          label: def.label,
          description: def.description,
          email_enabled: pref ? pref.email_enabled : true,
          in_app_enabled: pref ? pref.in_app_enabled : true,
        };
      });

      setPreferences(rows);
      setHasChanges(false);
    } catch {
      message.error('Failed to load notification preferences');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadPreferences();
  }, [loadPreferences]);

  const handleToggle = (
    eventType: string,
    field: 'email_enabled' | 'in_app_enabled',
    value: boolean,
  ) => {
    setPreferences((prev) =>
      prev.map((p) => (p.event_type === eventType ? { ...p, [field]: value } : p)),
    );
    setHasChanges(true);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const items: UpdatePreferenceItem[] = preferences.map((p) => ({
        event_type: p.event_type,
        email_enabled: p.email_enabled,
        in_app_enabled: p.in_app_enabled,
      }));
      await notificationsApi.updatePreferences(items);
      message.success('Preferences saved');
      setHasChanges(false);
    } catch {
      message.error('Failed to save preferences');
    } finally {
      setSaving(false);
    }
  };

  const columns: ColumnsType<PreferenceRow> = [
    {
      title: 'Event Type',
      key: 'event_type',
      render: (_: unknown, record: PreferenceRow) => (
        <div>
          <div style={{ fontWeight: 500 }}>{record.label}</div>
          <Text type="secondary" style={{ fontSize: 13 }}>
            {record.description}
          </Text>
        </div>
      ),
    },
    {
      title: 'Email',
      dataIndex: 'email_enabled',
      key: 'email_enabled',
      width: 100,
      align: 'center',
      render: (value: boolean, record: PreferenceRow) => (
        <Switch
          checked={value}
          onChange={(checked) => handleToggle(record.event_type, 'email_enabled', checked)}
        />
      ),
    },
    {
      title: 'In-App',
      dataIndex: 'in_app_enabled',
      key: 'in_app_enabled',
      width: 100,
      align: 'center',
      render: (value: boolean, record: PreferenceRow) => (
        <Switch
          checked={value}
          onChange={(checked) => handleToggle(record.event_type, 'in_app_enabled', checked)}
        />
      ),
    },
  ];

  return (
    <div>
      <Space style={{ marginBottom: 16, width: '100%', justifyContent: 'space-between' }}>
        <Title level={3} style={{ margin: 0 }}>
          Notification Preferences
        </Title>
        <Button
          type="primary"
          icon={<SaveOutlined />}
          onClick={handleSave}
          loading={saving}
          disabled={!hasChanges}
        >
          Save Preferences
        </Button>
      </Space>

      <Card>
        <Text type="secondary" style={{ display: 'block', marginBottom: 16 }}>
          Configure how you receive notifications for different event types. Email notifications will
          be delivered when the email integration is configured.
        </Text>

        <Table<PreferenceRow>
          columns={columns}
          dataSource={preferences}
          rowKey="event_type"
          loading={loading}
          pagination={false}
        />
      </Card>
    </div>
  );
};

export default NotificationPreferencesPage;

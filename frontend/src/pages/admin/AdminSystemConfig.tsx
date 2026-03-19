import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  Button,
  Input,
  Space,
  Typography,
  message,
  Collapse,
  Descriptions,
  Modal,
  Tag,
  Spin,
} from 'antd';
import {
  EyeOutlined,
  EyeInvisibleOutlined,
  EditOutlined,
  ApiOutlined,
} from '@ant-design/icons';
import { adminApi, type SystemSetting } from '../../services/adminApi';

const { Text, Title } = Typography;

interface SettingsByCategory {
  [category: string]: SystemSetting[];
}

const CATEGORY_LABELS: Record<string, string> = {
  AI: 'AI Configuration',
  Auth: 'Authentication',
  Email: 'Email Notifications',
  App: 'Application',
};

const CATEGORY_ORDER = ['AI', 'Auth', 'Email', 'App'];

// Keys that support test connection
const TESTABLE_KEYS = ['anthropic_api_key', 'openai_api_key', 'graph_client_secret'];

const AdminSystemConfig: React.FC = () => {
  const [settings, setSettings] = useState<SystemSetting[]>([]);
  const [loading, setLoading] = useState(false);
  const [revealedKeys, setRevealedKeys] = useState<Set<string>>(new Set());
  const [revealedValues, setRevealedValues] = useState<Record<string, string>>({});
  const [testingKey, setTestingKey] = useState<string | null>(null);

  // Edit modal
  const [editModalOpen, setEditModalOpen] = useState(false);
  const [editingSetting, setEditingSetting] = useState<SystemSetting | null>(null);
  const [editValue, setEditValue] = useState('');
  const [editSaving, setEditSaving] = useState(false);

  // Reveal timers
  const revealTimers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});

  const fetchSettings = useCallback(async () => {
    setLoading(true);
    try {
      const response = await adminApi.listSettings();
      setSettings(response.data.settings);
    } catch {
      message.error('Failed to load system settings');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchSettings();
    return () => {
      // Clean up reveal timers
      Object.values(revealTimers.current).forEach(clearTimeout);
    };
  }, [fetchSettings]);

  const handleReveal = async (key: string) => {
    if (revealedKeys.has(key)) {
      // Hide it
      setRevealedKeys((prev) => {
        const next = new Set(prev);
        next.delete(key);
        return next;
      });
      setRevealedValues((prev) => {
        const next = { ...prev };
        delete next[key];
        return next;
      });
      if (revealTimers.current[key]) {
        clearTimeout(revealTimers.current[key]);
        delete revealTimers.current[key];
      }
      return;
    }

    try {
      const response = await adminApi.revealSetting(key);
      setRevealedValues((prev) => ({ ...prev, [key]: response.data.value }));
      setRevealedKeys((prev) => new Set([...prev, key]));

      // Auto-hide after 30 seconds
      revealTimers.current[key] = setTimeout(() => {
        setRevealedKeys((prev) => {
          const next = new Set(prev);
          next.delete(key);
          return next;
        });
        setRevealedValues((prev) => {
          const next = { ...prev };
          delete next[key];
          return next;
        });
        delete revealTimers.current[key];
      }, 30000);
    } catch {
      message.error('Failed to reveal setting');
    }
  };

  const handleEdit = (setting: SystemSetting) => {
    setEditingSetting(setting);
    setEditValue('');
    setEditModalOpen(true);
  };

  const handleEditSave = async () => {
    if (!editingSetting) return;
    setEditSaving(true);
    try {
      await adminApi.updateSetting(editingSetting.key, editValue);
      message.success(`${editingSetting.display_name} updated`);
      setEditModalOpen(false);
      setEditingSetting(null);
      // Clear revealed state for this key
      setRevealedKeys((prev) => {
        const next = new Set(prev);
        next.delete(editingSetting.key);
        return next;
      });
      fetchSettings();
    } catch (err: unknown) {
      const error = err as { response?: { data?: { error?: { message?: string } } } };
      message.error(error?.response?.data?.error?.message || 'Failed to update setting');
    } finally {
      setEditSaving(false);
    }
  };

  const handleTestConnection = async (key: string) => {
    setTestingKey(key);
    try {
      const response = await adminApi.testConnection(key);
      if (response.data.success) {
        message.success(response.data.message);
      } else {
        message.error(response.data.message);
      }
    } catch (err: unknown) {
      const error = err as { response?: { data?: { error?: { message?: string } } } };
      message.error(error?.response?.data?.error?.message || 'Test connection failed');
    } finally {
      setTestingKey(null);
    }
  };

  // Group settings by category
  const grouped: SettingsByCategory = {};
  for (const setting of settings) {
    if (!grouped[setting.category]) {
      grouped[setting.category] = [];
    }
    grouped[setting.category].push(setting);
  }

  const renderSettingValue = (setting: SystemSetting) => {
    if (setting.is_encrypted) {
      if (revealedKeys.has(setting.key)) {
        return (
          <Text code style={{ wordBreak: 'break-all' }}>
            {revealedValues[setting.key] || ''}
          </Text>
        );
      }
      if (!setting.is_set) {
        return <Text type="secondary">Not configured</Text>;
      }
      return (
        <Text code>
          {setting.value}
        </Text>
      );
    }
    if (!setting.is_set || !setting.value) {
      return <Text type="secondary">Not configured</Text>;
    }
    return <Text>{setting.value}</Text>;
  };

  const renderSettingActions = (setting: SystemSetting) => {
    const actions: React.ReactNode[] = [];

    if (setting.is_encrypted && setting.is_set) {
      actions.push(
        <Button
          key="reveal"
          size="small"
          icon={revealedKeys.has(setting.key) ? <EyeInvisibleOutlined /> : <EyeOutlined />}
          onClick={() => handleReveal(setting.key)}
        >
          {revealedKeys.has(setting.key) ? 'Hide' : 'Reveal'}
        </Button>,
      );
    }

    actions.push(
      <Button
        key="edit"
        size="small"
        icon={<EditOutlined />}
        onClick={() => handleEdit(setting)}
      >
        {setting.is_set ? 'Edit' : 'Set Value'}
      </Button>,
    );

    if (TESTABLE_KEYS.includes(setting.key)) {
      actions.push(
        <Button
          key="test"
          size="small"
          icon={<ApiOutlined />}
          loading={testingKey === setting.key}
          onClick={() => handleTestConnection(setting.key)}
          disabled={!setting.is_set}
        >
          Test Connection
        </Button>,
      );
    }

    return <Space wrap>{actions}</Space>;
  };

  const collapseItems = CATEGORY_ORDER.filter((cat) => grouped[cat]).map((category) => ({
    key: category,
    label: (
      <Space>
        <Text strong>{CATEGORY_LABELS[category] || category}</Text>
        <Tag color="blue">{grouped[category].length} settings</Tag>
      </Space>
    ),
    children: (
      <Descriptions
        column={1}
        bordered
        size="small"
        labelStyle={{ width: 200, fontWeight: 500 }}
      >
        {grouped[category].map((setting) => (
          <Descriptions.Item
            key={setting.key}
            label={
              <div>
                <div>{setting.display_name}</div>
                {setting.description && (
                  <Text type="secondary" style={{ fontSize: 12, fontWeight: 400 }}>
                    {setting.description}
                  </Text>
                )}
              </div>
            }
          >
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'flex-start',
                gap: 12,
              }}
            >
              <div style={{ flex: 1 }}>
                {renderSettingValue(setting)}
                {setting.updated_by_name && (
                  <div style={{ marginTop: 4 }}>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      Last updated by {setting.updated_by_name} on{' '}
                      {new Date(setting.updated_at).toLocaleString()}
                    </Text>
                  </div>
                )}
              </div>
              <div>{renderSettingActions(setting)}</div>
            </div>
          </Descriptions.Item>
        ))}
      </Descriptions>
    ),
  }));

  return (
    <div>
      <Title level={5} style={{ marginBottom: 16 }}>
        System Configuration
      </Title>

      <Spin spinning={loading}>
        <Collapse
          defaultActiveKey={CATEGORY_ORDER}
          items={collapseItems}
          style={{ background: 'transparent' }}
        />
      </Spin>

      {/* Edit Setting Modal */}
      <Modal
        title={editingSetting ? `Edit: ${editingSetting.display_name}` : 'Edit Setting'}
        open={editModalOpen}
        onOk={handleEditSave}
        onCancel={() => {
          setEditModalOpen(false);
          setEditingSetting(null);
        }}
        confirmLoading={editSaving}
        okText="Save"
      >
        {editingSetting && (
          <div style={{ marginTop: 16 }}>
            <Text type="secondary" style={{ display: 'block', marginBottom: 12 }}>
              {editingSetting.description}
            </Text>
            {editingSetting.is_set && (
              <div style={{ marginBottom: 12 }}>
                <Text type="secondary">
                  Current value:{' '}
                  {editingSetting.is_encrypted ? (
                    <Text code>{editingSetting.value || 'Not set'}</Text>
                  ) : (
                    <Text code>{editingSetting.value || 'Empty'}</Text>
                  )}
                </Text>
              </div>
            )}
            <Input.TextArea
              rows={editingSetting.is_encrypted ? 2 : 1}
              placeholder={`Enter new value for ${editingSetting.display_name}`}
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
            />
            {editingSetting.validation_regex && (
              <Text type="secondary" style={{ fontSize: 12, marginTop: 4, display: 'block' }}>
                Expected format: <Text code>{editingSetting.validation_regex}</Text>
              </Text>
            )}
          </div>
        )}
      </Modal>
    </div>
  );
};

export default AdminSystemConfig;

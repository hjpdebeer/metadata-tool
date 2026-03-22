import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Card, Form, Input, Button, Typography, Alert, Space, Divider } from 'antd';
import { DatabaseOutlined, LockOutlined, MailOutlined, WindowsOutlined } from '@ant-design/icons';
import { useAuth } from '../hooks/useAuth';

const { Title, Text } = Typography;

const LoginPage: React.FC = () => {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { login } = useAuth();
  const navigate = useNavigate();

  const onFinish = async (values: { email: string; password: string }) => {
    setLoading(true);
    setError(null);
    try {
      await login(values.email, values.password);
      navigate('/dashboard', { replace: true });
    } catch (err: unknown) {
      if (err && typeof err === 'object' && 'response' in err) {
        const axiosErr = err as { response?: { data?: { message?: string } } };
        setError(axiosErr.response?.data?.message || 'Invalid credentials. Please try again.');
      } else {
        setError('Unable to connect to the server. Please try again later.');
      }
    } finally {
      setLoading(false);
    }
  };

  const handleSsoLogin = () => {
    // Full page redirect to backend SSO endpoint (not an AJAX call)
    window.location.href = '/api/v1/auth/login';
  };

  return (
    <div
      style={{
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'linear-gradient(135deg, #F5F7FA 0%, #E8EEF5 100%)',
      }}
    >
      <Card
        style={{
          width: 400,
          boxShadow: '0 4px 24px rgba(27, 58, 92, 0.10)',
          borderRadius: 8,
        }}
        styles={{ body: { padding: 32 } }}
      >
        <Space
          direction="vertical"
          size="small"
          style={{ width: '100%', textAlign: 'center', marginBottom: 24 }}
        >
          <DatabaseOutlined style={{ fontSize: 40, color: '#1B3A5C' }} />
          <Title level={3} style={{ margin: 0, color: '#1B3A5C' }}>
            Metadata Management Tool
          </Title>
          <Text type="secondary">Sign in to your account</Text>
        </Space>

        {error && (
          <Alert
            message={error}
            type="error"
            showIcon
            closable
            onClose={() => setError(null)}
            style={{ marginBottom: 20 }}
          />
        )}

        <Button
          block
          size="large"
          icon={<WindowsOutlined />}
          onClick={handleSsoLogin}
          style={{
            height: 44,
            background: '#2F2F2F',
            color: '#FFFFFF',
            border: 'none',
            fontWeight: 500,
          }}
        >
          Sign in with Microsoft
        </Button>

        <Divider plain style={{ margin: '20px 0' }}>
          <Text type="secondary" style={{ fontSize: 12 }}>or sign in with email</Text>
        </Divider>

        <Form
          name="login"
          onFinish={onFinish}
          layout="vertical"
          size="large"
          initialValues={undefined}
        >
          <Form.Item
            name="email"
            label="Email"
            rules={[
              { required: true, message: 'Please enter your email' },
              { type: 'email', message: 'Please enter a valid email' },
            ]}
          >
            <Input prefix={<MailOutlined />} placeholder="Email address" />
          </Form.Item>

          <Form.Item
            name="password"
            label="Password"
            rules={[{ required: true, message: 'Please enter your password' }]}
          >
            <Input.Password prefix={<LockOutlined />} placeholder="Password" />
          </Form.Item>

          <Form.Item style={{ marginBottom: 8 }}>
            <Button
              type="primary"
              htmlType="submit"
              loading={loading}
              block
              style={{ height: 44 }}
            >
              Sign In
            </Button>
          </Form.Item>
        </Form>

        {import.meta.env.DEV && (
          <Text
            type="secondary"
            style={{ display: 'block', textAlign: 'center', fontSize: 12, marginTop: 12 }}
          >
            Development mode — email login available for convenience
          </Text>
        )}
      </Card>
    </div>
  );
};

export default LoginPage;

import React, { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Spin, Alert, Typography } from 'antd';
import { useAuth } from '../hooks/useAuth';

const { Text } = Typography;

const SsoCallback: React.FC = () => {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { loginWithToken } = useAuth();
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const token = searchParams.get('token');
    if (!token) {
      setError('No authentication token received. Please try signing in again.');
      return;
    }

    loginWithToken(token)
      .then(() => {
        navigate('/dashboard', { replace: true });
      })
      .catch(() => {
        setError('Authentication failed. Please try signing in again.');
      });
  }, [searchParams, loginWithToken, navigate]);

  if (error) {
    return (
      <div
        style={{
          minHeight: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexDirection: 'column',
          gap: 16,
        }}
      >
        <Alert
          message="Authentication Error"
          description={error}
          type="error"
          showIcon
        />
        <a href="/login">Return to login</a>
      </div>
    );
  }

  return (
    <div
      style={{
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        flexDirection: 'column',
        gap: 16,
      }}
    >
      <Spin size="large" />
      <Text type="secondary">Completing sign-in...</Text>
    </div>
  );
};

export default SsoCallback;

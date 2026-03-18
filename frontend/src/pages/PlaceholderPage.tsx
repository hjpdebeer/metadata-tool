import React from 'react';
import { Card, Typography, Empty } from 'antd';

interface PlaceholderPageProps {
  title: string;
  description: string;
}

const PlaceholderPage: React.FC<PlaceholderPageProps> = ({ title, description }) => {
  return (
    <div>
      <Typography.Title level={3}>{title}</Typography.Title>
      <Card>
        <Empty description={description} />
      </Card>
    </div>
  );
};

export default PlaceholderPage;

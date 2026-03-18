import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Breadcrumb, Button, Card, Col, Row, Space, Statistic, Table, Tag, Typography, message } from 'antd';
import { ArrowLeftOutlined, WarningOutlined } from '@ant-design/icons';
import { dataDictionaryApi } from '../services/dataDictionaryApi';
import type { DataElementListItem } from '../services/dataDictionaryApi';

const { Title, Text } = Typography;

const statusColors: Record<string, string> = {
  DRAFT: 'default',
  PROPOSED: 'processing',
  UNDER_REVIEW: 'warning',
  REVISED: 'orange',
  ACCEPTED: 'success',
  REJECTED: 'error',
  DEPRECATED: 'default',
};

const statusLabels: Record<string, string> = {
  DRAFT: 'Draft',
  PROPOSED: 'Proposed',
  UNDER_REVIEW: 'Under Review',
  REVISED: 'Revised',
  ACCEPTED: 'Accepted',
  REJECTED: 'Rejected',
  DEPRECATED: 'Deprecated',
};

const CdePage: React.FC = () => {
  const navigate = useNavigate();

  const [cdeElements, setCdeElements] = useState<DataElementListItem[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchCdeElements = useCallback(async () => {
    setLoading(true);
    try {
      const response = await dataDictionaryApi.listCde();
      const data = response.data;
      if (Array.isArray(data)) {
        setCdeElements(data);
      } else {
        const paginated = data as unknown as { data: DataElementListItem[] };
        setCdeElements(paginated.data);
      }
    } catch {
      message.error('Failed to load critical data elements.');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchCdeElements();
  }, [fetchCdeElements]);

  // Compute summary stats
  const totalCdes = cdeElements.length;
  const cdeByDomain: Record<string, number> = {};
  const cdeByClassification: Record<string, number> = {};

  cdeElements.forEach((el) => {
    const domain = el.domain_name || 'Unassigned';
    cdeByDomain[domain] = (cdeByDomain[domain] || 0) + 1;

    const classification = el.classification_name || 'Unclassified';
    cdeByClassification[classification] = (cdeByClassification[classification] || 0) + 1;
  });

  const columns = [
    {
      title: 'Element Name',
      dataIndex: 'element_name',
      key: 'element_name',
      sorter: (a: DataElementListItem, b: DataElementListItem) =>
        a.element_name.localeCompare(b.element_name),
      render: (name: string, record: DataElementListItem) => (
        <a onClick={() => navigate(`/data-dictionary/${record.element_id}`)}>{name}</a>
      ),
    },
    {
      title: 'Element Code',
      dataIndex: 'element_code',
      key: 'element_code',
      width: 200,
      render: (code: string) => (
        <Text code style={{ fontSize: 12 }}>
          {code}
        </Text>
      ),
    },
    {
      title: 'Domain',
      dataIndex: 'domain_name',
      key: 'domain_name',
      render: (domain: string | null) => domain || '-',
      filters: Object.keys(cdeByDomain).map((d) => ({ text: d, value: d })),
      onFilter: (value: React.Key | boolean, record: DataElementListItem) => {
        const domain = record.domain_name || 'Unassigned';
        return domain === value;
      },
    },
    {
      title: 'Classification',
      dataIndex: 'classification_name',
      key: 'classification_name',
      render: (name: string | null) => name || '-',
      filters: Object.keys(cdeByClassification).map((c) => ({ text: c, value: c })),
      onFilter: (value: React.Key | boolean, record: DataElementListItem) => {
        const classification = record.classification_name || 'Unclassified';
        return classification === value;
      },
    },
    {
      title: 'Data Type',
      dataIndex: 'data_type',
      key: 'data_type',
      width: 120,
    },
    {
      title: 'Status',
      dataIndex: 'status_code',
      key: 'status_code',
      width: 140,
      render: (status: string) => (
        <Tag color={statusColors[status] || 'default'}>
          {statusLabels[status] || status}
        </Tag>
      ),
    },
    {
      title: 'Owner',
      dataIndex: 'owner_name',
      key: 'owner_name',
      render: (name: string | null) => name || '-',
    },
    {
      title: 'Updated',
      dataIndex: 'updated_at',
      key: 'updated_at',
      width: 140,
      sorter: (a: DataElementListItem, b: DataElementListItem) =>
        new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime(),
      render: (date: string) => {
        if (!date) return '-';
        return new Date(date).toLocaleDateString('en-ZA', {
          year: 'numeric',
          month: 'short',
          day: 'numeric',
        });
      },
    },
  ];

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/data-dictionary')}>Data Dictionary</a> },
          { title: 'Critical Data Elements' },
        ]}
      />

      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 16,
        }}
      >
        <Space align="center">
          <Button
            type="text"
            icon={<ArrowLeftOutlined />}
            onClick={() => navigate('/data-dictionary')}
          />
          <Title level={3} style={{ margin: 0 }}>
            Critical Data Elements
          </Title>
          <Tag color="red" style={{ fontSize: 14, padding: '2px 12px', fontWeight: 600 }}>
            CDE
          </Tag>
        </Space>
      </div>

      <Row gutter={16} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Total CDEs"
              value={totalCdes}
              prefix={<WarningOutlined />}
              valueStyle={{ color: '#FF4D4F' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <div style={{ marginBottom: 8 }}>
              <Text type="secondary" style={{ fontSize: 14 }}>CDEs by Domain</Text>
            </div>
            {Object.keys(cdeByDomain).length === 0 ? (
              <Text type="secondary">-</Text>
            ) : (
              <Space wrap>
                {Object.entries(cdeByDomain).map(([domain, count]) => (
                  <Tag key={domain} color="red">
                    {domain}: {count}
                  </Tag>
                ))}
              </Space>
            )}
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <div style={{ marginBottom: 8 }}>
              <Text type="secondary" style={{ fontSize: 14 }}>CDEs by Classification</Text>
            </div>
            {Object.keys(cdeByClassification).length === 0 ? (
              <Text type="secondary">-</Text>
            ) : (
              <Space wrap>
                {Object.entries(cdeByClassification).map(([classification, count]) => (
                  <Tag key={classification} color="red">
                    {classification}: {count}
                  </Tag>
                ))}
              </Space>
            )}
          </Card>
        </Col>
      </Row>

      <Card>
        <Table
          columns={columns}
          dataSource={cdeElements}
          rowKey="element_id"
          loading={loading}
          pagination={{
            showSizeChanger: true,
            pageSizeOptions: ['10', '20', '50', '100'],
            showTotal: (total, range) => `${range[0]}-${range[1]} of ${total} CDEs`,
          }}
        />
      </Card>
    </div>
  );
};

export default CdePage;

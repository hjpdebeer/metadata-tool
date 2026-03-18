import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Card, Col, Progress, Row, Space, Table, Tag, Typography, message } from 'antd';
import { ArrowRightOutlined, SafetyCertificateOutlined } from '@ant-design/icons';
import { dataQualityApi } from '../services/dataQualityApi';
import type { QualityAssessment, QualityDimensionSummary } from '../services/dataQualityApi';

const { Title, Text } = Typography;

const getScoreColor = (score: number | null): string => {
  if (score === null) return '#D9D9D9';
  if (score >= 90) return '#52C41A';
  if (score >= 70) return '#FAAD14';
  return '#FF4D4F';
};

const getScoreStatus = (score: number | null): 'success' | 'normal' | 'exception' => {
  if (score === null) return 'normal';
  if (score >= 90) return 'success';
  if (score >= 70) return 'normal';
  return 'exception';
};

const assessmentStatusColors: Record<string, string> = {
  PASSED: 'success',
  FAILED: 'error',
  WARNING: 'warning',
  COMPLETED: 'processing',
};

const DataQualityDashboard: React.FC = () => {
  const navigate = useNavigate();

  const [dimensions, setDimensions] = useState<QualityDimensionSummary[]>([]);
  const [recentAssessments, setRecentAssessments] = useState<QualityAssessment[]>([]);
  const [loadingDimensions, setLoadingDimensions] = useState(false);
  const [loadingAssessments, setLoadingAssessments] = useState(false);

  const fetchDimensions = useCallback(async () => {
    setLoadingDimensions(true);
    try {
      const response = await dataQualityApi.listDimensions();
      setDimensions(response.data);
    } catch {
      message.error('Failed to load quality dimensions.');
    } finally {
      setLoadingDimensions(false);
    }
  }, []);

  const fetchRecentAssessments = useCallback(async () => {
    setLoadingAssessments(true);
    try {
      const response = await dataQualityApi.getRecentAssessments(10);
      setRecentAssessments(response.data);
    } catch {
      // Recent assessments are non-critical; endpoint may not exist yet
    } finally {
      setLoadingAssessments(false);
    }
  }, []);

  useEffect(() => {
    fetchDimensions();
    fetchRecentAssessments();
  }, [fetchDimensions, fetchRecentAssessments]);

  const assessmentColumns = [
    {
      title: 'Rule',
      dataIndex: 'rule_name',
      key: 'rule_name',
      render: (name: string | undefined, record: QualityAssessment) => (
        <a onClick={() => navigate(`/data-quality/rules/${record.rule_id}`)}>
          {name || record.rule_id.slice(0, 8)}
        </a>
      ),
    },
    {
      title: 'Score',
      dataIndex: 'score_percentage',
      key: 'score_percentage',
      width: 120,
      render: (score: number) => (
        <Tag
          color={getScoreColor(score)}
          style={{ fontWeight: 600, minWidth: 52, textAlign: 'center' }}
        >
          {score.toFixed(1)}%
        </Tag>
      ),
    },
    {
      title: 'Records',
      key: 'records',
      width: 180,
      render: (_: unknown, record: QualityAssessment) => (
        <Text type="secondary" style={{ fontSize: 12 }}>
          {record.records_passed.toLocaleString()} / {record.records_assessed.toLocaleString()} passed
        </Text>
      ),
    },
    {
      title: 'Status',
      dataIndex: 'status',
      key: 'status',
      width: 120,
      render: (status: string) => (
        <Tag color={assessmentStatusColors[status] || 'default'}>
          {status}
        </Tag>
      ),
    },
    {
      title: 'Assessed',
      dataIndex: 'assessed_at',
      key: 'assessed_at',
      width: 140,
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
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 16,
        }}
      >
        <Title level={3} style={{ margin: 0 }}>
          Data Quality
        </Title>
        <Button
          type="primary"
          icon={<SafetyCertificateOutlined />}
          onClick={() => navigate('/data-quality/rules')}
        >
          Manage Rules
        </Button>
      </div>

      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        {loadingDimensions ? (
          <Col span={24}>
            <Card loading />
          </Col>
        ) : dimensions.length > 0 ? (
          dimensions.map((dim) => (
            <Col xs={24} sm={12} md={8} key={dim.dimension_id}>
              <Card
                hoverable
                style={{ height: '100%' }}
                onClick={() => navigate(`/data-quality/rules?dimension_id=${dim.dimension_id}`)}
              >
                <div style={{ marginBottom: 12 }}>
                  <Text strong style={{ fontSize: 16 }}>
                    {dim.dimension_name}
                  </Text>
                </div>
                <Progress
                  type="dashboard"
                  percent={dim.avg_score !== null ? Math.round(dim.avg_score) : 0}
                  size={100}
                  status={getScoreStatus(dim.avg_score)}
                  format={(percent) =>
                    dim.avg_score !== null ? (
                      <span style={{ color: getScoreColor(dim.avg_score), fontWeight: 600 }}>
                        {percent}%
                      </span>
                    ) : (
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        N/A
                      </Text>
                    )
                  }
                />
                <div style={{ marginTop: 12 }}>
                  <Space direction="vertical" size={2}>
                    <Text type="secondary" style={{ fontSize: 13 }}>
                      {dim.rules_count} {dim.rules_count === 1 ? 'rule' : 'rules'}
                    </Text>
                    {dim.last_assessed_at && (
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        Last assessed:{' '}
                        {new Date(dim.last_assessed_at).toLocaleDateString('en-ZA', {
                          month: 'short',
                          day: 'numeric',
                        })}
                      </Text>
                    )}
                  </Space>
                </div>
              </Card>
            </Col>
          ))
        ) : (
          <Col span={24}>
            <Card>
              <div style={{ textAlign: 'center', padding: '24px 0' }}>
                <SafetyCertificateOutlined style={{ fontSize: 48, color: '#D9D9D9', marginBottom: 16 }} />
                <div>
                  <Text type="secondary" style={{ fontSize: 16 }}>
                    No quality dimensions configured yet.
                  </Text>
                </div>
                <Button
                  type="link"
                  icon={<ArrowRightOutlined />}
                  onClick={() => navigate('/data-quality/rules')}
                  style={{ marginTop: 8 }}
                >
                  Set up quality rules
                </Button>
              </div>
            </Card>
          </Col>
        )}
      </Row>

      <Card
        title="Recent Assessments"
        extra={
          <Button type="link" onClick={() => navigate('/data-quality/rules')}>
            View All Rules <ArrowRightOutlined />
          </Button>
        }
      >
        <Table
          columns={assessmentColumns}
          dataSource={recentAssessments}
          rowKey="assessment_id"
          loading={loadingAssessments}
          pagination={false}
          locale={{ emptyText: 'No assessments recorded yet.' }}
          size="small"
        />
      </Card>
    </div>
  );
};

export default DataQualityDashboard;

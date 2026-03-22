import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams, Link } from 'react-router-dom';
import {
  Breadcrumb,
  Button,
  Card,
  Col,
  Descriptions,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Space,
  Spin,
  Statistic,
  Table,
  Tag,
  Typography,
  message,
} from 'antd';
import {
  ArrowLeftOutlined,
  EditOutlined,
  LinkOutlined,
} from '@ant-design/icons';
import { dataQualityApi } from '../services/dataQualityApi';
import type { QualityAssessment, QualityRule } from '../services/dataQualityApi';

const { Title, Text } = Typography;

const severityColors: Record<string, string> = {
  LOW: '#52C41A',
  MEDIUM: '#1890FF',
  HIGH: '#FA8C16',
  CRITICAL: '#FF4D4F',
};

const getScoreColor = (score: number): string => {
  if (score >= 90) return '#52C41A';
  if (score >= 70) return '#FAAD14';
  return '#FF4D4F';
};

const QualityRuleDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const [rule, setRule] = useState<QualityRule | null>(null);
  const [assessments, setAssessments] = useState<QualityAssessment[]>([]);
  const [loading, setLoading] = useState(true);
  const [assessmentModalOpen, setAssessmentModalOpen] = useState(false);
  const [assessmentLoading, setAssessmentLoading] = useState(false);
  const [assessmentForm] = Form.useForm();
  const [elementStatus, setElementStatus] = useState<string | null>(null);

  const fetchRule = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await dataQualityApi.getRule(id);
      setRule(response.data);
    } catch {
      message.error('Failed to load rule details.');
      navigate('/data-quality/rules');
    } finally {
      setLoading(false);
    }
  }, [id, navigate]);

  const fetchAssessments = useCallback(async () => {
    if (!id) return;
    try {
      const response = await dataQualityApi.getAssessments(id);
      setAssessments(response.data);
    } catch {
      // Assessments fetch is non-critical
    }
  }, [id]);

  useEffect(() => {
    fetchRule();
    fetchAssessments();
  }, [fetchRule, fetchAssessments]);

  // Fetch parent element status to determine if edit is allowed
  useEffect(() => {
    if (rule?.element_id) {
      import('../services/dataDictionaryApi').then(({ dataDictionaryApi }) => {
        dataDictionaryApi.getElement(rule.element_id!).then((res) => {
          const sc = (res.data as unknown as Record<string, unknown>).status_code as string || null;
          setElementStatus(sc);
        }).catch(() => {});
      });
    }
  }, [rule?.element_id]);

  const handleRecordAssessment = async () => {
    if (!id) return;

    try {
      const values = await assessmentForm.validateFields();
      setAssessmentLoading(true);

      await dataQualityApi.createAssessment({
        rule_id: id,
        records_assessed: values.records_assessed,
        records_passed: values.records_passed,
        records_failed: values.records_assessed - values.records_passed,
        details: values.details || undefined,
      });

      message.success('Assessment recorded successfully.');
      setAssessmentModalOpen(false);
      assessmentForm.resetFields();
      fetchAssessments();
    } catch {
      message.error('Failed to record assessment.');
    } finally {
      setAssessmentLoading(false);
    }
  };

  const formatDate = (dateStr: string | null | undefined) => {
    if (!dateStr) return '-';
    return new Date(dateStr).toLocaleString('en-ZA', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  if (loading) {
    return (
      <div style={{ textAlign: 'center', padding: 80 }}>
        <Spin size="large" />
      </div>
    );
  }

  if (!rule) {
    return null;
  }

  const isMutableElement = elementStatus && !['ACCEPTED', 'DEPRECATED', 'SUPERSEDED', 'REJECTED'].includes(elementStatus);

  const renderActionButtons = () => {
    const buttons: React.ReactNode[] = [];

    if (isMutableElement) {
      buttons.push(
        <Button
          key="edit"
          icon={<EditOutlined />}
          onClick={() => navigate(`/data-quality/rules/${id}/edit`)}
        >
          Edit
        </Button>,
      );
    }

    return buttons;
  };

  const latestScore = assessments.length > 0 ? assessments[0].score_percentage : null;

  const assessmentColumns = [
    {
      title: 'Date',
      dataIndex: 'assessed_at',
      key: 'assessed_at',
      width: 180,
      render: (date: string) => formatDate(date),
    },
    {
      title: 'Score',
      dataIndex: 'score_percentage',
      key: 'score_percentage',
      width: 100,
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
      title: 'Records Assessed',
      dataIndex: 'records_assessed',
      key: 'records_assessed',
      width: 150,
      align: 'right' as const,
      render: (val: number) => val.toLocaleString(),
    },
    {
      title: 'Passed',
      dataIndex: 'records_passed',
      key: 'records_passed',
      width: 120,
      align: 'right' as const,
      render: (val: number) => (
        <Text style={{ color: '#52C41A' }}>{val.toLocaleString()}</Text>
      ),
    },
    {
      title: 'Failed',
      dataIndex: 'records_failed',
      key: 'records_failed',
      width: 120,
      align: 'right' as const,
      render: (val: number) => (
        <Text style={{ color: val > 0 ? '#FF4D4F' : undefined }}>
          {val.toLocaleString()}
        </Text>
      ),
    },
    {
      title: 'Status',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (assessmentStatus: string) => {
        const colors: Record<string, string> = {
          PASSED: 'success',
          FAILED: 'error',
          WARNING: 'warning',
          COMPLETED: 'processing',
        };
        return (
          <Tag color={colors[assessmentStatus] || 'default'}>
            {assessmentStatus}
          </Tag>
        );
      },
    },
    {
      title: 'Details',
      dataIndex: 'details',
      key: 'details',
      ellipsis: true,
      render: (details: string | null) => details || '-',
    },
  ];

  let ruleDefinitionDisplay: string;
  try {
    ruleDefinitionDisplay =
      typeof rule.rule_definition === 'string'
        ? JSON.stringify(JSON.parse(rule.rule_definition as unknown as string), null, 2)
        : JSON.stringify(rule.rule_definition, null, 2);
  } catch {
    ruleDefinitionDisplay = String(rule.rule_definition);
  }

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/data-quality')}>Data Quality</a> },
          { title: <a onClick={() => navigate('/data-quality/rules')}>Quality Rules</a> },
          { title: rule.rule_name },
        ]}
      />

      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'flex-start',
          marginBottom: 16,
        }}
      >
        <Space align="center">
          <Button
            type="text"
            icon={<ArrowLeftOutlined />}
            onClick={() => navigate('/data-quality/rules')}
          />
          <Title level={3} style={{ margin: 0 }}>
            {rule.rule_name}
          </Title>
          <Tag
            color={severityColors[rule.severity] || 'default'}
            style={{ fontSize: 14, padding: '2px 12px', fontWeight: 600 }}
          >
            {rule.severity}
          </Tag>
        </Space>
        <Space>{renderActionButtons()}</Space>
      </div>

      <Row gutter={16} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Threshold"
              value={rule.threshold_percentage}
              suffix="%"
              valueStyle={{ color: '#1B3A5C' }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Latest Score"
              value={latestScore !== null ? latestScore : undefined}
              suffix={latestScore !== null ? '%' : undefined}
              formatter={(value) =>
                latestScore !== null ? (
                  <span style={{ color: getScoreColor(latestScore) }}>
                    {Number(value).toFixed(1)}%
                  </span>
                ) : (
                  <Text type="secondary">No assessments</Text>
                )
              }
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Total Assessments"
              value={assessments.length}
            />
          </Card>
        </Col>
      </Row>

      <Card title="Rule Details" style={{ marginBottom: 24 }}>
        <Descriptions column={1} bordered size="small" labelStyle={{ width: 180 }}>
          <Descriptions.Item label="Rule Name">{rule.rule_name}</Descriptions.Item>
          <Descriptions.Item label="Rule Code">
            <Text code>{rule.rule_code}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Description" span={2}>
            {rule.description}
          </Descriptions.Item>
          <Descriptions.Item label="Dimension">{rule.dimension_name || '-'}</Descriptions.Item>
          <Descriptions.Item label="Rule Type">{rule.rule_type_name || '-'}</Descriptions.Item>
          <Descriptions.Item label="Data Element">
            {rule.element_name && rule.element_id ? (
              <Link to={`/data-dictionary/${rule.element_id}`}>
                <Tag color="purple" style={{ cursor: 'pointer' }}>
                  <LinkOutlined /> {rule.element_name}
                </Tag>
              </Link>
            ) : (
              '-'
            )}
          </Descriptions.Item>
          <Descriptions.Item label="Severity">
            <Tag color={severityColors[rule.severity] || 'default'} style={{ fontWeight: 600 }}>
              {rule.severity}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Threshold">
            {rule.threshold_percentage}%
          </Descriptions.Item>
          <Descriptions.Item label="Active">
            {rule.is_active ? (
              <Tag color="green">Active</Tag>
            ) : (
              <Tag color="default">Inactive</Tag>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="Created">
            {formatDate(rule.created_at)}
          </Descriptions.Item>
          <Descriptions.Item label="Last Updated">
            {formatDate(rule.updated_at)}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Card title="Rule Definition" style={{ marginBottom: 24 }}>
        <pre
          style={{
            backgroundColor: '#F5F5F5',
            border: '1px solid #E8E8E8',
            borderRadius: 6,
            padding: 16,
            fontSize: 13,
            fontFamily: "'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace",
            overflow: 'auto',
            maxHeight: 400,
            margin: 0,
          }}
        >
          {ruleDefinitionDisplay}
        </pre>
      </Card>

      <Card title="Assessment History" style={{ marginBottom: 24 }}>
        <Table
          columns={assessmentColumns}
          dataSource={assessments}
          rowKey="assessment_id"
          pagination={{
            pageSize: 10,
            showSizeChanger: false,
            showTotal: (total) => `${total} assessments`,
          }}
          size="small"
          locale={{ emptyText: 'No assessments recorded yet.' }}
        />
      </Card>

      <Modal
        title="Record Assessment"
        open={assessmentModalOpen}
        onOk={handleRecordAssessment}
        onCancel={() => setAssessmentModalOpen(false)}
        confirmLoading={assessmentLoading}
        okText="Record"
      >
        <Form
          form={assessmentForm}
          layout="vertical"
          style={{ marginTop: 16 }}
        >
          <Form.Item
            name="records_assessed"
            label="Records Assessed"
            rules={[
              { required: true, message: 'Number of records assessed is required' },
              { type: 'number', min: 1, message: 'Must be at least 1' },
            ]}
          >
            <InputNumber
              style={{ width: '100%' }}
              min={1}
              placeholder="Total number of records assessed"
              formatter={(value) => `${value}`.replace(/\B(?=(\d{3})+(?!\d))/g, ',')}
            />
          </Form.Item>
          <Form.Item
            name="records_passed"
            label="Records Passed"
            rules={[
              { required: true, message: 'Number of records passed is required' },
              { type: 'number', min: 0, message: 'Cannot be negative' },
            ]}
          >
            <InputNumber
              style={{ width: '100%' }}
              min={0}
              placeholder="Number of records that passed the rule"
              formatter={(value) => `${value}`.replace(/\B(?=(\d{3})+(?!\d))/g, ',')}
            />
          </Form.Item>
          <Form.Item
            name="details"
            label="Details"
          >
            <Input.TextArea
              rows={3}
              placeholder="Additional details about the assessment (optional)"
            />
          </Form.Item>
          <div
            style={{
              backgroundColor: '#F5F5F5',
              padding: '8px 12px',
              borderRadius: 6,
              fontSize: 13,
            }}
          >
            <Text type="secondary">
              Score will be calculated automatically as: (records_passed / records_assessed) x 100
            </Text>
          </div>
        </Form>
      </Modal>
    </div>
  );
};

export default QualityRuleDetail;

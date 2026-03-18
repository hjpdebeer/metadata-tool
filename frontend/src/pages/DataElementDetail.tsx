import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  Alert,
  Breadcrumb,
  Button,
  Card,
  Col,
  Descriptions,
  Divider,
  Input,
  Modal,
  Row,
  Space,
  Spin,
  Statistic,
  Table,
  Tag,
  Timeline,
  Tooltip,
  Typography,
  message,
} from 'antd';
import {
  ArrowLeftOutlined,
  CheckCircleOutlined,
  CheckOutlined,
  CloseCircleOutlined,
  CloseOutlined,
  EditOutlined,
  KeyOutlined,
  LinkOutlined,
  SafetyCertificateOutlined,
  SendOutlined,
  UndoOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import { dataDictionaryApi } from '../services/dataDictionaryApi';
import { workflowApi } from '../services/glossaryApi';
import type { DataElementFullView, TechnicalColumn } from '../services/dataDictionaryApi';
import type { WorkflowInstanceView } from '../services/glossaryApi';
import { useAuth } from '../hooks/useAuth';

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

const sensitivityColors: Record<string, string> = {
  PUBLIC: 'green',
  INTERNAL: 'blue',
  CONFIDENTIAL: 'orange',
  RESTRICTED: 'red',
};

const DataElementDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();

  const [element, setElement] = useState<DataElementFullView | null>(null);
  const [workflowInstance, setWorkflowInstance] = useState<WorkflowInstanceView | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [transitionModalOpen, setTransitionModalOpen] = useState(false);
  const [transitionAction, setTransitionAction] = useState('');
  const [transitionComments, setTransitionComments] = useState('');
  const [cdeModalOpen, setCdeModalOpen] = useState(false);
  const [cdeRationale, setCdeRationale] = useState('');
  const [cdeLoading, setCdeLoading] = useState(false);

  const isSteward = user?.roles?.includes('data_steward') || user?.roles?.includes('admin');

  const fetchElement = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await dataDictionaryApi.getElement(id);
      setElement(response.data);

      // Fetch workflow instance if one exists
      if (response.data.workflow_instance_id) {
        try {
          const wfResponse = await workflowApi.getInstance(response.data.workflow_instance_id);
          setWorkflowInstance(wfResponse.data);
        } catch {
          // Workflow instance may not exist yet or endpoint may not be implemented
        }
      }
    } catch {
      message.error('Failed to load element details.');
      navigate('/data-dictionary');
    } finally {
      setLoading(false);
    }
  }, [id, navigate]);

  useEffect(() => {
    fetchElement();
  }, [fetchElement]);

  const handleWorkflowAction = (action: string) => {
    if (!element?.workflow_instance_id) {
      message.error('No active workflow for this element.');
      return;
    }
    setTransitionAction(action);
    setTransitionComments('');
    setTransitionModalOpen(true);
  };

  const submitTransition = async () => {
    if (!element?.workflow_instance_id) return;

    setActionLoading(true);
    try {
      await workflowApi.transitionWorkflow(
        element.workflow_instance_id,
        transitionAction,
        transitionComments || undefined,
      );
      message.success(`Workflow action "${transitionAction}" completed successfully.`);
      setTransitionModalOpen(false);
      fetchElement();
    } catch {
      message.error(`Failed to perform action "${transitionAction}".`);
    } finally {
      setActionLoading(false);
    }
  };

  const handleDesignateCde = async () => {
    if (!id) return;
    setCdeLoading(true);
    try {
      await dataDictionaryApi.designateCde(id, {
        is_cde: true,
        cde_rationale: cdeRationale || undefined,
      });
      message.success('Element designated as Critical Data Element.');
      setCdeModalOpen(false);
      setCdeRationale('');
      fetchElement();
    } catch {
      message.error('Failed to designate element as CDE.');
    } finally {
      setCdeLoading(false);
    }
  };

  const handleRemoveCde = async () => {
    if (!id) return;
    setCdeLoading(true);
    try {
      await dataDictionaryApi.designateCde(id, { is_cde: false });
      message.success('CDE designation removed.');
      fetchElement();
    } catch {
      message.error('Failed to remove CDE designation.');
    } finally {
      setCdeLoading(false);
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

  if (!element) {
    return null;
  }

  const status = element.status_code || 'DRAFT';

  const renderActionButtons = () => {
    const buttons: React.ReactNode[] = [];

    if (status === 'DRAFT') {
      buttons.push(
        <Button
          key="submit"
          type="primary"
          icon={<SendOutlined />}
          onClick={() => handleWorkflowAction('SUBMIT')}
        >
          Submit for Review
        </Button>,
      );
    }

    if (status === 'UNDER_REVIEW' && isSteward) {
      buttons.push(
        <Button
          key="approve"
          type="primary"
          icon={<CheckOutlined />}
          style={{ backgroundColor: '#52C41A', borderColor: '#52C41A' }}
          onClick={() => handleWorkflowAction('APPROVE')}
        >
          Approve
        </Button>,
        <Button
          key="reject"
          danger
          icon={<CloseOutlined />}
          onClick={() => handleWorkflowAction('REJECT')}
        >
          Reject
        </Button>,
        <Button
          key="revise"
          icon={<UndoOutlined />}
          onClick={() => handleWorkflowAction('REVISE')}
        >
          Request Revision
        </Button>,
      );
    }

    if (status === 'REVISED') {
      buttons.push(
        <Button
          key="resubmit"
          type="primary"
          icon={<SendOutlined />}
          onClick={() => handleWorkflowAction('SUBMIT')}
        >
          Resubmit
        </Button>,
      );
    }

    buttons.push(
      <Button
        key="edit"
        icon={<EditOutlined />}
        onClick={() => navigate(`/data-dictionary/${id}/edit`)}
        disabled={status === 'ACCEPTED' || status === 'DEPRECATED'}
      >
        Edit
      </Button>,
    );

    return buttons;
  };

  const technicalColumns = [
    {
      title: 'Column Name',
      dataIndex: 'column_name',
      key: 'column_name',
      render: (name: string) => (
        <Text code style={{ fontSize: 12 }}>
          {name}
        </Text>
      ),
    },
    {
      title: 'Data Type',
      dataIndex: 'data_type',
      key: 'data_type',
      width: 120,
    },
    {
      title: 'Position',
      dataIndex: 'ordinal_position',
      key: 'ordinal_position',
      width: 80,
      align: 'center' as const,
    },
    {
      title: 'Nullable',
      dataIndex: 'is_nullable',
      key: 'is_nullable',
      width: 80,
      align: 'center' as const,
      render: (val: boolean) => (val ? 'Yes' : 'No'),
    },
    {
      title: 'Keys',
      key: 'keys',
      width: 100,
      render: (_: unknown, record: TechnicalColumn) => (
        <Space size={4}>
          {record.is_primary_key && (
            <Tooltip title="Primary Key">
              <Tag color="gold" icon={<KeyOutlined />}>
                PK
              </Tag>
            </Tooltip>
          )}
          {record.is_foreign_key && (
            <Tooltip title="Foreign Key">
              <Tag color="blue" icon={<LinkOutlined />}>
                FK
              </Tag>
            </Tooltip>
          )}
        </Space>
      ),
    },
    {
      title: 'Naming Compliance',
      key: 'naming_compliance',
      width: 160,
      render: (_: unknown, record: TechnicalColumn) => {
        if (record.naming_standard_compliant === null) {
          return <Text type="secondary">-</Text>;
        }
        return record.naming_standard_compliant ? (
          <Tag icon={<CheckCircleOutlined />} color="success">
            Compliant
          </Tag>
        ) : (
          <Tooltip title={record.naming_standard_violation || 'Naming violation'}>
            <Tag icon={<CloseCircleOutlined />} color="error">
              Violation
            </Tag>
          </Tooltip>
        );
      },
    },
  ];

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/data-dictionary')}>Data Dictionary</a> },
          { title: element.element_name },
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
            onClick={() => navigate('/data-dictionary')}
          />
          <Title level={3} style={{ margin: 0 }}>
            {element.element_name}
          </Title>
          <Tag
            color={statusColors[status] || 'default'}
            style={{ fontSize: 14, padding: '2px 12px' }}
          >
            {statusLabels[status] || status}
          </Tag>
          {element.is_cde && (
            <Tag color="red" style={{ fontSize: 14, padding: '2px 12px', fontWeight: 600 }}>
              CDE
            </Tag>
          )}
        </Space>
        <Space>{renderActionButtons()}</Space>
      </div>

      {element.is_cde && (
        <Alert
          type="error"
          showIcon
          icon={<WarningOutlined />}
          message="Critical Data Element"
          description={
            <div>
              <Text strong>Rationale: </Text>
              <Text>{element.cde_rationale || 'No rationale provided.'}</Text>
              {element.cde_designated_at && (
                <>
                  <br />
                  <Text type="secondary">
                    Designated on {formatDate(element.cde_designated_at)}
                  </Text>
                </>
              )}
            </div>
          }
          style={{ marginBottom: 24 }}
          action={
            isSteward ? (
              <Button size="small" danger onClick={handleRemoveCde} loading={cdeLoading}>
                Remove CDE
              </Button>
            ) : undefined
          }
        />
      )}

      <Row gutter={16} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Quality Rules"
              value={element.quality_rules_count}
              prefix={<SafetyCertificateOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Linked Processes"
              value={element.linked_processes_count}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Linked Applications"
              value={element.linked_applications_count}
            />
          </Card>
        </Col>
      </Row>

      <Card title="Element Details" style={{ marginBottom: 24 }}>
        <Descriptions column={{ xs: 1, sm: 1, md: 2 }} bordered size="small">
          <Descriptions.Item label="Element Name">{element.element_name}</Descriptions.Item>
          <Descriptions.Item label="Element Code">
            <Text code>{element.element_code}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Description" span={2}>
            {element.description}
          </Descriptions.Item>
          <Descriptions.Item label="Business Definition" span={2}>
            {element.business_definition || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Business Rules" span={2}>
            {element.business_rules || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Data Type">{element.data_type}</Descriptions.Item>
          <Descriptions.Item label="Format Pattern">
            {element.format_pattern || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Allowed Values" span={2}>
            {element.allowed_values ? (
              <Text code style={{ fontSize: 12, whiteSpace: 'pre-wrap' }}>
                {typeof element.allowed_values === 'string'
                  ? element.allowed_values
                  : JSON.stringify(element.allowed_values, null, 2)}
              </Text>
            ) : (
              '-'
            )}
          </Descriptions.Item>
          <Descriptions.Item label="Default Value">
            {element.default_value || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Nullable">
            {element.is_nullable ? 'Yes' : 'No'}
          </Descriptions.Item>
          <Descriptions.Item label="Glossary Term">
            {element.glossary_term_name ? (
              <a onClick={() => navigate(`/glossary/${element.glossary_term_id}`)}>
                {element.glossary_term_name}
              </a>
            ) : (
              '-'
            )}
          </Descriptions.Item>
          <Descriptions.Item label="Domain">
            {element.domain_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Classification">
            {element.classification_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Sensitivity Level">
            {element.sensitivity_level ? (
              <Tag color={sensitivityColors[element.sensitivity_level] || 'default'}>
                {element.sensitivity_level}
              </Tag>
            ) : (
              '-'
            )}
          </Descriptions.Item>
          <Descriptions.Item label="Owner">
            {element.owner_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Steward">
            {element.steward_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Status">
            <Tag color={statusColors[status] || 'default'}>
              {statusLabels[status] || status}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="CDE">
            {element.is_cde ? (
              <Tag color="red" style={{ fontWeight: 600 }}>
                CDE
              </Tag>
            ) : (
              <Space>
                <Text type="secondary">No</Text>
                {isSteward && (
                  <Button
                    size="small"
                    type="link"
                    onClick={() => {
                      setCdeRationale('');
                      setCdeModalOpen(true);
                    }}
                  >
                    Designate as CDE
                  </Button>
                )}
              </Space>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="Created">
            {formatDate(element.created_at)}
            {element.created_by_name ? ` by ${element.created_by_name}` : ''}
          </Descriptions.Item>
          <Descriptions.Item label="Last Updated">
            {formatDate(element.updated_at)}
            {element.updated_by_name ? ` by ${element.updated_by_name}` : ''}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      {element.technical_columns && element.technical_columns.length > 0 && (
        <Card title="Technical Metadata" style={{ marginBottom: 24 }}>
          <Table
            columns={technicalColumns}
            dataSource={element.technical_columns}
            rowKey="column_id"
            pagination={false}
            size="small"
          />
        </Card>
      )}

      {workflowInstance && (
        <Card title="Workflow" style={{ marginBottom: 24 }}>
          <Descriptions column={{ xs: 1, sm: 2 }} size="small" style={{ marginBottom: 16 }}>
            <Descriptions.Item label="Current State">
              <Tag color={statusColors[workflowInstance.current_state_name?.toUpperCase()] || 'processing'}>
                {workflowInstance.current_state_name}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="Initiated By">
              {workflowInstance.initiated_by_name}
            </Descriptions.Item>
            <Descriptions.Item label="Initiated At">
              {formatDate(workflowInstance.initiated_at)}
            </Descriptions.Item>
            {workflowInstance.completed_at && (
              <Descriptions.Item label="Completed At">
                {formatDate(workflowInstance.completed_at)}
              </Descriptions.Item>
            )}
          </Descriptions>

          {workflowInstance.history && workflowInstance.history.length > 0 && (
            <>
              <Divider orientation="left" plain>
                <Text strong>History</Text>
              </Divider>
              <Timeline
                items={workflowInstance.history.map((entry) => ({
                  color:
                    entry.action === 'APPROVE'
                      ? 'green'
                      : entry.action === 'REJECT'
                        ? 'red'
                        : 'blue',
                  children: (
                    <div>
                      <Text strong>{entry.action}</Text>
                      {entry.from_state_name && entry.to_state_name && (
                        <Text type="secondary">
                          {' '}
                          ({entry.from_state_name} → {entry.to_state_name})
                        </Text>
                      )}
                      <br />
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        {entry.performed_by_name || 'System'} - {formatDate(entry.performed_at)}
                      </Text>
                      {entry.comments && (
                        <>
                          <br />
                          <Text italic style={{ fontSize: 13 }}>
                            {entry.comments}
                          </Text>
                        </>
                      )}
                    </div>
                  ),
                }))}
              />
            </>
          )}
        </Card>
      )}

      <Modal
        title={`Workflow Action: ${transitionAction}`}
        open={transitionModalOpen}
        onOk={submitTransition}
        onCancel={() => setTransitionModalOpen(false)}
        confirmLoading={actionLoading}
        okText="Confirm"
      >
        <div style={{ marginBottom: 12 }}>
          <Text>
            You are about to <strong>{transitionAction.toLowerCase()}</strong> this element.
          </Text>
        </div>
        <Input.TextArea
          rows={3}
          placeholder="Add comments (optional)"
          value={transitionComments}
          onChange={(e) => setTransitionComments(e.target.value)}
        />
      </Modal>

      <Modal
        title="Designate as Critical Data Element"
        open={cdeModalOpen}
        onOk={handleDesignateCde}
        onCancel={() => setCdeModalOpen(false)}
        confirmLoading={cdeLoading}
        okText="Designate as CDE"
        okButtonProps={{ danger: true }}
      >
        <div style={{ marginBottom: 12 }}>
          <Text>
            Designating this element as a CDE will flag it for enhanced data quality monitoring
            and governance. Please provide a rationale.
          </Text>
        </div>
        <Input.TextArea
          rows={4}
          placeholder="Rationale for CDE designation (e.g., regulatory requirement, critical for reporting)"
          value={cdeRationale}
          onChange={(e) => setCdeRationale(e.target.value)}
        />
      </Modal>
    </div>
  );
};

export default DataElementDetail;

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
  Form,
  Input,
  Modal,
  Row,
  Select,
  Space,
  Spin,
  Statistic,
  Switch,
  Table,
  Tag,
  Timeline,
  Typography,
  message,
} from 'antd';
import {
  ApiOutlined,
  ArrowLeftOutlined,
  CheckOutlined,
  CloseOutlined,
  EditOutlined,
  LinkOutlined,
  SendOutlined,
  UndoOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import { applicationsApi } from '../services/applicationsApi';
import { dataDictionaryApi } from '../services/dataDictionaryApi';
import { workflowApi } from '../services/glossaryApi';
import type {
  ApplicationDataElementLink,
  ApplicationFullView,
  ApplicationInterface,
} from '../services/applicationsApi';
import type { DataElementListItem } from '../services/dataDictionaryApi';
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

const deploymentTypeColors: Record<string, string> = {
  ON_PREMISE: 'default',
  CLOUD: 'blue',
  HYBRID: 'purple',
  SAAS: 'cyan',
};

const deploymentTypeLabels: Record<string, string> = {
  ON_PREMISE: 'On-Premise',
  CLOUD: 'Cloud',
  HYBRID: 'Hybrid',
  SAAS: 'SaaS',
};

const ApplicationDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();

  const [application, setApplication] = useState<ApplicationFullView | null>(null);
  const [linkedElements, setLinkedElements] = useState<ApplicationDataElementLink[]>([]);
  const [interfaces, setInterfaces] = useState<ApplicationInterface[]>([]);
  const [workflowInstance, setWorkflowInstance] = useState<WorkflowInstanceView | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [transitionModalOpen, setTransitionModalOpen] = useState(false);
  const [transitionAction, setTransitionAction] = useState('');
  const [transitionComments, setTransitionComments] = useState('');

  // Link data element modal
  const [linkModalOpen, setLinkModalOpen] = useState(false);
  const [linkForm] = Form.useForm();
  const [linkLoading, setLinkLoading] = useState(false);
  const [availableElements, setAvailableElements] = useState<DataElementListItem[]>([]);
  const [elementsLoading, setElementsLoading] = useState(false);

  const isSteward = user?.roles?.includes('data_steward') || user?.roles?.includes('admin');

  const fetchApplication = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await applicationsApi.getApplication(id);
      setApplication(response.data);

      // Fetch workflow instance if one exists
      if (response.data.workflow_instance_id) {
        try {
          const wfResponse = await workflowApi.getInstance(response.data.workflow_instance_id);
          setWorkflowInstance(wfResponse.data);
        } catch {
          // Workflow instance may not exist yet
        }
      }
    } catch {
      message.error('Failed to load application details.');
      navigate('/applications');
    } finally {
      setLoading(false);
    }
  }, [id, navigate]);

  const fetchLinkedElements = useCallback(async () => {
    if (!id) return;
    try {
      const response = await applicationsApi.listAppElements(id);
      setLinkedElements(response.data);
    } catch {
      // Non-critical
    }
  }, [id]);

  const fetchInterfaces = useCallback(async () => {
    if (!id) return;
    try {
      const response = await applicationsApi.listInterfaces(id);
      setInterfaces(response.data);
    } catch {
      // Non-critical
    }
  }, [id]);

  useEffect(() => {
    fetchApplication();
  }, [fetchApplication]);

  useEffect(() => {
    fetchLinkedElements();
  }, [fetchLinkedElements]);

  useEffect(() => {
    fetchInterfaces();
  }, [fetchInterfaces]);

  const fetchAvailableElements = async () => {
    setElementsLoading(true);
    try {
      const response = await dataDictionaryApi.listElements({ page_size: 500 });
      const data = response.data;
      if (Array.isArray(data)) {
        setAvailableElements(data);
      } else {
        const paginated = data as unknown as { data: DataElementListItem[] };
        setAvailableElements(paginated.data);
      }
    } catch {
      // Non-critical
    } finally {
      setElementsLoading(false);
    }
  };

  const handleWorkflowAction = (action: string) => {
    if (!application?.workflow_instance_id) {
      message.error('No active workflow for this application.');
      return;
    }
    setTransitionAction(action);
    setTransitionComments('');
    setTransitionModalOpen(true);
  };

  const submitTransition = async () => {
    if (!application?.workflow_instance_id) return;

    setActionLoading(true);
    try {
      await workflowApi.transitionWorkflow(
        application.workflow_instance_id,
        transitionAction,
        transitionComments || undefined,
      );
      message.success(`Workflow action "${transitionAction}" completed successfully.`);
      setTransitionModalOpen(false);
      fetchApplication();
    } catch {
      message.error(`Failed to perform action "${transitionAction}".`);
    } finally {
      setActionLoading(false);
    }
  };

  const handleLinkElement = async () => {
    try {
      const values = await linkForm.validateFields();
      setLinkLoading(true);
      await applicationsApi.linkDataElement(id!, {
        element_id: values.element_id,
        usage_type: values.usage_type,
        is_authoritative_source: values.is_authoritative_source || false,
        description: values.description || undefined,
      });
      message.success('Data element linked successfully.');
      setLinkModalOpen(false);
      linkForm.resetFields();
      fetchLinkedElements();
    } catch {
      message.error('Failed to link data element.');
    } finally {
      setLinkLoading(false);
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

  if (!application) {
    return null;
  }

  const status = application.status_code || 'DRAFT';

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
        onClick={() => navigate(`/applications/${id}/edit`)}
        disabled={status === 'ACCEPTED' || status === 'DEPRECATED'}
      >
        Edit
      </Button>,
    );

    return buttons;
  };

  const elementColumns = [
    {
      title: 'Element Name',
      dataIndex: 'element_name',
      key: 'element_name',
      render: (name: string, record: ApplicationDataElementLink) => (
        <a onClick={() => navigate(`/data-dictionary/${record.element_id}`)}>{name}</a>
      ),
    },
    {
      title: 'Element Code',
      dataIndex: 'element_code',
      key: 'element_code',
      width: 180,
      render: (code: string) => (
        <Text code style={{ fontSize: 12 }}>
          {code}
        </Text>
      ),
    },
    {
      title: 'Usage Type',
      dataIndex: 'usage_type',
      key: 'usage_type',
      width: 120,
      render: (type: string) => {
        const colors: Record<string, string> = {
          PRODUCER: 'green',
          CONSUMER: 'blue',
          BOTH: 'purple',
        };
        return <Tag color={colors[type] || 'default'}>{type}</Tag>;
      },
    },
    {
      title: 'Authoritative Source',
      dataIndex: 'is_authoritative_source',
      key: 'is_authoritative_source',
      width: 160,
      align: 'center' as const,
      render: (isAuth: boolean) =>
        isAuth ? (
          <Tag color="gold" style={{ fontWeight: 600 }}>
            Authoritative
          </Tag>
        ) : null,
    },
    {
      title: 'CDE',
      dataIndex: 'is_cde',
      key: 'is_cde',
      width: 80,
      align: 'center' as const,
      render: (isCde: boolean) =>
        isCde ? (
          <Tag color="red" style={{ fontWeight: 600 }}>
            CDE
          </Tag>
        ) : null,
    },
  ];

  const interfaceColumns = [
    {
      title: 'Interface Name',
      dataIndex: 'interface_name',
      key: 'interface_name',
    },
    {
      title: 'Source',
      dataIndex: 'source_app_name',
      key: 'source_app_name',
      render: (name: string, record: ApplicationInterface) => (
        <a onClick={() => navigate(`/applications/${record.source_app_id}`)}>{name}</a>
      ),
    },
    {
      title: '',
      key: 'arrow',
      width: 40,
      align: 'center' as const,
      render: () => <Text type="secondary">&rarr;</Text>,
    },
    {
      title: 'Target',
      dataIndex: 'target_app_name',
      key: 'target_app_name',
      render: (name: string, record: ApplicationInterface) => (
        <a onClick={() => navigate(`/applications/${record.target_app_id}`)}>{name}</a>
      ),
    },
    {
      title: 'Type',
      dataIndex: 'interface_type',
      key: 'interface_type',
      width: 130,
      render: (type: string) => <Tag>{type}</Tag>,
    },
    {
      title: 'Protocol',
      dataIndex: 'protocol',
      key: 'protocol',
      width: 100,
      render: (protocol: string | null) => protocol || '-',
    },
    {
      title: 'Frequency',
      dataIndex: 'frequency',
      key: 'frequency',
      width: 120,
      render: (freq: string | null) => freq || '-',
    },
  ];

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/applications')}>Applications</a> },
          { title: application.application_name },
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
            onClick={() => navigate('/applications')}
          />
          <Title level={3} style={{ margin: 0 }}>
            {application.application_name}
          </Title>
          <Tag
            color={statusColors[status] || 'default'}
            style={{ fontSize: 14, padding: '2px 12px' }}
          >
            {statusLabels[status] || status}
          </Tag>
          {application.is_critical && (
            <Tag color="red" style={{ fontSize: 14, padding: '2px 12px', fontWeight: 600 }}>
              Critical
            </Tag>
          )}
        </Space>
        <Space>{renderActionButtons()}</Space>
      </div>

      {application.is_critical && (
        <Alert
          type="error"
          showIcon
          icon={<WarningOutlined />}
          message="Critical Application"
          description={
            <div>
              <Text strong>Rationale: </Text>
              <Text>{application.criticality_rationale || 'No rationale provided.'}</Text>
            </div>
          }
          style={{ marginBottom: 24 }}
        />
      )}

      <Row gutter={16} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Linked Data Elements"
              value={application.data_elements_count}
              prefix={<LinkOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Interfaces"
              value={application.interfaces_count}
              prefix={<ApiOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Linked Processes"
              value={application.linked_processes?.length || 0}
            />
          </Card>
        </Col>
      </Row>

      <Card title="Application Details" style={{ marginBottom: 24 }}>
        <Descriptions column={{ xs: 1, sm: 1, md: 2 }} bordered size="small">
          <Descriptions.Item label="Application Name">{application.application_name}</Descriptions.Item>
          <Descriptions.Item label="Application Code">
            <Text code>{application.application_code}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Description" span={2}>
            {application.description}
          </Descriptions.Item>
          <Descriptions.Item label="Classification">
            {application.classification_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Vendor">
            {application.vendor || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Version">
            {application.version || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Deployment Type">
            {application.deployment_type ? (
              <Tag color={deploymentTypeColors[application.deployment_type] || 'default'}>
                {deploymentTypeLabels[application.deployment_type] || application.deployment_type}
              </Tag>
            ) : (
              '-'
            )}
          </Descriptions.Item>
          {application.technology_stack != null ? (
            <Descriptions.Item label="Technology Stack" span={2}>
              <Text code style={{ fontSize: 12, whiteSpace: 'pre-wrap' }}>
                {typeof application.technology_stack === 'string'
                  ? application.technology_stack
                  : JSON.stringify(application.technology_stack, null, 2)}
              </Text>
            </Descriptions.Item>
          ) : null}
          <Descriptions.Item label="Critical">
            {application.is_critical ? (
              <Tag color="red" style={{ fontWeight: 600 }}>
                Critical
              </Tag>
            ) : (
              <Text type="secondary">No</Text>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="Status">
            <Tag color={statusColors[status] || 'default'}>
              {statusLabels[status] || status}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Business Owner">
            {application.business_owner_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Technical Owner">
            {application.technical_owner_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Go-Live Date">
            {application.go_live_date
              ? new Date(application.go_live_date).toLocaleDateString('en-ZA', {
                  year: 'numeric',
                  month: 'short',
                  day: 'numeric',
                })
              : '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Retirement Date">
            {application.retirement_date
              ? new Date(application.retirement_date).toLocaleDateString('en-ZA', {
                  year: 'numeric',
                  month: 'short',
                  day: 'numeric',
                })
              : '-'}
          </Descriptions.Item>
          {application.documentation_url && (
            <Descriptions.Item label="Documentation" span={2}>
              <a href={application.documentation_url} target="_blank" rel="noopener noreferrer">
                {application.documentation_url}
              </a>
            </Descriptions.Item>
          )}
          <Descriptions.Item label="Created">
            {formatDate(application.created_at)}
            {application.created_by_name ? ` by ${application.created_by_name}` : ''}
          </Descriptions.Item>
          <Descriptions.Item label="Last Updated">
            {formatDate(application.updated_at)}
            {application.updated_by_name ? ` by ${application.updated_by_name}` : ''}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Card
        title="Linked Data Elements"
        style={{ marginBottom: 24 }}
        extra={
          <Button
            type="primary"
            size="small"
            icon={<LinkOutlined />}
            onClick={() => {
              linkForm.resetFields();
              fetchAvailableElements();
              setLinkModalOpen(true);
            }}
          >
            Link Data Element
          </Button>
        }
      >
        <Table
          columns={elementColumns}
          dataSource={linkedElements}
          rowKey="id"
          pagination={false}
          size="small"
          locale={{ emptyText: 'No data elements linked to this application.' }}
        />
      </Card>

      {interfaces.length > 0 && (
        <Card title="Interfaces" style={{ marginBottom: 24 }}>
          <Table
            columns={interfaceColumns}
            dataSource={interfaces}
            rowKey="interface_id"
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
                          ({entry.from_state_name} &rarr; {entry.to_state_name})
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

      {/* Workflow Transition Modal */}
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
            You are about to <strong>{transitionAction.toLowerCase()}</strong> this application.
          </Text>
        </div>
        <Input.TextArea
          rows={3}
          placeholder="Add comments (optional)"
          value={transitionComments}
          onChange={(e) => setTransitionComments(e.target.value)}
        />
      </Modal>

      {/* Link Data Element Modal */}
      <Modal
        title="Link Data Element"
        open={linkModalOpen}
        onOk={handleLinkElement}
        onCancel={() => setLinkModalOpen(false)}
        confirmLoading={linkLoading}
        okText="Link Element"
        width={600}
      >
        <Form form={linkForm} layout="vertical">
          <Form.Item
            name="element_id"
            label="Data Element"
            rules={[{ required: true, message: 'Please select a data element' }]}
          >
            <Select
              placeholder="Select a data element"
              showSearch
              optionFilterProp="label"
              loading={elementsLoading}
              options={availableElements.map((e) => ({
                value: e.element_id,
                label: `${e.element_name} (${e.element_code})`,
              }))}
            />
          </Form.Item>
          <Form.Item
            name="usage_type"
            label="Usage Type"
            rules={[{ required: true, message: 'Please select a usage type' }]}
            initialValue="BOTH"
          >
            <Select
              options={[
                { value: 'PRODUCER', label: 'Producer' },
                { value: 'CONSUMER', label: 'Consumer' },
                { value: 'BOTH', label: 'Both' },
              ]}
            />
          </Form.Item>
          <Form.Item
            name="is_authoritative_source"
            label="Authoritative Source"
            valuePropName="checked"
          >
            <Switch checkedChildren="Yes" unCheckedChildren="No" />
          </Form.Item>
          <Form.Item name="description" label="Description">
            <Input.TextArea rows={2} placeholder="Optional description of the relationship" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default ApplicationDetail;

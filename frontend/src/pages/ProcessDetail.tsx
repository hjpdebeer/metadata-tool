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
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Spin,
  Statistic,
  Table,
  Tag,
  Timeline,
  Typography,
  message,
} from 'antd';
import {
  AppstoreOutlined,
  ArrowLeftOutlined,
  CheckOutlined,
  CloseOutlined,
  EditOutlined,
  LinkOutlined,
  OrderedListOutlined,
  PlusOutlined,
  SendOutlined,
  UndoOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import { processesApi } from '../services/processesApi';
import { dataDictionaryApi } from '../services/dataDictionaryApi';
import { applicationsApi } from '../services/applicationsApi';
import { workflowApi } from '../services/glossaryApi';
import type {
  BusinessProcessFullView,
  ProcessApplicationLink,
  ProcessDataElementLink,
  ProcessStep,
} from '../services/processesApi';
import type { DataElementListItem } from '../services/dataDictionaryApi';
import type { ApplicationListItem } from '../services/applicationsApi';
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

const frequencyLabels: Record<string, string> = {
  DAILY: 'Daily',
  WEEKLY: 'Weekly',
  MONTHLY: 'Monthly',
  QUARTERLY: 'Quarterly',
  ANNUAL: 'Annual',
  ON_DEMAND: 'On Demand',
};

const ProcessDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();

  const [process, setProcess] = useState<BusinessProcessFullView | null>(null);
  const [linkedElements, setLinkedElements] = useState<ProcessDataElementLink[]>([]);
  const [linkedApps, setLinkedApps] = useState<ProcessApplicationLink[]>([]);
  const [workflowInstance, setWorkflowInstance] = useState<WorkflowInstanceView | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [transitionModalOpen, setTransitionModalOpen] = useState(false);
  const [transitionAction, setTransitionAction] = useState('');
  const [transitionComments, setTransitionComments] = useState('');

  // Add step modal
  const [stepModalOpen, setStepModalOpen] = useState(false);
  const [stepForm] = Form.useForm();
  const [stepLoading, setStepLoading] = useState(false);
  const [availableApps, setAvailableApps] = useState<ApplicationListItem[]>([]);

  // Link data element modal
  const [linkElementModalOpen, setLinkElementModalOpen] = useState(false);
  const [linkElementForm] = Form.useForm();
  const [linkElementLoading, setLinkElementLoading] = useState(false);
  const [availableElements, setAvailableElements] = useState<DataElementListItem[]>([]);
  const [elementsLoading, setElementsLoading] = useState(false);

  // Link application modal
  const [linkAppModalOpen, setLinkAppModalOpen] = useState(false);
  const [linkAppForm] = Form.useForm();
  const [linkAppLoading, setLinkAppLoading] = useState(false);
  const [appsLoading, setAppsLoading] = useState(false);

  const isSteward = user?.roles?.includes('data_steward') || user?.roles?.includes('admin');

  const fetchProcess = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await processesApi.getProcess(id);
      setProcess(response.data);

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
      message.error('Failed to load process details.');
      navigate('/processes');
    } finally {
      setLoading(false);
    }
  }, [id, navigate]);

  const fetchLinkedElements = useCallback(async () => {
    if (!id) return;
    try {
      const response = await processesApi.listProcessElements(id);
      setLinkedElements(response.data);
    } catch {
      // Non-critical
    }
  }, [id]);

  const fetchLinkedApps = useCallback(async () => {
    if (!id) return;
    try {
      const response = await processesApi.listProcessApplications(id);
      setLinkedApps(response.data);
    } catch {
      // Non-critical
    }
  }, [id]);

  useEffect(() => {
    fetchProcess();
  }, [fetchProcess]);

  useEffect(() => {
    fetchLinkedElements();
  }, [fetchLinkedElements]);

  useEffect(() => {
    fetchLinkedApps();
  }, [fetchLinkedApps]);

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

  const fetchAvailableApps = async () => {
    setAppsLoading(true);
    try {
      const response = await applicationsApi.listApplications({ page_size: 500 });
      const data = response.data;
      if (Array.isArray(data)) {
        setAvailableApps(data);
      } else {
        const paginated = data as unknown as { data: ApplicationListItem[] };
        setAvailableApps(paginated.data);
      }
    } catch {
      // Non-critical
    } finally {
      setAppsLoading(false);
    }
  };

  const handleWorkflowAction = (action: string) => {
    if (!process?.workflow_instance_id) {
      message.error('No active workflow for this process.');
      return;
    }
    setTransitionAction(action);
    setTransitionComments('');
    setTransitionModalOpen(true);
  };

  const submitTransition = async () => {
    if (!process?.workflow_instance_id) return;

    setActionLoading(true);
    try {
      await workflowApi.transitionWorkflow(
        process.workflow_instance_id,
        transitionAction,
        transitionComments || undefined,
      );
      message.success(`Workflow action "${transitionAction}" completed successfully.`);
      setTransitionModalOpen(false);
      fetchProcess();
    } catch {
      message.error(`Failed to perform action "${transitionAction}".`);
    } finally {
      setActionLoading(false);
    }
  };

  const handleAddStep = async () => {
    try {
      const values = await stepForm.validateFields();
      setStepLoading(true);
      await processesApi.addStep(id!, {
        step_number: values.step_number,
        step_name: values.step_name,
        description: values.description || undefined,
        responsible_role: values.responsible_role || undefined,
        application_id: values.application_id || undefined,
      });
      message.success('Process step added successfully.');
      setStepModalOpen(false);
      stepForm.resetFields();
      fetchProcess();
    } catch {
      message.error('Failed to add process step.');
    } finally {
      setStepLoading(false);
    }
  };

  const handleLinkElement = async () => {
    try {
      const values = await linkElementForm.validateFields();
      setLinkElementLoading(true);
      await processesApi.linkDataElement(id!, {
        element_id: values.element_id,
        usage_type: values.usage_type,
        is_required: values.is_required ?? true,
        description: values.description || undefined,
      });
      message.success('Data element linked successfully.');
      setLinkElementModalOpen(false);
      linkElementForm.resetFields();
      fetchLinkedElements();
    } catch {
      message.error('Failed to link data element.');
    } finally {
      setLinkElementLoading(false);
    }
  };

  const handleLinkApp = async () => {
    try {
      const values = await linkAppForm.validateFields();
      setLinkAppLoading(true);
      await processesApi.linkApplication(id!, {
        application_id: values.application_id,
        role_in_process: values.role_in_process || undefined,
        description: values.description || undefined,
      });
      message.success('Application linked successfully.');
      setLinkAppModalOpen(false);
      linkAppForm.resetFields();
      fetchLinkedApps();
    } catch {
      message.error('Failed to link application.');
    } finally {
      setLinkAppLoading(false);
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

  if (!process) {
    return null;
  }

  const status = process.status_code || 'DRAFT';

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
        onClick={() => navigate(`/processes/${id}/edit`)}
        disabled={status === 'ACCEPTED' || status === 'DEPRECATED'}
      >
        Edit
      </Button>,
    );

    return buttons;
  };

  const stepColumns = [
    {
      title: '#',
      dataIndex: 'step_number',
      key: 'step_number',
      width: 60,
      align: 'center' as const,
      render: (num: number) => (
        <Tag color="blue" style={{ fontWeight: 600, minWidth: 32, textAlign: 'center' }}>
          {num}
        </Tag>
      ),
    },
    {
      title: 'Step Name',
      dataIndex: 'step_name',
      key: 'step_name',
    },
    {
      title: 'Description',
      dataIndex: 'description',
      key: 'description',
      render: (desc: string | null) => desc || '-',
    },
    {
      title: 'Responsible Role',
      dataIndex: 'responsible_role',
      key: 'responsible_role',
      width: 160,
      render: (role: string | null) => role || '-',
    },
    {
      title: 'Application',
      dataIndex: 'application_name',
      key: 'application_name',
      width: 160,
      render: (name: string | null, record: ProcessStep) =>
        name ? (
          <a onClick={() => navigate(`/applications/${record.application_id}`)}>{name}</a>
        ) : (
          '-'
        ),
    },
  ];

  const elementColumns = [
    {
      title: 'Element Name',
      dataIndex: 'element_name',
      key: 'element_name',
      render: (name: string, record: ProcessDataElementLink) => (
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
          INPUT: 'green',
          OUTPUT: 'blue',
          BOTH: 'purple',
        };
        return <Tag color={colors[type] || 'default'}>{type}</Tag>;
      },
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
    {
      title: 'Required',
      dataIndex: 'is_required',
      key: 'is_required',
      width: 90,
      align: 'center' as const,
      render: (isRequired: boolean) => (isRequired ? 'Yes' : 'No'),
    },
  ];

  const appColumns = [
    {
      title: 'Application Name',
      dataIndex: 'application_name',
      key: 'application_name',
      render: (name: string, record: ProcessApplicationLink) => (
        <a onClick={() => navigate(`/applications/${record.application_id}`)}>{name}</a>
      ),
    },
    {
      title: 'Application Code',
      dataIndex: 'application_code',
      key: 'application_code',
      width: 160,
      render: (code: string) => (
        <Text code style={{ fontSize: 12 }}>
          {code}
        </Text>
      ),
    },
    {
      title: 'Role in Process',
      dataIndex: 'role_in_process',
      key: 'role_in_process',
      width: 200,
      render: (role: string | null) => role || '-',
    },
  ];

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/processes')}>Business Processes</a> },
          { title: process.process_name },
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
            onClick={() => navigate('/processes')}
          />
          <Title level={3} style={{ margin: 0 }}>
            {process.process_name}
          </Title>
          <Tag
            color={statusColors[status] || 'default'}
            style={{ fontSize: 14, padding: '2px 12px' }}
          >
            {statusLabels[status] || status}
          </Tag>
          {process.is_critical && (
            <Tag color="red" style={{ fontSize: 14, padding: '2px 12px', fontWeight: 600 }}>
              Critical Business Process
            </Tag>
          )}
        </Space>
        <Space>{renderActionButtons()}</Space>
      </div>

      {process.is_critical && (
        <Alert
          type="error"
          showIcon
          icon={<WarningOutlined />}
          message="Critical Business Process"
          description={
            <div>
              <Text>
                All data elements linked to this process are automatically designated as
                Critical Data Elements (CDEs). This ensures enhanced data quality monitoring
                and governance for data supporting critical business operations.
              </Text>
              {process.criticality_rationale && (
                <>
                  <br />
                  <br />
                  <Text strong>Rationale: </Text>
                  <Text>{process.criticality_rationale}</Text>
                </>
              )}
            </div>
          }
          style={{ marginBottom: 24 }}
        />
      )}

      <Row gutter={16} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Process Steps"
              value={process.steps?.length || 0}
              prefix={<OrderedListOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Linked Data Elements"
              value={process.data_elements_count}
              prefix={<LinkOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Linked Applications"
              value={process.linked_applications?.length || 0}
              prefix={<AppstoreOutlined />}
            />
          </Card>
        </Col>
      </Row>

      <Card title="Process Details" style={{ marginBottom: 24 }}>
        <Descriptions column={{ xs: 1, sm: 1, md: 2 }} bordered size="small">
          <Descriptions.Item label="Process Name">{process.process_name}</Descriptions.Item>
          <Descriptions.Item label="Process Code">
            <Text code>{process.process_code}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Description" span={2}>
            {process.description}
          </Descriptions.Item>
          {process.detailed_description && (
            <Descriptions.Item label="Detailed Description" span={2}>
              {process.detailed_description}
            </Descriptions.Item>
          )}
          <Descriptions.Item label="Category">
            {process.category_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Frequency">
            {process.frequency
              ? (frequencyLabels[process.frequency] || process.frequency)
              : '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Critical">
            {process.is_critical ? (
              <Tag color="red" style={{ fontWeight: 600 }}>
                Critical Business Process
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
          <Descriptions.Item label="Owner">
            {process.owner_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Parent Process">
            {process.parent_process_name ? (
              <a onClick={() => navigate(`/processes/${process.parent_process_id}`)}>
                {process.parent_process_name}
              </a>
            ) : (
              '-'
            )}
          </Descriptions.Item>
          {process.regulatory_requirement && (
            <Descriptions.Item label="Regulatory Requirement" span={2}>
              {process.regulatory_requirement}
            </Descriptions.Item>
          )}
          {process.sla_description && (
            <Descriptions.Item label="SLA Description" span={2}>
              {process.sla_description}
            </Descriptions.Item>
          )}
          {process.documentation_url && (
            <Descriptions.Item label="Documentation" span={2}>
              <a href={process.documentation_url} target="_blank" rel="noopener noreferrer">
                {process.documentation_url}
              </a>
            </Descriptions.Item>
          )}
          <Descriptions.Item label="Created">
            {formatDate(process.created_at)}
            {process.created_by_name ? ` by ${process.created_by_name}` : ''}
          </Descriptions.Item>
          <Descriptions.Item label="Last Updated">
            {formatDate(process.updated_at)}
            {process.updated_by_name ? ` by ${process.updated_by_name}` : ''}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Card
        title="Process Steps"
        style={{ marginBottom: 24 }}
        extra={
          <Button
            type="primary"
            size="small"
            icon={<PlusOutlined />}
            onClick={() => {
              stepForm.resetFields();
              const nextStepNumber = (process.steps?.length || 0) + 1;
              stepForm.setFieldsValue({ step_number: nextStepNumber });
              fetchAvailableApps();
              setStepModalOpen(true);
            }}
          >
            Add Step
          </Button>
        }
      >
        <Table
          columns={stepColumns}
          dataSource={[...(process.steps || [])].sort((a, b) => a.step_number - b.step_number)}
          rowKey="step_id"
          pagination={false}
          size="small"
          locale={{ emptyText: 'No process steps defined yet.' }}
        />
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
              linkElementForm.resetFields();
              fetchAvailableElements();
              setLinkElementModalOpen(true);
            }}
          >
            Link Data Element
          </Button>
        }
      >
        {process.is_critical && linkedElements.length === 0 && (
          <Alert
            type="info"
            message="CDE Auto-Propagation Active"
            description="Any data elements linked to this critical business process will be automatically designated as Critical Data Elements (CDEs)."
            style={{ marginBottom: 16 }}
            showIcon
          />
        )}
        <Table
          columns={elementColumns}
          dataSource={linkedElements}
          rowKey="id"
          pagination={false}
          size="small"
          locale={{ emptyText: 'No data elements linked to this process.' }}
        />
      </Card>

      <Card
        title="Linked Applications"
        style={{ marginBottom: 24 }}
        extra={
          <Button
            type="primary"
            size="small"
            icon={<AppstoreOutlined />}
            onClick={() => {
              linkAppForm.resetFields();
              fetchAvailableApps();
              setLinkAppModalOpen(true);
            }}
          >
            Link Application
          </Button>
        }
      >
        <Table
          columns={appColumns}
          dataSource={linkedApps}
          rowKey="id"
          pagination={false}
          size="small"
          locale={{ emptyText: 'No applications linked to this process.' }}
        />
      </Card>

      {process.sub_processes && process.sub_processes.length > 0 && (
        <Card title="Sub-Processes" style={{ marginBottom: 24 }}>
          <Table
            columns={[
              {
                title: 'Process Name',
                dataIndex: 'process_name',
                key: 'process_name',
                render: (name: string, record: { process_id: string }) => (
                  <a onClick={() => navigate(`/processes/${record.process_id}`)}>{name}</a>
                ),
              },
              {
                title: 'Process Code',
                dataIndex: 'process_code',
                key: 'process_code',
                width: 150,
                render: (code: string) => (
                  <Text code style={{ fontSize: 12 }}>
                    {code}
                  </Text>
                ),
              },
              {
                title: 'Critical',
                dataIndex: 'is_critical',
                key: 'is_critical',
                width: 100,
                align: 'center' as const,
                render: (isCritical: boolean) =>
                  isCritical ? (
                    <Tag color="red" style={{ fontWeight: 600 }}>
                      Critical
                    </Tag>
                  ) : null,
              },
              {
                title: 'Description',
                dataIndex: 'description',
                key: 'description',
                ellipsis: true,
              },
            ]}
            dataSource={process.sub_processes}
            rowKey="process_id"
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
            You are about to <strong>{transitionAction.toLowerCase()}</strong> this process.
          </Text>
        </div>
        <Input.TextArea
          rows={3}
          placeholder="Add comments (optional)"
          value={transitionComments}
          onChange={(e) => setTransitionComments(e.target.value)}
        />
      </Modal>

      {/* Add Step Modal */}
      <Modal
        title="Add Process Step"
        open={stepModalOpen}
        onOk={handleAddStep}
        onCancel={() => setStepModalOpen(false)}
        confirmLoading={stepLoading}
        okText="Add Step"
        width={600}
      >
        <Form form={stepForm} layout="vertical">
          <Form.Item
            name="step_number"
            label="Step Number"
            rules={[{ required: true, message: 'Step number is required' }]}
          >
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="step_name"
            label="Step Name"
            rules={[{ required: true, message: 'Step name is required' }]}
          >
            <Input placeholder="Enter the step name" />
          </Form.Item>
          <Form.Item name="description" label="Description">
            <Input.TextArea rows={3} placeholder="Describe what happens in this step" />
          </Form.Item>
          <Form.Item name="responsible_role" label="Responsible Role">
            <Input placeholder="e.g., Operations Manager, Data Analyst" />
          </Form.Item>
          <Form.Item name="application_id" label="Application">
            <Select
              placeholder="Select the application used in this step"
              showSearch
              optionFilterProp="label"
              loading={appsLoading}
              allowClear
              options={availableApps.map((a) => ({
                value: a.application_id,
                label: `${a.application_name} (${a.application_code})`,
              }))}
            />
          </Form.Item>
        </Form>
      </Modal>

      {/* Link Data Element Modal */}
      <Modal
        title="Link Data Element"
        open={linkElementModalOpen}
        onOk={handleLinkElement}
        onCancel={() => setLinkElementModalOpen(false)}
        confirmLoading={linkElementLoading}
        okText="Link Element"
        width={600}
      >
        {process.is_critical && (
          <Alert
            type="warning"
            message="CDE Auto-Designation"
            description="This is a critical business process. The linked data element will be automatically designated as a Critical Data Element (CDE)."
            style={{ marginBottom: 16 }}
            showIcon
            icon={<WarningOutlined />}
          />
        )}
        <Form form={linkElementForm} layout="vertical">
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
                { value: 'INPUT', label: 'Input' },
                { value: 'OUTPUT', label: 'Output' },
                { value: 'BOTH', label: 'Both' },
              ]}
            />
          </Form.Item>
          <Form.Item name="description" label="Description">
            <Input.TextArea rows={2} placeholder="Optional description of the relationship" />
          </Form.Item>
        </Form>
      </Modal>

      {/* Link Application Modal */}
      <Modal
        title="Link Application"
        open={linkAppModalOpen}
        onOk={handleLinkApp}
        onCancel={() => setLinkAppModalOpen(false)}
        confirmLoading={linkAppLoading}
        okText="Link Application"
        width={600}
      >
        <Form form={linkAppForm} layout="vertical">
          <Form.Item
            name="application_id"
            label="Application"
            rules={[{ required: true, message: 'Please select an application' }]}
          >
            <Select
              placeholder="Select an application"
              showSearch
              optionFilterProp="label"
              loading={appsLoading}
              options={availableApps.map((a) => ({
                value: a.application_id,
                label: `${a.application_name} (${a.application_code})`,
              }))}
            />
          </Form.Item>
          <Form.Item name="role_in_process" label="Role in Process">
            <Input placeholder="e.g., Primary data source, Processing engine, Reporting tool" />
          </Form.Item>
          <Form.Item name="description" label="Description">
            <Input.TextArea rows={2} placeholder="Optional description of how this application is used" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default ProcessDetail;

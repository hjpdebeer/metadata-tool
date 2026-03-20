import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  Alert,
  Breadcrumb,
  Button,
  Card,
  Col,
  Collapse,
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
  UserOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import { applicationsApi } from '../services/applicationsApi';
import { dataDictionaryApi } from '../services/dataDictionaryApi';
import { workflowApi } from '../services/glossaryApi';
import { usersApi } from '../services/usersApi';
import type {
  ApplicationDataElementLink,
  ApplicationFullView,
  ApplicationInterface,
} from '../services/applicationsApi';
import type { DataElementListItem } from '../services/dataDictionaryApi';
import type { WorkflowInstanceView, OrganisationalUnit } from '../services/glossaryApi';
import type { UserListItem } from '../services/usersApi';
import { useAuth } from '../hooks/useAuth';
import { glossaryApi } from '../services/glossaryApi';

const { Title, Text } = Typography;

const statusColors: Record<string, string> = {
  DRAFT: 'default',
  PROPOSED: 'processing',
  UNDER_REVIEW: 'warning',
  PENDING_APPROVAL: 'processing',
  REVISED: 'orange',
  ACCEPTED: 'success',
  REJECTED: 'error',
  DEPRECATED: 'default',
};

const statusLabels: Record<string, string> = {
  DRAFT: 'Draft',
  PROPOSED: 'Proposed',
  UNDER_REVIEW: 'Under Review',
  PENDING_APPROVAL: 'Pending Approval',
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

/** Placeholder for empty values */
const EmptyValue: React.FC<{ text?: string }> = ({ text }) => (
  <Text type="secondary" italic style={{ fontSize: 13 }}>
    {text || 'Not yet populated'}
  </Text>
);

const ApplicationDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();
  const currentUserId = user?.user_id;

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

  // Ownership assignment state
  const [allUsers, setAllUsers] = useState<UserListItem[]>([]);
  const [allOrgUnits, setAllOrgUnits] = useState<OrganisationalUnit[]>([]);
  const [ownershipLoading, setOwnershipLoading] = useState(false);
  type LabeledValue = { value: string; label: string };
  const [businessOwnerId, setBusinessOwnerId] = useState<LabeledValue | undefined>();
  const [technicalOwnerId, setTechnicalOwnerId] = useState<LabeledValue | undefined>();
  const [stewardUserId, setStewardUserId] = useState<LabeledValue | undefined>();
  const [approverUserId, setApproverUserId] = useState<LabeledValue | undefined>();
  const [orgUnit, setOrgUnit] = useState<string | undefined>();

  const fetchApplication = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await applicationsApi.getApplication(id);
      setApplication(response.data);
    } catch {
      message.error('Failed to load application details.');
      navigate('/applications');
    } finally {
      setLoading(false);
    }
  }, [id, navigate]);

  const fetchWorkflowInstance = useCallback(async () => {
    if (!id) return;
    try {
      const response = await workflowApi.getInstanceByEntity(id);
      setWorkflowInstance(response.data);
    } catch {
      setWorkflowInstance(null);
    }
  }, [id]);

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

  const fetchLookups = useCallback(async () => {
    const [usersRes, orgRes] = await Promise.allSettled([
      usersApi.listUsers({ page_size: 500, is_active: true }),
      glossaryApi.listOrganisationalUnits(),
    ]);
    if (usersRes.status === 'fulfilled') {
      const data = usersRes.value.data;
      setAllUsers(Array.isArray(data) ? data : (data as unknown as { data: UserListItem[] }).data || []);
    }
    if (orgRes.status === 'fulfilled') setAllOrgUnits(orgRes.value.data);
  }, []);

  useEffect(() => {
    fetchApplication();
    fetchWorkflowInstance();
    fetchLookups();
  }, [fetchApplication, fetchWorkflowInstance, fetchLookups]);

  useEffect(() => {
    fetchLinkedElements();
  }, [fetchLinkedElements]);

  useEffect(() => {
    fetchInterfaces();
  }, [fetchInterfaces]);

  // Sync ownership state from response — uses resolved names so UUIDs never display
  useEffect(() => {
    if (application) {
      setBusinessOwnerId(application.business_owner_id && application.business_owner_name
        ? { value: application.business_owner_id, label: application.business_owner_name } : undefined);
      setTechnicalOwnerId(application.technical_owner_id && application.technical_owner_name
        ? { value: application.technical_owner_id, label: application.technical_owner_name } : undefined);
      setStewardUserId(application.steward_user_id && application.steward_name
        ? { value: application.steward_user_id, label: application.steward_name } : undefined);
      setApproverUserId(application.approver_user_id && application.approver_name
        ? { value: application.approver_user_id, label: application.approver_name } : undefined);
      setOrgUnit(application.organisational_unit || undefined);
    }
  }, [application]);

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

  // --- Ownership assignment ---

  const handleSaveOwnership = async () => {
    if (!id) return;
    setOwnershipLoading(true);
    try {
      await applicationsApi.updateApplication(id, {
        business_owner_id: businessOwnerId?.value || undefined,
        technical_owner_id: technicalOwnerId?.value || undefined,
        steward_user_id: stewardUserId?.value || undefined,
        approver_user_id: approverUserId?.value || undefined,
        organisational_unit: orgUnit || undefined,
      });
      message.success('Ownership updated successfully.');
      fetchApplication();
    } catch {
      message.error('Failed to update ownership.');
    } finally {
      setOwnershipLoading(false);
    }
  };

  const ownershipComplete = !!(businessOwnerId && technicalOwnerId && stewardUserId && approverUserId);

  // --- Workflow ---

  const handleWorkflowAction = (action: string) => {
    if (!workflowInstance?.instance_id) {
      message.error('No active workflow for this application.');
      return;
    }
    if (action === 'SUBMIT') {
      const missing: string[] = [];
      if (!application?.business_owner_id) missing.push('Business Owner');
      if (!application?.technical_owner_id) missing.push('Technical Owner');
      if (!application?.steward_user_id) missing.push('Data Steward');
      if (!application?.approver_user_id) missing.push('Approver');
      if (missing.length > 0) {
        message.warning(
          `Please assign all ownership fields before submitting: ${missing.join(', ')}. Use the Ownership section below to assign owners.`,
          8,
        );
        return;
      }
    }
    setTransitionAction(action);
    setTransitionComments('');
    setTransitionModalOpen(true);
  };

  const submitTransition = async () => {
    if (!workflowInstance?.instance_id) return;

    setActionLoading(true);
    try {
      await workflowApi.transitionWorkflow(
        workflowInstance.instance_id,
        transitionAction,
        transitionComments || undefined,
      );
      message.success(`Workflow action "${transitionAction}" completed successfully.`);
      setTransitionModalOpen(false);
      fetchApplication();
      fetchWorkflowInstance();
    } catch (error: unknown) {
      const apiMsg = (error as { response?: { data?: { error?: { message?: string } } } })
        ?.response?.data?.error?.message;
      message.error(apiMsg || `Failed to perform action "${transitionAction}".`, 8);
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
    if (!dateStr) return null;
    return new Date(dateStr).toLocaleString('en-ZA', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const formatDateShort = (dateStr: string | null | undefined) => {
    if (!dateStr) return null;
    return new Date(dateStr).toLocaleDateString('en-ZA', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
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

  const detail = application;
  const status = application.status_code || 'DRAFT';

  // --- Action buttons (matching glossary pattern) ---

  const renderActionButtons = () => {
    const buttons: React.ReactNode[] = [];
    const isAdmin = user?.roles?.includes('admin') || user?.roles?.includes('ADMIN');
    const isSteward = currentUserId === detail.steward_user_id || isAdmin;
    const isOwner = currentUserId === detail.business_owner_id || isAdmin;

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
          Approve (Steward)
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

    if (status === 'PENDING_APPROVAL' && isOwner) {
      buttons.push(
        <Button
          key="final-approve"
          type="primary"
          icon={<CheckOutlined />}
          style={{ backgroundColor: '#52C41A', borderColor: '#52C41A' }}
          onClick={() => handleWorkflowAction('APPROVE')}
        >
          Final Approval (Owner)
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
          key="return"
          icon={<UndoOutlined />}
          onClick={() => handleWorkflowAction('REVISE')}
        >
          Return to Steward
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

  const showOwnershipSection = allUsers.length > 0 && (status === 'DRAFT' || status === 'REVISED');

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

  // --- Collapse panels ---

  const collapseItems = [
    {
      key: 'core',
      label: <Text strong>Core Identity</Text>,
      children: (
        <Descriptions column={{ xs: 1, sm: 2, md: 3 }} bordered size="small">
          <Descriptions.Item label="Application Name">{detail.application_name}</Descriptions.Item>
          <Descriptions.Item label="Application Code">
            <Text code>{detail.application_code}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Abbreviation">
            {detail.abbreviation || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="External Reference ID">
            {detail.external_reference_id || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Description" span={2}>
            {detail.description}
          </Descriptions.Item>
        </Descriptions>
      ),
    },
    {
      key: 'classification',
      label: <Text strong>Classification</Text>,
      children: (
        <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small">
          <Descriptions.Item label="Classification">
            {detail.classification_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Deployment Type">
            {detail.deployment_type ? (
              <Tag color={deploymentTypeColors[detail.deployment_type] || 'default'}>
                {deploymentTypeLabels[detail.deployment_type] || detail.deployment_type}
              </Tag>
            ) : (
              <EmptyValue />
            )}
          </Descriptions.Item>
          <Descriptions.Item label="Lifecycle Stage">
            {detail.lifecycle_stage_name ? (
              <Tag color="blue">{detail.lifecycle_stage_name}</Tag>
            ) : (
              <EmptyValue />
            )}
          </Descriptions.Item>
          {detail.technology_stack != null ? (
            <Descriptions.Item label="Technology Stack" span={2}>
              <Text code style={{ fontSize: 12, whiteSpace: 'pre-wrap' }}>
                {typeof detail.technology_stack === 'string'
                  ? detail.technology_stack
                  : JSON.stringify(detail.technology_stack, null, 2)}
              </Text>
            </Descriptions.Item>
          ) : (
            <Descriptions.Item label="Technology Stack">
              <EmptyValue />
            </Descriptions.Item>
          )}
        </Descriptions>
      ),
    },
    {
      key: 'vendor',
      label: <Text strong>Vendor & Product</Text>,
      children: (
        <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small">
          <Descriptions.Item label="Vendor">
            {detail.vendor || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Vendor Product Name">
            {detail.vendor_product_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Version">
            {detail.version || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="License Type">
            {detail.license_type || <EmptyValue />}
          </Descriptions.Item>
          {detail.documentation_url && (
            <Descriptions.Item label="Documentation" span={2}>
              <a href={detail.documentation_url} target="_blank" rel="noopener noreferrer">
                {detail.documentation_url} <LinkOutlined />
              </a>
            </Descriptions.Item>
          )}
        </Descriptions>
      ),
    },
    {
      key: 'business',
      label: <Text strong>Business Context</Text>,
      children: (
        <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small">
          <Descriptions.Item label="Business Capability">
            {detail.business_capability || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="User Base">
            {detail.user_base || <EmptyValue />}
          </Descriptions.Item>
        </Descriptions>
      ),
    },
    {
      key: 'ownership',
      label: <Text strong>Ownership</Text>,
      children: (
        <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small">
          <Descriptions.Item label="Business Owner">
            {detail.business_owner_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Technical Owner">
            {detail.technical_owner_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Data Steward">
            {detail.steward_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Approver">
            {detail.approver_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Organisational Unit" span={2}>
            {detail.organisational_unit || <EmptyValue />}
          </Descriptions.Item>
        </Descriptions>
      ),
    },
    {
      key: 'criticality',
      label: <Text strong>Criticality & Risk</Text>,
      children: (
        <div>
          {detail.is_cba && (
            <Alert
              message="Critical Business Application"
              description={detail.cba_rationale || 'No rationale provided.'}
              type="error"
              showIcon
              icon={<WarningOutlined />}
              style={{ marginBottom: 16 }}
            />
          )}
          <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small">
            <Descriptions.Item label="CBA Designation">
              {detail.is_cba ? (
                <Tag color="red" style={{ fontWeight: 600 }}>
                  <WarningOutlined /> Critical Business Application
                </Tag>
              ) : (
                <Tag color="default">Not CBA</Tag>
              )}
            </Descriptions.Item>
            <Descriptions.Item label="CBA Rationale">
              {detail.cba_rationale || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label="Criticality Tier">
              {detail.criticality_tier_name ? (
                <Tag color="volcano">{detail.criticality_tier_name}</Tag>
              ) : (
                <EmptyValue />
              )}
            </Descriptions.Item>
            <Descriptions.Item label="Risk Rating">
              {detail.risk_rating_name ? (
                <Tag color={
                  detail.risk_rating_name === 'Critical' ? 'red' :
                  detail.risk_rating_name === 'High' ? 'volcano' :
                  detail.risk_rating_name === 'Medium' ? 'gold' : 'green'
                }>
                  {detail.risk_rating_name}
                </Tag>
              ) : (
                <EmptyValue />
              )}
            </Descriptions.Item>
            <Descriptions.Item label="Data Classification">
              {detail.data_classification_name ? (
                <Tag color="volcano">{detail.data_classification_name}</Tag>
              ) : (
                <EmptyValue />
              )}
            </Descriptions.Item>
            <Descriptions.Item label="Regulatory Scope">
              {detail.regulatory_scope || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label="Last Security Assessment">
              {formatDateShort(detail.last_security_assessment) || <EmptyValue />}
            </Descriptions.Item>
          </Descriptions>
        </div>
      ),
    },
    {
      key: 'operational',
      label: <Text strong>Operational</Text>,
      children: (
        <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small">
          <Descriptions.Item label="Support Model">
            {detail.support_model || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="DR Tier">
            {detail.dr_tier_name ? (
              <Tag color="purple">{detail.dr_tier_name}</Tag>
            ) : (
              <EmptyValue />
            )}
          </Descriptions.Item>
          <Descriptions.Item label="RTO / RPO">
            {detail.dr_tier_rto_hours != null && detail.dr_tier_rpo_minutes != null ? (
              <span>
                RTO: <Text strong>{detail.dr_tier_rto_hours}h</Text>{' '}
                / RPO: <Text strong>{detail.dr_tier_rpo_minutes}min</Text>
              </span>
            ) : (
              <EmptyValue />
            )}
          </Descriptions.Item>
        </Descriptions>
      ),
    },
    {
      key: 'lifecycle',
      label: <Text strong>Lifecycle</Text>,
      children: (
        <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small">
          <Descriptions.Item label="Go-Live Date">
            {formatDateShort(detail.go_live_date) || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Retirement Date">
            {formatDateShort(detail.retirement_date) || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Contract End Date">
            {formatDateShort(detail.contract_end_date) || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Review Frequency">
            {detail.review_frequency_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Next Review Date">
            {formatDateShort(detail.next_review_date) || <EmptyValue text="Not scheduled" />}
          </Descriptions.Item>
          <Descriptions.Item label="Approved Date">
            {formatDate(detail.approved_at) || <EmptyValue text="Not yet approved" />}
          </Descriptions.Item>
          <Descriptions.Item label="Created">
            {formatDate(detail.created_at)}
            {detail.created_by_name ? ` by ${detail.created_by_name}` : ''}
          </Descriptions.Item>
          <Descriptions.Item label="Last Updated">
            {formatDate(detail.updated_at)}
            {detail.updated_by_name ? ` by ${detail.updated_by_name}` : ''}
          </Descriptions.Item>
        </Descriptions>
      ),
    },
    {
      key: 'relationships',
      label: <Text strong>Relationships</Text>,
      children: (
        <Descriptions column={{ xs: 1, sm: 3 }} bordered size="small">
          <Descriptions.Item label="Linked Data Elements">
            <Text strong>{detail.data_elements_count}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Interfaces">
            <Text strong>{detail.interfaces_count}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Linked Processes">
            {detail.linked_processes?.length > 0 ? (
              <Space wrap size={[4, 4]}>
                {detail.linked_processes.map((name, idx) => (
                  <Tag key={idx} color="green">{name}</Tag>
                ))}
              </Space>
            ) : (
              <EmptyValue text="None" />
            )}
          </Descriptions.Item>
        </Descriptions>
      ),
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

      {/* --- Header --- */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'flex-start',
          marginBottom: 16,
          flexWrap: 'wrap',
          gap: 12,
        }}
      >
        <Space align="center" wrap>
          <Button
            type="text"
            icon={<ArrowLeftOutlined />}
            onClick={() => navigate('/applications')}
          />
          <Title level={3} style={{ margin: 0 }}>
            {application.application_name}
          </Title>
          {detail.abbreviation && (
            <Tag color="geekblue" style={{ fontFamily: 'monospace', fontSize: 12 }}>
              {detail.abbreviation}
            </Tag>
          )}
          <Tag
            color={statusColors[status] || 'default'}
            style={{ fontSize: 14, padding: '2px 12px' }}
          >
            {statusLabels[status] || status}
          </Tag>
          {detail.is_cba && (
            <Tag color="red" style={{ fontSize: 14, padding: '2px 12px', fontWeight: 600 }}>
              <WarningOutlined /> CBA
            </Tag>
          )}
          {detail.lifecycle_stage_name && (
            <Tag color="blue">{detail.lifecycle_stage_name}</Tag>
          )}
        </Space>
        <Space wrap>{renderActionButtons()}</Space>
      </div>

      {/* --- CBA Banner --- */}
      {detail.is_cba && (
        <Alert
          type="error"
          showIcon
          icon={<WarningOutlined />}
          message="Critical Business Application"
          description={
            <div>
              <Text strong>Rationale: </Text>
              <Text>{detail.cba_rationale || 'No rationale provided.'}</Text>
            </div>
          }
          style={{ marginBottom: 16 }}
          banner
        />
      )}

      {/* --- Summary statistics --- */}
      <Row gutter={16} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Linked Data Elements"
              value={detail.data_elements_count}
              prefix={<LinkOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Interfaces"
              value={detail.interfaces_count}
              prefix={<ApiOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={8}>
          <Card>
            <Statistic
              title="Linked Processes"
              value={detail.linked_processes?.length || 0}
            />
          </Card>
        </Col>
      </Row>

      {/* --- Ownership Assignment (shown in DRAFT/REVISED status) --- */}
      {showOwnershipSection && (
        <Card
          title={
            <Space>
              <UserOutlined />
              <Text strong>Assign Ownership</Text>
              {ownershipComplete ? (
                <Tag color="success">Complete</Tag>
              ) : (
                <Tag color="warning">Required before submission</Tag>
              )}
            </Space>
          }
          style={{
            marginBottom: 24,
            border: ownershipComplete ? '1px solid #B7EB8F' : '1px solid #FFD591',
            background: ownershipComplete ? '#F6FFED' : '#FFF7E6',
          }}
        >
          {!ownershipComplete && (
            <Alert
              message="All ownership fields must be assigned before this application can be submitted for review."
              type="warning"
              showIcon
              style={{ marginBottom: 16 }}
            />
          )}
          <Row gutter={[16, 16]}>
            <Col xs={24} md={12}>
              <div style={{ marginBottom: 4 }}>
                <Text strong>Business Owner</Text>
                {!businessOwnerId && <Text type="danger"> *</Text>}
              </div>
              <Select
                style={{ width: '100%' }}
                labelInValue
                value={businessOwnerId}
                onChange={(val) => setBusinessOwnerId(val || undefined)}
                options={allUsers.map((u) => ({ value: u.user_id, label: `${u.display_name} (${u.email})` }))}
                placeholder="Select business owner..."
                showSearch
                optionFilterProp="label"
                allowClear
              />
            </Col>
            <Col xs={24} md={12}>
              <div style={{ marginBottom: 4 }}>
                <Text strong>Technical Owner</Text>
                {!technicalOwnerId && <Text type="danger"> *</Text>}
              </div>
              <Select
                style={{ width: '100%' }}
                labelInValue
                value={technicalOwnerId}
                onChange={(val) => setTechnicalOwnerId(val || undefined)}
                options={allUsers.map((u) => ({ value: u.user_id, label: `${u.display_name} (${u.email})` }))}
                placeholder="Select technical owner..."
                showSearch
                optionFilterProp="label"
                allowClear
              />
            </Col>
            <Col xs={24} md={12}>
              <div style={{ marginBottom: 4 }}>
                <Text strong>Data Steward</Text>
                {!stewardUserId && <Text type="danger"> *</Text>}
              </div>
              <Select
                style={{ width: '100%' }}
                labelInValue
                value={stewardUserId}
                onChange={(val) => setStewardUserId(val || undefined)}
                options={allUsers.map((u) => ({ value: u.user_id, label: `${u.display_name} (${u.email})` }))}
                placeholder="Select steward..."
                showSearch
                optionFilterProp="label"
                allowClear
              />
            </Col>
            <Col xs={24} md={12}>
              <div style={{ marginBottom: 4 }}>
                <Text strong>Approver</Text>
                {!approverUserId && <Text type="danger"> *</Text>}
              </div>
              <Select
                style={{ width: '100%' }}
                labelInValue
                value={approverUserId}
                onChange={(val) => setApproverUserId(val || undefined)}
                options={allUsers.map((u) => ({ value: u.user_id, label: `${u.display_name} (${u.email})` }))}
                placeholder="Select approver..."
                showSearch
                optionFilterProp="label"
                allowClear
              />
            </Col>
          </Row>
          <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
            <Col xs={24} md={12}>
              <div style={{ marginBottom: 4 }}>
                <Text strong>Organisational Unit</Text>
              </div>
              <Select
                style={{ width: '100%' }}
                value={orgUnit}
                onChange={(val) => setOrgUnit(val)}
                options={allOrgUnits.map((u) => ({ value: u.unit_name, label: u.unit_name }))}
                placeholder="Select organisational unit..."
                showSearch
                optionFilterProp="label"
                allowClear
              />
            </Col>
          </Row>
          <div style={{ marginTop: 16, textAlign: 'right' }}>
            <Button
              type="primary"
              onClick={handleSaveOwnership}
              loading={ownershipLoading}
              disabled={!businessOwnerId && !technicalOwnerId && !stewardUserId && !approverUserId}
            >
              Save Ownership
            </Button>
          </div>
        </Card>
      )}

      {/* --- 9-Section Collapse --- */}
      <Card style={{ marginBottom: 24 }}>
        <Collapse
          defaultActiveKey={['core', 'classification']}
          ghost
          items={collapseItems}
          size="large"
        />
      </Card>

      {/* --- Linked Data Elements --- */}
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

      {/* --- Interfaces --- */}
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

      {/* --- Workflow Section --- */}
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

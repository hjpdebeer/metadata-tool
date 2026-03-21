import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams, Link } from 'react-router-dom';
import {
  Alert,
  Badge,
  Breadcrumb,
  Button,
  Card,
  Col,
  Descriptions,
  Divider,
  Input,
  Modal,
  Progress,
  Row,
  Select,
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
  DeleteOutlined,
  EditOutlined,
  KeyOutlined,
  LinkOutlined,
  PlusOutlined,
  RobotOutlined,
  SafetyCertificateOutlined,
  SendOutlined,
  UndoOutlined,
  UserOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import { dataDictionaryApi } from '../services/dataDictionaryApi';
import { glossaryApi, workflowApi } from '../services/glossaryApi';
import { usersApi } from '../services/usersApi';
import { dataQualityApi } from '../services/dataQualityApi';
import type { DataElementFullView, TechnicalColumn } from '../services/dataDictionaryApi';
import type { OrganisationalUnit, WorkflowInstanceView } from '../services/glossaryApi';
import type { UserListItem } from '../services/usersApi';
import type { QualityRuleListItem, AiRuleSuggestion } from '../services/dataQualityApi';
import { useAuth } from '../hooks/useAuth';
import AiEnrichmentPanel from '../components/AiEnrichmentPanel';

import { statusColors, statusLabels } from '../constants/statusConfig';

const { Title, Text } = Typography;

const DataElementDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();
  const currentUserId = user?.user_id;

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

  // Ownership assignment state — uses {value, label} to prevent UUID display
  type LabeledValue = { value: string; label: string };
  const [allUsers, setAllUsers] = useState<UserListItem[]>([]);
  const [allOrgUnits, setAllOrgUnits] = useState<OrganisationalUnit[]>([]);
  const [ownershipLoading, setOwnershipLoading] = useState(false);
  const [ownerUserId, setOwnerUserId] = useState<LabeledValue | undefined>();
  const [stewardUserId, setStewardUserId] = useState<LabeledValue | undefined>();
  const [approverUserId, setApproverUserId] = useState<LabeledValue | undefined>();
  const [orgUnit, setOrgUnit] = useState<string | undefined>();

  const fetchElement = useCallback(async (showSpinner = false) => {
    if (!id) return;
    if (showSpinner) setLoading(true);
    try {
      const response = await dataDictionaryApi.getElement(id);
      setElement(response.data);
    } catch {
      message.error('Failed to load element details.');
      navigate('/data-dictionary');
    } finally {
      if (showSpinner) setLoading(false);
    }
  }, [id, navigate]);

  const fetchLookups = useCallback(async () => {
    const [usersRes, orgRes] = await Promise.allSettled([
      usersApi.lookupUsers(),
      glossaryApi.listOrganisationalUnits(),
    ]);
    if (usersRes.status === 'fulfilled') setAllUsers(usersRes.value.data);
    if (orgRes.status === 'fulfilled') setAllOrgUnits(orgRes.value.data);
  }, []);

  const fetchWorkflowInstance = useCallback(async () => {
    if (!id) return;
    try {
      const response = await workflowApi.getInstanceByEntity(id);
      setWorkflowInstance(response.data);
    } catch {
      // No workflow instance for this entity — not an error for the UI
      setWorkflowInstance(null);
    }
  }, [id]);

  // Quality rules for this element
  const [qualityRules, setQualityRules] = useState<QualityRuleListItem[]>([]);
  const [qualityRulesLoading, setQualityRulesLoading] = useState(false);

  const fetchQualityRules = useCallback(async () => {
    if (!id) return;
    setQualityRulesLoading(true);
    try {
      const response = await dataQualityApi.listRules({ element_id: id, page_size: 100 });
      const data = response.data;
      if (Array.isArray(data)) {
        setQualityRules(data);
      } else {
        const paginated = data as unknown as { data: QualityRuleListItem[]; total_count: number };
        setQualityRules(paginated.data);
      }
    } catch {
      // Non-critical — element page still works without quality rules
    } finally {
      setQualityRulesLoading(false);
    }
  }, [id]);

  // AI-suggested quality rules
  const [aiRuleSuggestions, setAiRuleSuggestions] = useState<AiRuleSuggestion[]>([]);
  const [aiRulesLoading, setAiRulesLoading] = useState(false);
  const [aiRulesProvider, setAiRulesProvider] = useState('');
  const [acceptingRuleIndex, setAcceptingRuleIndex] = useState<number | null>(null);

  const handleSuggestQualityRules = async () => {
    if (!id) return;
    setAiRulesLoading(true);
    setAiRuleSuggestions([]);
    try {
      const response = await dataQualityApi.suggestQualityRules(id);
      setAiRuleSuggestions(response.data.suggestions);
      setAiRulesProvider(`${response.data.provider} / ${response.data.model}`);
      if (response.data.suggestions.length === 0) {
        message.info('AI did not suggest any quality rules for this element.');
      }
    } catch {
      message.error('Failed to get AI quality rule suggestions.');
    } finally {
      setAiRulesLoading(false);
    }
  };

  const handleAcceptRuleSuggestion = async (suggestion: AiRuleSuggestion, index: number) => {
    if (!id) return;
    setAcceptingRuleIndex(index);
    try {
      await dataQualityApi.acceptRuleSuggestion({
        element_id: id,
        dimension_code: suggestion.dimension,
        rule_name: suggestion.rule_name,
        description: suggestion.description,
        comparison_type: suggestion.comparison_type,
        comparison_value: suggestion.comparison_value,
        threshold_percentage: suggestion.threshold_percentage,
        severity: suggestion.severity,
      });
      message.success(`Quality rule "${suggestion.rule_name}" created successfully.`);
      // Remove from suggestions list
      setAiRuleSuggestions((prev) => prev.filter((_, i) => i !== index));
      // Refresh the rules list
      fetchQualityRules();
    } catch {
      message.error(`Failed to create quality rule "${suggestion.rule_name}".`);
    } finally {
      setAcceptingRuleIndex(null);
    }
  };

  const handleDismissRuleSuggestion = (index: number) => {
    setAiRuleSuggestions((prev) => prev.filter((_, i) => i !== index));
  };

  const handleAcceptAllRuleSuggestions = async () => {
    if (!id || aiRuleSuggestions.length === 0) return;
    setAiRulesLoading(true);
    let accepted = 0;
    let failed = 0;
    for (const suggestion of aiRuleSuggestions) {
      try {
        await dataQualityApi.acceptRuleSuggestion({
          element_id: id,
          dimension_code: suggestion.dimension,
          rule_name: suggestion.rule_name,
          description: suggestion.description,
          comparison_type: suggestion.comparison_type,
          comparison_value: suggestion.comparison_value,
          threshold_percentage: suggestion.threshold_percentage,
          severity: suggestion.severity,
        });
        accepted++;
      } catch {
        failed++;
      }
    }
    setAiRuleSuggestions([]);
    fetchQualityRules();
    if (failed === 0) {
      message.success(`All ${accepted} quality rules created successfully.`);
    } else {
      message.warning(`${accepted} rules created, ${failed} failed.`);
    }
    setAiRulesLoading(false);
  };

  useEffect(() => {
    fetchElement(true);
    fetchLookups();
    fetchWorkflowInstance();
    fetchQualityRules();
  }, [fetchElement, fetchLookups, fetchWorkflowInstance, fetchQualityRules]);

  // Sync ownership state from detail response — uses resolved names so UUIDs never display
  useEffect(() => {
    if (element) {
      setOwnerUserId(element.owner_user_id && element.owner_name
        ? { value: element.owner_user_id, label: element.owner_name } : undefined);
      setStewardUserId(element.steward_user_id && element.steward_name
        ? { value: element.steward_user_id, label: element.steward_name } : undefined);
      setApproverUserId(element.approver_user_id && element.approver_name
        ? { value: element.approver_user_id, label: element.approver_name } : undefined);
      setOrgUnit(element.organisational_unit || undefined);
    }
  }, [element]);

  // --- Ownership assignment ---

  const handleSaveOwnership = async () => {
    if (!id) return;
    setOwnershipLoading(true);
    try {
      await dataDictionaryApi.updateElement(id, {
        owner_user_id: ownerUserId?.value || undefined,
        steward_user_id: stewardUserId?.value || undefined,
        approver_user_id: approverUserId?.value || undefined,
        organisational_unit: orgUnit || undefined,
      } as Record<string, unknown>);
      message.success('Ownership updated successfully.');
      fetchElement();
    } catch {
      message.error('Failed to update ownership.');
    } finally {
      setOwnershipLoading(false);
    }
  };

  const ownershipComplete = !!(ownerUserId && stewardUserId && approverUserId);
  const showOwnershipSection = element && allUsers.length > 0 && (element.status_code === 'DRAFT' || element.status_code === 'REVISED');

  // --- Amendment ---

  const handleProposeAmendment = async () => {
    if (!id) return;
    try {
      const response = await dataDictionaryApi.amendElement(id);
      message.success('Amendment created. You can now edit the new draft version.');
      navigate(`/data-dictionary/${response.data.element_id}`);
    } catch (err: unknown) {
      const apiMsg = (err as { response?: { data?: { error?: { message?: string } } } })
        ?.response?.data?.error?.message;
      message.error(apiMsg || 'Failed to create amendment.');
    }
  };

  const handleDiscardAmendment = async () => {
    if (!id) return;
    try {
      await dataDictionaryApi.discardAmendment(id);
      message.success(element?.previous_version_id ? 'Amendment discarded.' : 'Draft deleted.');
      if (element?.previous_version_id) {
        navigate(`/data-dictionary/${element.previous_version_id}`);
      } else {
        navigate('/data-dictionary');
      }
    } catch (err: unknown) {
      const apiMsg = (err as { response?: { data?: { error?: { message?: string } } } })
        ?.response?.data?.error?.message;
      message.error(apiMsg || 'Failed to delete draft.');
    }
  };

  // --- Workflow ---

  const handleWorkflowAction = (action: string) => {
    if (!workflowInstance?.instance_id) {
      message.error('No active workflow for this element.');
      return;
    }
    // Pre-flight check: warn user about missing ownership before opening modal
    if (action === 'SUBMIT') {
      const missing: string[] = [];
      if (!element?.owner_user_id) missing.push('Data Owner');
      if (!element?.steward_user_id) missing.push('Data Steward');
      if (!element?.approver_user_id) missing.push('Approver');
      if (missing.length > 0) {
        message.warning(
          `Please assign all ownership fields before submitting: ${missing.join(', ')}. Use the Ownership card below to assign owners.`,
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
      fetchElement();
      fetchWorkflowInstance();
    } catch (error: unknown) {
      const apiMsg = (error as { response?: { data?: { error?: { message?: string } } } })
        ?.response?.data?.error?.message;
      message.error(apiMsg || `Failed to perform action "${transitionAction}".`, 8);
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

  if (!element) {
    return null;
  }

  const status = element.status_code || 'DRAFT';

  // --- Role gating ---
  const isAdmin = user?.roles?.includes('admin') || user?.roles?.includes('ADMIN');
  const isSteward = currentUserId === element?.steward_user_id || isAdmin;
  const isOwner = currentUserId === element?.owner_user_id || isAdmin;

  // --- Action buttons ---

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
      // Discard/Delete button: any draft, only for creator or admin
      if (currentUserId === element?.created_by || isAdmin) {
        buttons.push(
          <Button
            key="discard"
            danger
            icon={<DeleteOutlined />}
            onClick={handleDiscardAmendment}
          >
            {element?.previous_version_id ? 'Discard Amendment' : 'Delete Draft'}
          </Button>,
        );
      }
    }

    // Under Review: only the assigned Data Steward (or admin) can act
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

    // Pending Approval: only the assigned Data Owner (or admin) can act
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
          key="revise"
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

    if (status === 'ACCEPTED') {
      buttons.push(
        <Button
          key="amend"
          type="primary"
          icon={<EditOutlined />}
          onClick={handleProposeAmendment}
        >
          Propose Amendment
        </Button>,
      );
    }

    // Edit button: hidden for terminal/accepted states
    if (!['ACCEPTED', 'DEPRECATED', 'REJECTED', 'SUPERSEDED'].includes(status)) {
      buttons.push(
        <Button
          key="edit"
          icon={<EditOutlined />}
          onClick={() => navigate(`/data-dictionary/${id}/edit`)}
        >
          Edit
        </Button>,
      );
    }

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
            onClick={() => navigate('/data-dictionary')}
          />
          <Title level={3} style={{ margin: 0 }}>
            {element.element_name}
          </Title>
          {element.element_code && (
            <Tag color="geekblue" style={{ fontFamily: 'monospace', fontSize: 12 }}>
              {element.element_code}
            </Tag>
          )}
          <Tag
            color={statusColors[status] || 'default'}
            style={{ fontSize: 14, padding: '2px 12px' }}
          >
            {statusLabels[status] || status}
          </Tag>
          {element.version_number > 1 && (
            <Tag color="geekblue">v{element.version_number}</Tag>
          )}
          {element.is_cde && (
            <Tag color="red" style={{ fontWeight: 600 }}>
              <SafetyCertificateOutlined /> CDE
            </Tag>
          )}
          {element.is_pii && (
            <Tag color="volcano">PII</Tag>
          )}
        </Space>
        <Space wrap>{renderActionButtons()}</Space>
      </div>

      {/* --- CDE Banner --- */}
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
          style={{ marginBottom: 16 }}
          action={
            isSteward ? (
              <Button size="small" danger onClick={handleRemoveCde} loading={cdeLoading}>
                Remove CDE
              </Button>
            ) : undefined
          }
          banner
        />
      )}

      {/* --- Amendment Context Banner --- */}
      {element.previous_version_id && (
        <Alert
          message={`Amendment of v${(element.version_number || 2) - 1}`}
          description={
            <span>
              This is a draft amendment. The original accepted version remains visible until this amendment is approved.{' '}
              <Link to={`/data-dictionary/${element.previous_version_id}`}>View original version</Link>
            </span>
          }
          type="info"
          showIcon
          icon={<EditOutlined />}
          style={{ marginBottom: 16 }}
          banner
        />
      )}

      {/* --- Stats Cards --- */}
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

      {/* --- AI Enrichment Panel --- */}
      <AiEnrichmentPanel
        entityType="data_element"
        entityId={id!}
        onSuggestionApplied={fetchElement}
      />

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
              message="All ownership fields must be assigned before this element can be submitted for review."
              type="warning"
              showIcon
              style={{ marginBottom: 16 }}
            />
          )}
          <Row gutter={[16, 16]}>
            <Col xs={24} md={12}>
              <div style={{ marginBottom: 4 }}>
                <Text strong>Data Owner</Text>
                {!ownerUserId && <Text type="danger"> *</Text>}
              </div>
              <Select
                style={{ width: '100%' }}
                labelInValue
                value={ownerUserId}
                onChange={(val) => setOwnerUserId(val || undefined)}
                options={allUsers.map((u) => ({ value: u.user_id, label: `${u.display_name} (${u.email})` }))}
                placeholder="Select owner..."
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
              disabled={!ownerUserId && !stewardUserId && !approverUserId}
            >
              Save Ownership
            </Button>
          </div>
        </Card>
      )}

      {/* --- Element Details --- */}
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
          <Descriptions.Item label="Data Type">
            {element.data_type}
            {element.data_type && ['DECIMAL', 'NUMERIC'].includes(element.data_type.toUpperCase()) && element.numeric_precision != null && (
              `(${element.numeric_precision}${element.numeric_scale != null ? `,${element.numeric_scale}` : ''})`
            )}
            {element.data_type && ['VARCHAR', 'CHAR', 'TEXT'].includes(element.data_type.toUpperCase()) && element.max_length != null && (
              `(${element.max_length})`
            )}
          </Descriptions.Item>
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
          <Descriptions.Item label="PII">
            {element.is_pii ? <Tag color="volcano">Yes</Tag> : 'No'}
          </Descriptions.Item>
          <Descriptions.Item label="Glossary Term">
            {element.glossary_term_name && element.glossary_term_id ? (
              <Link to={`/glossary/${element.glossary_term_id}`}>
                <Tag color="blue" style={{ cursor: 'pointer' }}>
                  <LinkOutlined /> {element.glossary_term_name}
                </Tag>
              </Link>
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
          <Descriptions.Item label="Status">
            <Tag color={statusColors[status] || 'default'}>
              {statusLabels[status] || status}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Version">
            {element.version_number}
            {element.is_current_version ? (
              <Tag color="green" style={{ marginLeft: 8 }}>Current</Tag>
            ) : (
              <Tag color="default" style={{ marginLeft: 8 }}>Superseded</Tag>
            )}
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
        </Descriptions>
      </Card>

      {/* --- Ownership Details --- */}
      <Card title="Ownership & Lifecycle" style={{ marginBottom: 24 }}>
        <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small">
          <Descriptions.Item label="Data Owner">
            {element.owner_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Data Steward">
            {element.steward_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Approver">
            {element.approver_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Organisational Unit">
            {element.organisational_unit || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Review Frequency">
            {element.review_frequency_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Next Review Date">
            {formatDateShort(element.next_review_date) || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Approved Date">
            {element.approved_at ? formatDate(element.approved_at) : '-'}
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

      {/* --- Technical Metadata --- */}
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

      {/* --- Quality Rules --- */}
      <Card
        title={
          <Space>
            <SafetyCertificateOutlined />
            <span>Quality Rules</span>
            <Badge count={qualityRules.length} showZero style={{ backgroundColor: qualityRules.length > 0 ? '#1B3A5C' : '#D9D9D9' }} />
          </Space>
        }
        extra={
          <Space>
            <Button
              size="small"
              icon={<RobotOutlined />}
              onClick={handleSuggestQualityRules}
              loading={aiRulesLoading}
            >
              Suggest Quality Rules
            </Button>
            <Button
              type="primary"
              size="small"
              icon={<PlusOutlined />}
              onClick={() => navigate(`/data-quality/rules/new?element_id=${id}`)}
            >
              Create Quality Rule
            </Button>
          </Space>
        }
        style={{ marginBottom: 24 }}
      >
        {/* AI Suggestions */}
        {aiRuleSuggestions.length > 0 && (
          <div style={{ marginBottom: 16 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
              <Space>
                <RobotOutlined style={{ color: '#722ED1' }} />
                <Text strong>AI-Suggested Quality Rules</Text>
                <Text type="secondary">({aiRulesProvider})</Text>
              </Space>
              <Space>
                <Button
                  type="primary"
                  size="small"
                  icon={<CheckOutlined />}
                  onClick={handleAcceptAllRuleSuggestions}
                  loading={aiRulesLoading}
                >
                  Accept All ({aiRuleSuggestions.length})
                </Button>
                <Button
                  size="small"
                  onClick={() => setAiRuleSuggestions([])}
                >
                  Dismiss All
                </Button>
              </Space>
            </div>
            {aiRuleSuggestions.map((suggestion, index) => {
              const dimensionColors: Record<string, string> = {
                COMPLETENESS: '#1890FF',
                UNIQUENESS: '#722ED1',
                VALIDITY: '#13C2C2',
                ACCURACY: '#52C41A',
                TIMELINESS: '#FA8C16',
                CONSISTENCY: '#EB2F96',
              };
              const severityColors: Record<string, string> = {
                LOW: '#52C41A',
                MEDIUM: '#1890FF',
                HIGH: '#FA8C16',
                CRITICAL: '#FF4D4F',
              };
              return (
                <Card
                  key={`ai-rule-${index}`}
                  size="small"
                  style={{
                    marginBottom: 8,
                    borderLeft: `3px solid ${dimensionColors[suggestion.dimension] || '#D9D9D9'}`,
                    backgroundColor: '#FAFAFA',
                  }}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                    <div style={{ flex: 1 }}>
                      <Space wrap style={{ marginBottom: 4 }}>
                        <Tag color={dimensionColors[suggestion.dimension] || 'default'}>
                          {suggestion.dimension}
                        </Tag>
                        <Text strong>{suggestion.rule_name}</Text>
                        <Tag color={severityColors[suggestion.severity] || 'default'} style={{ fontWeight: 600 }}>
                          {suggestion.severity}
                        </Tag>
                      </Space>
                      <div style={{ marginBottom: 4 }}>
                        <Text>{suggestion.description}</Text>
                      </div>
                      <Space wrap size="middle" style={{ marginBottom: 4 }}>
                        {suggestion.comparison_type && (
                          <Text type="secondary">
                            Type: <Text code>{suggestion.comparison_type}</Text>
                          </Text>
                        )}
                        {suggestion.comparison_value && (
                          <Text type="secondary">
                            Value: <Text code>{suggestion.comparison_value}</Text>
                          </Text>
                        )}
                        <Text type="secondary">
                          Threshold: {suggestion.threshold_percentage}%
                        </Text>
                      </Space>
                      <div style={{ marginBottom: 4 }}>
                        <Tooltip title="AI Confidence">
                          <Progress
                            percent={Math.round(suggestion.confidence * 100)}
                            size="small"
                            style={{ maxWidth: 150, display: 'inline-flex' }}
                            strokeColor={
                              suggestion.confidence >= 0.8
                                ? '#52C41A'
                                : suggestion.confidence >= 0.6
                                  ? '#1677FF'
                                  : '#FAAD14'
                            }
                          />
                        </Tooltip>
                      </div>
                      <Text type="secondary" italic style={{ fontSize: 12 }}>
                        {suggestion.rationale}
                      </Text>
                    </div>
                    <Space style={{ marginLeft: 16, flexShrink: 0 }}>
                      <Button
                        type="primary"
                        size="small"
                        icon={<CheckOutlined />}
                        style={{ backgroundColor: '#52C41A', borderColor: '#52C41A' }}
                        onClick={() => handleAcceptRuleSuggestion(suggestion, index)}
                        loading={acceptingRuleIndex === index}
                      >
                        Accept
                      </Button>
                      <Button
                        size="small"
                        icon={<CloseOutlined />}
                        onClick={() => handleDismissRuleSuggestion(index)}
                      >
                        Dismiss
                      </Button>
                    </Space>
                  </div>
                </Card>
              );
            })}
            <Divider style={{ margin: '12px 0' }} />
          </div>
        )}
        <Table
          columns={[
            {
              title: 'Rule Name',
              dataIndex: 'rule_name',
              key: 'rule_name',
              render: (name: string, record: QualityRuleListItem) => (
                <a onClick={() => navigate(`/data-quality/rules/${record.rule_id}`)}>{name}</a>
              ),
            },
            {
              title: 'Dimension',
              dataIndex: 'dimension_name',
              key: 'dimension_name',
              width: 140,
              render: (name: string) => <Tag color="blue">{name}</Tag>,
            },
            {
              title: 'Rule Type',
              dataIndex: 'rule_type_name',
              key: 'rule_type_name',
              width: 160,
            },
            {
              title: 'Threshold',
              dataIndex: 'threshold_percentage',
              key: 'threshold_percentage',
              width: 100,
              align: 'center' as const,
              render: (val: number) => val != null ? `${val}%` : '-',
            },
            {
              title: 'Severity',
              dataIndex: 'severity',
              key: 'severity',
              width: 110,
              render: (severity: string) => {
                const colors: Record<string, string> = {
                  LOW: '#52C41A',
                  MEDIUM: '#1890FF',
                  HIGH: '#FA8C16',
                  CRITICAL: '#FF4D4F',
                };
                return (
                  <Tag color={colors[severity] || 'default'} style={{ fontWeight: 600 }}>
                    {severity}
                  </Tag>
                );
              },
            },
            {
              title: 'Status',
              dataIndex: 'status_code',
              key: 'status_code',
              width: 120,
              render: (statusCode: string) => (
                <Tag color={statusColors[statusCode] || 'default'}>
                  {statusLabels[statusCode] || statusCode}
                </Tag>
              ),
            },
          ]}
          dataSource={qualityRules}
          rowKey="rule_id"
          loading={qualityRulesLoading}
          pagination={false}
          size="small"
          locale={{ emptyText: 'No quality rules defined for this element.' }}
        />
      </Card>

      {/* --- Workflow Section --- */}
      {workflowInstance && (
        <Card title="Workflow" style={{ marginBottom: 24 }}>
          <Descriptions column={{ xs: 1, sm: 2 }} size="small" style={{ marginBottom: 16 }}>
            <Descriptions.Item label="Current State">
              <Tag
                color={
                  statusColors[workflowInstance.current_state_name?.toUpperCase()] || 'processing'
                }
              >
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

      {/* --- Workflow Transition Modal --- */}
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

      {/* --- CDE Designation Modal --- */}
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

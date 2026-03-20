import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams, Link } from 'react-router-dom';
import {
  Alert,
  Breadcrumb,
  Button,
  Card,
  Col,
  Collapse,
  Descriptions,
  Divider,
  Input,
  Modal,
  Row,
  Select,
  Space,
  Spin,
  Tag,
  Timeline,
  Tooltip,
  Typography,
  message,
} from 'antd';
import {
  ArrowLeftOutlined,
  CheckOutlined,
  CloseOutlined,
  EditOutlined,
  ExperimentOutlined,
  LinkOutlined,
  PlusOutlined,
  SafetyCertificateOutlined,
  DeleteOutlined,
  SendOutlined,
  UndoOutlined,
  UserOutlined,
} from '@ant-design/icons';
import { glossaryApi, workflowApi } from '../services/glossaryApi';
import type {
  GlossaryTermDetailView,
  GlossaryRegulatoryTag,
  GlossarySubjectArea,
  WorkflowInstanceView,
} from '../services/glossaryApi';
import type { OrganisationalUnit } from '../services/glossaryApi';
import { usersApi } from '../services/usersApi';
import type { UserListItem } from '../services/usersApi';
import AiEnrichmentPanel from '../components/AiEnrichmentPanel';
import { useAuth } from '../hooks/useAuth';

const { Title, Text, Paragraph } = Typography;

const statusColors: Record<string, string> = {
  DRAFT: 'default',
  PROPOSED: 'processing',
  UNDER_REVIEW: 'warning',
  PENDING_APPROVAL: 'processing',
  REVISED: 'orange',
  ACCEPTED: 'success',
  REJECTED: 'error',
  DEPRECATED: 'default',
  SUPERSEDED: 'default',
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
  SUPERSEDED: 'Superseded',
};

/** Small sparkle icon for AI-suggestible fields */
const AiHint: React.FC = () => (
  <Tooltip title="This field can be populated by AI enrichment">
    <ExperimentOutlined style={{ color: '#8B5CF6', fontSize: 12, marginLeft: 4 }} />
  </Tooltip>
);

/** Placeholder for empty values */
const EmptyValue: React.FC<{ text?: string }> = ({ text }) => (
  <Text type="secondary" italic style={{ fontSize: 13 }}>
    {text || 'Not yet populated'}
  </Text>
);

const GlossaryTermDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();
  const currentUserId = user?.user_id;
  const [detail, setDetail] = useState<GlossaryTermDetailView | null>(null);
  const [workflowInstance, setWorkflowInstance] = useState<WorkflowInstanceView | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [transitionModalOpen, setTransitionModalOpen] = useState(false);
  const [transitionAction, setTransitionAction] = useState('');
  const [transitionComments, setTransitionComments] = useState('');

  // Ownership assignment state — uses {value, label} to prevent UUID display
  type LabeledValue = { value: string; label: string };
  const [allUsers, setAllUsers] = useState<UserListItem[]>([]);
  const [allOrgUnits, setAllOrgUnits] = useState<OrganisationalUnit[]>([]);
  const [ownershipLoading, setOwnershipLoading] = useState(false);
  const [ownerUserId, setOwnerUserId] = useState<LabeledValue | undefined>();
  const [stewardUserId, setStewardUserId] = useState<LabeledValue | undefined>();
  const [domainOwnerUserId, setDomainOwnerUserId] = useState<LabeledValue | undefined>();
  const [approverUserId, setApproverUserId] = useState<LabeledValue | undefined>();
  const [orgUnit, setOrgUnit] = useState<string | undefined>();

  // Tag management state
  const [allRegulatoryTags, setAllRegulatoryTags] = useState<GlossaryRegulatoryTag[]>([]);
  const [allSubjectAreas, setAllSubjectAreas] = useState<GlossarySubjectArea[]>([]);
  const [addingRegTag, setAddingRegTag] = useState(false);
  const [addingSubjectArea, setAddingSubjectArea] = useState(false);
  const [newTagInput, setNewTagInput] = useState('');
  const [addingFreeTag, setAddingFreeTag] = useState(false);

  const fetchDetail = useCallback(async (showSpinner = false) => {
    if (!id) return;
    if (showSpinner) setLoading(true);
    try {
      // ADR-0006: Single detail endpoint returns flat response with all resolved names + junction data
      const response = await glossaryApi.getTermDetail(id);
      setDetail(response.data);
    } catch {
      message.error('Failed to load term details.');
      navigate('/glossary');
    } finally {
      if (showSpinner) setLoading(false);
    }
  }, [id, navigate]);

  const fetchLookups = useCallback(async () => {
    const [regRes, areaRes, usersRes, orgRes] = await Promise.allSettled([
      glossaryApi.listRegulatoryTags(),
      glossaryApi.listSubjectAreas(),
      usersApi.lookupUsers(),
      glossaryApi.listOrganisationalUnits(),
    ]);
    if (regRes.status === 'fulfilled') setAllRegulatoryTags(regRes.value.data);
    if (areaRes.status === 'fulfilled') setAllSubjectAreas(areaRes.value.data);
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

  useEffect(() => {
    fetchDetail(true); // Initial load — show spinner
    fetchLookups();
    fetchWorkflowInstance();
  }, [fetchDetail, fetchLookups, fetchWorkflowInstance]);

  // Sync ownership state from detail response — uses resolved names so UUIDs never display
  useEffect(() => {
    if (detail) {
      setOwnerUserId(detail.owner_user_id && detail.owner_name
        ? { value: detail.owner_user_id, label: detail.owner_name } : undefined);
      setStewardUserId(detail.steward_user_id && detail.steward_name
        ? { value: detail.steward_user_id, label: detail.steward_name } : undefined);
      setDomainOwnerUserId(detail.domain_owner_user_id && detail.domain_owner_name
        ? { value: detail.domain_owner_user_id, label: detail.domain_owner_name } : undefined);
      setApproverUserId(detail.approver_user_id && detail.approver_name
        ? { value: detail.approver_user_id, label: detail.approver_name } : undefined);
      setOrgUnit(detail.organisational_unit || undefined);
    }
  }, [detail]);

  // --- Ownership assignment ---

  const handleSaveOwnership = async () => {
    if (!id) return;
    setOwnershipLoading(true);
    try {
      await glossaryApi.updateTerm(id, {
        owner_user_id: ownerUserId?.value || undefined,
        steward_user_id: stewardUserId?.value || undefined,
        domain_owner_user_id: domainOwnerUserId?.value || undefined,
        approver_user_id: approverUserId?.value || undefined,
        organisational_unit: orgUnit || undefined,
      } as Record<string, unknown>);
      message.success('Ownership updated successfully.');
      fetchDetail();
    } catch {
      message.error('Failed to update ownership.');
    } finally {
      setOwnershipLoading(false);
    }
  };

  const ownershipComplete = !!(ownerUserId && stewardUserId && domainOwnerUserId && approverUserId);
  const showOwnershipSection = detail && allUsers.length > 0 && (detail.status_code === 'DRAFT' || detail.status_code === 'REVISED');

  // --- Junction management ---

  const handleAttachRegTag = async (tagId: string) => {
    if (!id) return;
    try {
      await glossaryApi.attachRegulatoryTag(id, tagId);
      message.success('Regulatory tag added.');
      setAddingRegTag(false);
      fetchDetail();
    } catch {
      message.error('Failed to add regulatory tag.');
    }
  };

  const handleDetachRegTag = async (tagId: string) => {
    if (!id) return;
    try {
      await glossaryApi.detachRegulatoryTag(id, tagId);
      message.success('Regulatory tag removed.');
      fetchDetail();
    } catch {
      message.error('Failed to remove regulatory tag.');
    }
  };

  const handleAttachSubjectArea = async (areaId: string) => {
    if (!id) return;
    try {
      await glossaryApi.attachSubjectArea(id, areaId);
      message.success('Subject area added.');
      setAddingSubjectArea(false);
      fetchDetail();
    } catch {
      message.error('Failed to add subject area.');
    }
  };

  const handleDetachSubjectArea = async (areaId: string) => {
    if (!id) return;
    try {
      await glossaryApi.detachSubjectArea(id, areaId);
      message.success('Subject area removed.');
      fetchDetail();
    } catch {
      message.error('Failed to remove subject area.');
    }
  };

  const handleAddFreeTag = async () => {
    if (!id || !newTagInput.trim()) return;
    setAddingFreeTag(true);
    try {
      await glossaryApi.attachTag(id, newTagInput.trim());
      message.success('Tag added.');
      setNewTagInput('');
      fetchDetail();
    } catch {
      message.error('Failed to add tag.');
    } finally {
      setAddingFreeTag(false);
    }
  };

  const handleDetachFreeTag = async (tagId: string) => {
    if (!id) return;
    try {
      await glossaryApi.detachTag(id, tagId);
      message.success('Tag removed.');
      fetchDetail();
    } catch {
      message.error('Failed to remove tag.');
    }
  };

  // --- Amendment ---

  const handleProposeAmendment = async () => {
    if (!id) return;
    try {
      const response = await glossaryApi.amendTerm(id);
      message.success('Amendment created. You can now edit the new draft version.');
      navigate(`/glossary/${response.data.term_id}`);
    } catch (err: unknown) {
      const apiMsg = (err as { response?: { data?: { error?: { message?: string } } } })
        ?.response?.data?.error?.message;
      message.error(apiMsg || 'Failed to create amendment.');
    }
  };

  const handleDiscardAmendment = async () => {
    if (!id) return;
    try {
      await glossaryApi.discardAmendment(id);
      message.success('Amendment discarded.');
      // Navigate to the original term
      if (detail?.previous_version_id) {
        navigate(`/glossary/${detail.previous_version_id}`);
      } else {
        navigate('/glossary');
      }
    } catch (err: unknown) {
      const apiMsg = (err as { response?: { data?: { error?: { message?: string } } } })
        ?.response?.data?.error?.message;
      message.error(apiMsg || 'Failed to discard amendment.');
    }
  };

  // --- Workflow ---

  const handleWorkflowAction = (action: string) => {
    if (!workflowInstance?.instance_id) {
      message.error('No active workflow for this term.');
      return;
    }
    // Pre-flight check: warn user about missing ownership before opening modal
    if (action === 'SUBMIT') {
      const missing: string[] = [];
      if (!detail.owner_user_id) missing.push('Business Term Owner');
      if (!detail.steward_user_id) missing.push('Data Steward');
      if (!detail.domain_owner_user_id) missing.push('Data Domain Owner');
      if (!detail.approver_user_id) missing.push('Approver');
      if (missing.length > 0) {
        message.warning(
          `Please assign all ownership fields before submitting: ${missing.join(', ')}. Use the Edit button to assign owners.`,
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
      fetchDetail();
      fetchWorkflowInstance();
    } catch (error: unknown) {
      // Show specific validation messages from the backend (e.g., ownership missing)
      const apiMsg = (error as { response?: { data?: { error?: { message?: string } } } })
        ?.response?.data?.error?.message;
      message.error(apiMsg || `Failed to perform action "${transitionAction}".`, 8);
    } finally {
      setActionLoading(false);
    }
  };

  // --- Helpers ---

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

  if (!detail) {
    return null;
  }

  // ADR-0006: All fields are at the root level — flat struct, no nesting.
  const term = detail;
  const status = detail.status_code || 'DRAFT';

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
      // Discard button: only for amendments (has previous_version_id), only for creator or admin
      if (detail?.previous_version_id && (currentUserId === detail?.created_by || user?.roles?.includes('ADMIN') || user?.roles?.includes('admin'))) {
        buttons.push(
          <Button
            key="discard"
            danger
            icon={<DeleteOutlined />}
            onClick={handleDiscardAmendment}
          >
            Discard Amendment
          </Button>,
        );
      }
    }

    // Under Review: only the assigned Data Steward (or admin) can act
    const isAdmin = user?.roles?.includes('admin') || user?.roles?.includes('ADMIN');
    const isSteward = currentUserId === detail?.steward_user_id || isAdmin;
    const isOwner = currentUserId === detail?.owner_user_id || isAdmin;

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

    // Pending Approval: only the assigned Business Term Owner (or admin) can act
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

    // Edit button: available in draft/revised/under review/pending approval states
    if (!['ACCEPTED', 'DEPRECATED', 'REJECTED'].includes(status)) {
      buttons.push(
        <Button
          key="edit"
          icon={<EditOutlined />}
          onClick={() => navigate(`/glossary/${id}/edit`)}
        >
          Edit
        </Button>,
      );
    }

    return buttons;
  };

  // Child terms from backend (terms where parent_term_id = this term)
  const childTerms = detail.child_terms || [];

  // Related terms — placeholder until glossary_term_relationships junction is wired
  type RelatedTermRef = { term_id: string; term_name: string; relationship_type: string; relationship_type_name?: string };
  const allRelatedTerms: RelatedTermRef[] = [];
  const synonyms = allRelatedTerms.filter((r) => r.relationship_type === 'SYNONYM');
  const relatedTerms = allRelatedTerms.filter((r) => r.relationship_type === 'RELATED');
  const conflicting = allRelatedTerms.filter((r) => r.relationship_type === 'CONFLICTING');
  const isPartOf = allRelatedTerms.filter((r) => r.relationship_type === 'IS_PART_OF');
  const otherRelations = allRelatedTerms.filter(
    (r) =>
      !['CHILD', 'HAS_PART', 'SYNONYM', 'RELATED', 'CONFLICTING', 'IS_PART_OF'].includes(
        r.relationship_type,
      ),
  );

  // Available tags/areas not yet attached
  const attachedRegTagIds = new Set(detail.regulatory_tags.map((t) => t.tag_id));
  const availableRegTags = allRegulatoryTags.filter((t) => !attachedRegTagIds.has(t.tag_id));
  const attachedAreaIds = new Set(detail.subject_areas.map((a) => a.subject_area_id));
  const availableSubjectAreas = allSubjectAreas.filter((a) => !attachedAreaIds.has(a.subject_area_id));

  const renderTermLinks = (terms: { term_id: string; term_name: string }[]) => {
    if (terms.length === 0) return <EmptyValue text="None" />;
    return (
      <Space wrap size={[4, 4]}>
        {terms.map((t) => (
          <Link key={t.term_id} to={`/glossary/${t.term_id}`}>
            <Tag color="blue" style={{ cursor: 'pointer' }}>
              <LinkOutlined /> {t.term_name}
            </Tag>
          </Link>
        ))}
      </Space>
    );
  };

  // --- Collapse panels ---

  const collapseItems = [
    {
      key: 'core',
      label: <Text strong>Core Identity</Text>,
      children: (
        <Descriptions column={{ xs: 1, sm: 2, md: 3 }} bordered size="small">
          <Descriptions.Item label="Term Name">{term.term_name}</Descriptions.Item>
          <Descriptions.Item label="Term Code">
            {term.term_code ? (
              <Tag color="geekblue" style={{ fontFamily: 'monospace' }}>
                {term.term_code}
              </Tag>
            ) : (
              <EmptyValue text="Pending generation" />
            )}
          </Descriptions.Item>
          <Descriptions.Item label="Abbreviation">
            {term.abbreviation || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Version">{term.version_number}</Descriptions.Item>
          <Descriptions.Item label="Status">
            <Tag color={statusColors[status] || 'default'}>
              {statusLabels[status] || status}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Current Version">
            {term.is_current_version ? (
              <Tag color="green">Current</Tag>
            ) : (
              <Tag color="default">Superseded</Tag>
            )}
          </Descriptions.Item>
        </Descriptions>
      ),
    },
    {
      key: 'definition',
      label: (
        <Text strong>
          Definition & Semantics <AiHint />
        </Text>
      ),
      children: (
        <div>
          <div style={{ marginBottom: 16 }}>
            <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 4 }}>
              Definition
            </Text>
            <Paragraph style={{ fontSize: 15, marginBottom: 0 }}>
              {term.definition}
            </Paragraph>
          </div>

          <Descriptions column={{ xs: 1, sm: 1, md: 2 }} bordered size="small">
            <Descriptions.Item label={<>Definition Notes <AiHint /></>} span={2}>
              {term.definition_notes || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label={<>Counter-Examples <AiHint /></>} span={2}>
              {term.counter_examples || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label={<>Formula <AiHint /></>} span={2}>
              {term.formula ? (
                <div style={{ whiteSpace: 'pre-wrap' }}>{term.formula}</div>
              ) : (
                <EmptyValue />
              )}
            </Descriptions.Item>
            <Descriptions.Item label={<>Examples <AiHint /></>} span={2}>
              {term.examples || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label="Unit of Measure">
              {detail.unit_of_measure_name ? (
                <span>{detail.unit_of_measure_name}</span>
              ) : (
                <EmptyValue />
              )}
            </Descriptions.Item>
          </Descriptions>
        </div>
      ),
    },
    {
      key: 'classification',
      label: (
        <Text strong>
          Classification <AiHint />
        </Text>
      ),
      children: (
        <div>
          <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small" style={{ marginBottom: 16 }}>
            <Descriptions.Item label={<>Domain <AiHint /></>}>
              {term.domain_name || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label={<>Category <AiHint /></>}>
              {term.category_name || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label={<>Term Type <AiHint /></>}>
              {detail.term_type_name ? (
                <Tag color="purple">{detail.term_type_name}</Tag>
              ) : (
                <EmptyValue />
              )}
            </Descriptions.Item>
            <Descriptions.Item label="Data Classification">
              {detail.classification_name ? (
                <Tag color="volcano">{detail.classification_name}</Tag>
              ) : (
                <EmptyValue />
              )}
            </Descriptions.Item>
          </Descriptions>

          {/* Regulatory Tags */}
          <div style={{ marginBottom: 16 }}>
            <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>
              Regulatory Tags
            </Text>
            <Space wrap size={[4, 8]}>
              {detail.regulatory_tags.map((tag) => (
                <Tag
                  key={tag.tag_id}
                  color="red"
                  closable
                  onClose={(e) => {
                    e.preventDefault();
                    handleDetachRegTag(tag.tag_id);
                  }}
                >
                  {tag.tag_name}
                </Tag>
              ))}
              {detail.regulatory_tags.length === 0 && !addingRegTag && (
                <EmptyValue text="No regulatory tags" />
              )}
              {addingRegTag ? (
                <Select
                  placeholder="Select tag..."
                  style={{ minWidth: 180 }}
                  size="small"
                  showSearch
                  optionFilterProp="label"
                  options={availableRegTags.map((t) => ({ value: t.tag_id, label: t.tag_name }))}
                  onChange={handleAttachRegTag}
                  onBlur={() => setAddingRegTag(false)}
                  autoFocus
                />
              ) : (
                <Tag
                  style={{ borderStyle: 'dashed', cursor: 'pointer' }}
                  onClick={() => setAddingRegTag(true)}
                >
                  <PlusOutlined /> Add
                </Tag>
              )}
            </Space>
          </div>

          {/* Subject Areas */}
          <div>
            <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>
              Subject Areas
            </Text>
            <Space wrap size={[4, 8]}>
              {detail.subject_areas.map((area) => (
                <Tag
                  key={area.subject_area_id}
                  color="cyan"
                  closable
                  onClose={(e) => {
                    e.preventDefault();
                    handleDetachSubjectArea(area.subject_area_id);
                  }}
                >
                  {area.area_name}
                </Tag>
              ))}
              {detail.subject_areas.length === 0 && !addingSubjectArea && (
                <EmptyValue text="No subject areas" />
              )}
              {addingSubjectArea ? (
                <Select
                  placeholder="Select area..."
                  style={{ minWidth: 200 }}
                  size="small"
                  showSearch
                  optionFilterProp="label"
                  options={availableSubjectAreas.map((a) => ({
                    value: a.subject_area_id,
                    label: a.area_name,
                  }))}
                  onChange={handleAttachSubjectArea}
                  onBlur={() => setAddingSubjectArea(false)}
                  autoFocus
                />
              ) : (
                <Tag
                  style={{ borderStyle: 'dashed', cursor: 'pointer' }}
                  onClick={() => setAddingSubjectArea(true)}
                >
                  <PlusOutlined /> Add
                </Tag>
              )}
            </Space>
          </div>
        </div>
      ),
    },
    {
      key: 'ownership',
      label: <Text strong>Ownership</Text>,
      children: (
        <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small">
          <Descriptions.Item label="Business Term Owner">
            {term.owner_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Data Steward">
            {term.steward_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Data Domain Owner">
            {detail.domain_owner_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Approver">
            {detail.approver_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Organisational Unit" span={2}>
            {term.organisational_unit || <EmptyValue />}
          </Descriptions.Item>
        </Descriptions>
      ),
    },
    {
      key: 'lifecycle',
      label: <Text strong>Lifecycle</Text>,
      children: (
        <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small">
          <Descriptions.Item label="Created">
            {formatDate(term.created_at)}
          </Descriptions.Item>
          <Descriptions.Item label="Last Modified">
            {formatDate(term.updated_at)}
          </Descriptions.Item>
          <Descriptions.Item label="Approved Date">
            {formatDate(term.approved_at) || <EmptyValue text="Not yet approved" />}
          </Descriptions.Item>
          <Descriptions.Item label="Review Frequency">
            {detail.review_frequency_name || <EmptyValue />}
          </Descriptions.Item>
          <Descriptions.Item label="Next Review Date">
            {formatDateShort(term.next_review_date) || <EmptyValue text="Not scheduled" />}
          </Descriptions.Item>
        </Descriptions>
      ),
    },
    {
      key: 'relationships',
      label: <Text strong>Relationships</Text>,
      children: (
        <div>
          <Descriptions column={1} bordered size="small" style={{ marginBottom: 0 }}>
            <Descriptions.Item label="Parent Term">
              {term.parent_term_id && detail.parent_term_name ? (
                <Link to={`/glossary/${term.parent_term_id}`}>
                  <Tag color="blue" style={{ cursor: 'pointer' }}>
                    <LinkOutlined /> {detail.parent_term_name}
                  </Tag>
                </Link>
              ) : (
                <EmptyValue text="No parent term" />
              )}
            </Descriptions.Item>
            <Descriptions.Item label="Child Terms">
              {renderTermLinks(childTerms)}
            </Descriptions.Item>
            <Descriptions.Item label="Synonyms / Aliases">
              {(detail.aliases && detail.aliases.length > 0) ? (
                <Space wrap size={[4, 8]}>
                  {detail.aliases.map((a) => (
                    <Tag key={a.alias_id} color="cyan">{a.alias_name}</Tag>
                  ))}
                </Space>
              ) : synonyms.length > 0 ? (
                renderTermLinks(synonyms)
              ) : (
                <EmptyValue />
              )}
            </Descriptions.Item>
            <Descriptions.Item label="Related Terms">
              {renderTermLinks(relatedTerms)}
            </Descriptions.Item>
            <Descriptions.Item label="Conflicting Terms">
              {renderTermLinks(conflicting)}
            </Descriptions.Item>
            <Descriptions.Item label="Is Part Of">
              {renderTermLinks(isPartOf)}
            </Descriptions.Item>
            {otherRelations.length > 0 && (
              <Descriptions.Item label="Other Relationships">
                <Space wrap size={[4, 4]}>
                  {otherRelations.map((r) => (
                    <Link key={`${r.term_id}-${r.relationship_type}`} to={`/glossary/${r.term_id}`}>
                      <Tag color="blue" style={{ cursor: 'pointer' }}>
                        {r.relationship_type_name}: {r.term_name}
                      </Tag>
                    </Link>
                  ))}
                </Space>
              </Descriptions.Item>
            )}
          </Descriptions>
        </div>
      ),
    },
    {
      key: 'usage',
      label: (
        <Text strong>
          Usage & Context <AiHint />
        </Text>
      ),
      children: (
        <div>
          <Descriptions column={1} bordered size="small" style={{ marginBottom: 16 }}>
            <Descriptions.Item label={<>Business Rules <AiHint /></>}>
              {term.business_context || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label="Used in Reports">
              {term.used_in_reports || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label="Used in Policies">
              {term.used_in_policies || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label={<>Regulatory Reporting Usage <AiHint /></>}>
              {term.regulatory_reporting_usage || <EmptyValue />}
            </Descriptions.Item>
          </Descriptions>

          {/* Linked Processes */}
          <div style={{ marginTop: 12 }}>
            <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>
              Used in Processes
            </Text>
            {detail.linked_processes.length > 0 ? (
              <Space wrap size={[4, 4]}>
                {detail.linked_processes.map((p) => (
                  <Link key={p.process_id} to={`/processes/${p.process_id}`}>
                    <Tag color="green" style={{ cursor: 'pointer' }}>
                      <LinkOutlined /> {p.process_name}
                    </Tag>
                  </Link>
                ))}
              </Space>
            ) : (
              <EmptyValue text="No linked processes" />
            )}
          </div>
        </div>
      ),
    },
    {
      key: 'quality',
      label: <Text strong>Quality</Text>,
      children: (
        <div>
          {term.is_cbt && (
            <Alert
              message="Critical Business Term"
              description="This term has been designated as a Critical Business Term (CBT). CBTs require heightened governance, quality monitoring, and stewardship oversight. Linked data elements automatically inherit CDE status."
              type="error"
              showIcon
              icon={<SafetyCertificateOutlined />}
              style={{ marginBottom: 16 }}
            />
          )}
          <Descriptions column={{ xs: 1, sm: 2 }} bordered size="small">
            <Descriptions.Item label="CBT Designation">
              {term.is_cbt ? (
                <Tag color="red">
                  <SafetyCertificateOutlined /> Critical Business Term
                </Tag>
              ) : (
                <Tag color="default">Not CBT</Tag>
              )}
            </Descriptions.Item>
            <Descriptions.Item label="Golden Source">
              {detail.golden_source_app_id && detail.golden_source_app_name ? (
                <Link to={`/applications/${detail.golden_source_app_id}`}>
                  <Tag color="blue" style={{ cursor: 'pointer' }}>
                    <LinkOutlined /> {detail.golden_source_app_name}
                  </Tag>
                </Link>
              ) : (
                <EmptyValue />
              )}
            </Descriptions.Item>
            <Descriptions.Item label="Confidence Level">
              {detail.confidence_level_name ? (
                <Tag
                  color={
                    detail.confidence_level_name === 'High'
                      ? 'green'
                      : detail.confidence_level_name === 'Medium'
                        ? 'gold'
                        : 'orange'
                  }
                >
                  {detail.confidence_level_name}
                </Tag>
              ) : (
                <EmptyValue />
              )}
            </Descriptions.Item>
          </Descriptions>
        </div>
      ),
    },
    {
      key: 'discoverability',
      label: <Text strong>Discoverability</Text>,
      children: (
        <div>
          {/* Tags / Keywords */}
          <div style={{ marginBottom: 16 }}>
            <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>
              Tags / Keywords
            </Text>
            <Space wrap size={[4, 8]}>
              {detail.tags.map((tag) => (
                <Tag
                  key={tag.tag_id}
                  color="blue"
                  closable
                  onClose={(e) => {
                    e.preventDefault();
                    handleDetachFreeTag(tag.tag_id);
                  }}
                >
                  {tag.tag_name}
                </Tag>
              ))}
              {detail.tags.length === 0 && <EmptyValue text="No tags" />}
              <Space.Compact size="small">
                <Input
                  placeholder="Add tag..."
                  value={newTagInput}
                  onChange={(e) => setNewTagInput(e.target.value)}
                  onPressEnter={handleAddFreeTag}
                  style={{ width: 140 }}
                  size="small"
                />
                <Button
                  size="small"
                  icon={<PlusOutlined />}
                  onClick={handleAddFreeTag}
                  loading={addingFreeTag}
                />
              </Space.Compact>
            </Space>
          </div>

          <Descriptions column={1} bordered size="small" labelStyle={{ width: 200 }}>
            <Descriptions.Item label="Visibility">
              {detail.visibility_name || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label="Language">
              {detail.language_name || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label={<>Source Reference <AiHint /></>}>
              {term.source_reference || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label={<>Regulatory Reference <AiHint /></>}>
              {term.regulatory_reference || <EmptyValue />}
            </Descriptions.Item>
            <Descriptions.Item label="External Reference">
              {term.external_reference ? (
                term.external_reference.startsWith('http') ? (
                  <a href={term.external_reference} target="_blank" rel="noopener noreferrer">
                    {term.external_reference} <LinkOutlined />
                  </a>
                ) : (
                  term.external_reference
                )
              ) : (
                <EmptyValue />
              )}
            </Descriptions.Item>
          </Descriptions>
        </div>
      ),
    },
  ];

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/glossary')}>Business Glossary</a> },
          { title: term.term_name },
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
            onClick={() => navigate('/glossary')}
          />
          <Title level={3} style={{ margin: 0 }}>
            {term.term_name}
          </Title>
          {term.term_code && (
            <Tag color="geekblue" style={{ fontFamily: 'monospace', fontSize: 12 }}>
              {term.term_code}
            </Tag>
          )}
          <Tag
            color={statusColors[status] || 'default'}
            style={{ fontSize: 14, padding: '2px 12px' }}
          >
            {statusLabels[status] || status}
          </Tag>
          {term.version_number > 1 && (
            <Tag color="geekblue">v{term.version_number}</Tag>
          )}
          {term.is_cbt && (
            <Tag color="red">
              <SafetyCertificateOutlined /> CBT
            </Tag>
          )}
          {detail.term_type_name && (
            <Tag color="purple">{detail.term_type_name}</Tag>
          )}
        </Space>
        <Space wrap>{renderActionButtons()}</Space>
      </div>

      {/* --- CBT Banner --- */}
      {term.is_cbt && (
        <Alert
          message="Critical Business Term"
          description="This term is flagged as a Critical Business Term (CBT). It requires enhanced governance, quality controls, and regular review. Linked data elements inherit CDE status."
          type="error"
          showIcon
          icon={<SafetyCertificateOutlined />}
          style={{ marginBottom: 16 }}
          banner
        />
      )}

      {/* --- Amendment Context Banners --- */}
      {detail.previous_version_id && (
        <Alert
          message={`Amendment of v${(term.version_number || 2) - 1}`}
          description={
            <span>
              This is a draft amendment. The original accepted version remains visible until this amendment is approved.{' '}
              <Link to={`/glossary/${detail.previous_version_id}`}>View original version</Link>
            </span>
          }
          type="info"
          showIcon
          icon={<EditOutlined />}
          style={{ marginBottom: 16 }}
          banner
        />
      )}

      {/* --- AI Enrichment Panel --- */}
      <AiEnrichmentPanel
        entityType="glossary_term"
        entityId={id!}
        onSuggestionApplied={fetchDetail}
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
              message="All ownership fields must be assigned before this term can be submitted for review."
              type="warning"
              showIcon
              style={{ marginBottom: 16 }}
            />
          )}
          <Row gutter={[16, 16]}>
            <Col xs={24} md={12}>
              <div style={{ marginBottom: 4 }}>
                <Text strong>Business Term Owner</Text>
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
                <Text strong>Data Domain Owner</Text>
                {!domainOwnerUserId && <Text type="danger"> *</Text>}
              </div>
              <Select
                style={{ width: '100%' }}
                labelInValue
                value={domainOwnerUserId}
                onChange={(val) => setDomainOwnerUserId(val || undefined)}
                options={allUsers.map((u) => ({ value: u.user_id, label: `${u.display_name} (${u.email})` }))}
                placeholder="Select domain owner..."
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
              disabled={!ownerUserId && !stewardUserId && !domainOwnerUserId && !approverUserId}
            >
              Save Ownership
            </Button>
          </div>
        </Card>
      )}

      {/* --- 9-Section Collapse --- */}
      <Card style={{ marginBottom: 24 }}>
        <Collapse
          defaultActiveKey={['core', 'definition']}
          ghost
          items={collapseItems}
          size="large"
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
            You are about to <strong>{transitionAction.toLowerCase()}</strong> this term.
          </Text>
        </div>
        <Input.TextArea
          rows={3}
          placeholder="Add comments (optional)"
          value={transitionComments}
          onChange={(e) => setTransitionComments(e.target.value)}
        />
      </Modal>
    </div>
  );
};

export default GlossaryTermDetail;

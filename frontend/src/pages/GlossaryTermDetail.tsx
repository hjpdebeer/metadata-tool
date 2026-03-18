import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  Breadcrumb,
  Button,
  Card,
  Descriptions,
  Divider,
  Input,
  Modal,
  Space,
  Spin,
  Tag,
  Timeline,
  Typography,
  message,
} from 'antd';
import {
  ArrowLeftOutlined,
  CheckOutlined,
  CloseOutlined,
  EditOutlined,
  SendOutlined,
  UndoOutlined,
} from '@ant-design/icons';
import { glossaryApi, workflowApi } from '../services/glossaryApi';
import type { GlossaryTerm, WorkflowInstanceView } from '../services/glossaryApi';
import { useAuth } from '../hooks/useAuth';
import AiEnrichmentPanel from '../components/AiEnrichmentPanel';

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

const GlossaryTermDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();

  const [term, setTerm] = useState<GlossaryTerm | null>(null);
  const [workflowInstance, setWorkflowInstance] = useState<WorkflowInstanceView | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [transitionModalOpen, setTransitionModalOpen] = useState(false);
  const [transitionAction, setTransitionAction] = useState('');
  const [transitionComments, setTransitionComments] = useState('');

  const isSteward = user?.roles?.includes('data_steward') || user?.roles?.includes('admin');

  const fetchTerm = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await glossaryApi.getTerm(id);
      setTerm(response.data);

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
      message.error('Failed to load term details.');
      navigate('/glossary');
    } finally {
      setLoading(false);
    }
  }, [id, navigate]);

  useEffect(() => {
    fetchTerm();
  }, [fetchTerm]);

  const handleWorkflowAction = (action: string) => {
    if (!term?.workflow_instance_id) {
      message.error('No active workflow for this term.');
      return;
    }
    setTransitionAction(action);
    setTransitionComments('');
    setTransitionModalOpen(true);
  };

  const submitTransition = async () => {
    if (!term?.workflow_instance_id) return;

    setActionLoading(true);
    try {
      await workflowApi.transitionWorkflow(
        term.workflow_instance_id,
        transitionAction,
        transitionComments || undefined,
      );
      message.success(`Workflow action "${transitionAction}" completed successfully.`);
      setTransitionModalOpen(false);
      fetchTerm();
    } catch {
      message.error(`Failed to perform action "${transitionAction}".`);
    } finally {
      setActionLoading(false);
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

  if (!term) {
    return null;
  }

  const status = term.status_code || 'DRAFT';

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
        onClick={() => navigate(`/glossary/${id}/edit`)}
        disabled={status === 'ACCEPTED' || status === 'DEPRECATED'}
      >
        Edit
      </Button>,
    );

    return buttons;
  };

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/glossary')}>Business Glossary</a> },
          { title: term.term_name },
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
            onClick={() => navigate('/glossary')}
          />
          <Title level={3} style={{ margin: 0 }}>
            {term.term_name}
          </Title>
          <Tag
            color={statusColors[status] || 'default'}
            style={{ fontSize: 14, padding: '2px 12px' }}
          >
            {statusLabels[status] || status}
          </Tag>
        </Space>
        <Space>{renderActionButtons()}</Space>
      </div>

      <Card title="Term Details" style={{ marginBottom: 24 }}>
        <Descriptions column={{ xs: 1, sm: 1, md: 2 }} bordered size="small">
          <Descriptions.Item label="Term Name">{term.term_name}</Descriptions.Item>
          <Descriptions.Item label="Abbreviation">
            {term.abbreviation || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Definition" span={2}>
            {term.definition}
          </Descriptions.Item>
          <Descriptions.Item label="Business Context" span={2}>
            {term.business_context || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Examples" span={2}>
            {term.examples || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Domain">
            {term.domain_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Category">
            {term.category_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Source Reference">
            {term.source_reference || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Regulatory Reference">
            {term.regulatory_reference || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Owner">
            {term.owner_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Steward">
            {term.steward_name || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Version">{term.version_number}</Descriptions.Item>
          <Descriptions.Item label="Status">
            <Tag color={statusColors[status] || 'default'}>
              {statusLabels[status] || status}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Created">
            {formatDate(term.created_at)}
            {term.created_by_name ? ` by ${term.created_by_name}` : ''}
          </Descriptions.Item>
          <Descriptions.Item label="Last Updated">
            {formatDate(term.updated_at)}
            {term.updated_by_name ? ` by ${term.updated_by_name}` : ''}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <AiEnrichmentPanel
        entityType="glossary_term"
        entityId={id!}
        onSuggestionApplied={fetchTerm}
      />

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

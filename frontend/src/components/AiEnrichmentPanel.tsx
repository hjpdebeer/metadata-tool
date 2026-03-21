import React, { useCallback, useEffect, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Collapse,
  Empty,
  Input,
  List,
  Modal,
  Progress,
  Rate,
  Select,
  Space,
  Spin,
  Tag,
  Tooltip,
  Typography,
  message,
} from 'antd';
import {
  BulbOutlined,
  CheckOutlined,
  CloseOutlined,
  EditOutlined,
  RobotOutlined,
  StarOutlined,
} from '@ant-design/icons';
import { aiApi } from '../services/aiApi';
import type { AiSuggestion } from '../services/aiApi';
import { glossaryApi } from '../services/glossaryApi';

const { Text, Paragraph } = Typography;

// Lookup fields where AI returns UUIDs (Section 15.6) or fixed enum values.
// These show a dropdown in the modify modal instead of a text input.
const LOOKUP_FIELDS = ['domain', 'category', 'data_classification', 'term_type', 'unit_of_measure', 'data_type'];

// Static options for non-UUID dropdown fields
const DATA_TYPE_OPTIONS = [
  'VARCHAR', 'CHAR', 'TEXT', 'INTEGER', 'BIGINT', 'SMALLINT',
  'DECIMAL', 'NUMERIC', 'FLOAT', 'DOUBLE', 'BOOLEAN',
  'DATE', 'TIMESTAMP', 'TIMESTAMPTZ', 'UUID', 'JSON', 'JSONB', 'BLOB', 'CLOB',
].map((t) => ({ value: t, label: t }));

interface AiEnrichmentPanelProps {
  entityType: string; // "glossary_term", "data_element", or "application"
  entityId: string;
  onSuggestionApplied?: () => void; // callback to refresh parent data
}

const confidenceColor = (confidence: number): string => {
  if (confidence >= 0.8) return '#52C41A';
  if (confidence >= 0.6) return '#1677FF';
  if (confidence >= 0.4) return '#FAAD14';
  return '#FF4D4F';
};

const sourceLabel = (source: string): string => {
  switch (source) {
    case 'CLAUDE':
      return 'Claude';
    case 'OPENAI':
      return 'OpenAI';
    default:
      return source;
  }
};

const statusTag = (status: string) => {
  switch (status) {
    case 'PENDING':
      return <Tag color="processing">Pending Review</Tag>;
    case 'ACCEPTED':
      return <Tag color="success">Accepted</Tag>;
    case 'MODIFIED':
      return <Tag color="cyan">Accepted (Modified)</Tag>;
    case 'REJECTED':
      return <Tag color="error">Rejected</Tag>;
    default:
      return <Tag>{status}</Tag>;
  }
};

const AiEnrichmentPanel: React.FC<AiEnrichmentPanelProps> = ({
  entityType,
  entityId,
  onSuggestionApplied,
}) => {
  const [suggestions, setSuggestions] = useState<AiSuggestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [enriching, setEnriching] = useState(false);
  // UUID → display name map for lookup fields (Section 15.6)
  const [lookupNames, setLookupNames] = useState<Record<string, string>>({});
  // field_name → [{value: uuid, label: name}] for Select dropdowns in Modify modal
  const [lookupOptions, setLookupOptions] = useState<Record<string, { value: string; label: string }[]>>({});
  const [initialLoaded, setInitialLoaded] = useState(false);

  // Modify modal state
  const [modifyModalOpen, setModifyModalOpen] = useState(false);
  const [modifySuggestionId, setModifySuggestionId] = useState<string | null>(null);
  const [modifyValue, setModifyValue] = useState('');
  const [modifyFieldName, setModifyFieldName] = useState('');
  const [modifyLoading, setModifyLoading] = useState(false);

  // Feedback modal state
  const [feedbackModalOpen, setFeedbackModalOpen] = useState(false);
  const [feedbackSuggestionId, setFeedbackSuggestionId] = useState<string | null>(null);
  const [feedbackRating, setFeedbackRating] = useState<number>(0);
  const [feedbackText, setFeedbackText] = useState('');
  const [feedbackLoading, setFeedbackLoading] = useState(false);

  // Fetch lookup tables to resolve UUIDs → display names for lookup field suggestions
  const fetchLookupNames = useCallback(async () => {
    const map: Record<string, string> = {};
    const opts: Record<string, { value: string; label: string }[]> = {};

    try {
      if (entityType === 'glossary_term') {
        const [domains, categories, classifications, termTypes, units] = await Promise.allSettled([
          glossaryApi.listDomains(),
          glossaryApi.listCategories(),
          glossaryApi.listClassifications(),
          glossaryApi.listTermTypes(),
          glossaryApi.listUnitsOfMeasure(),
        ]);
        if (domains.status === 'fulfilled') {
          domains.value.data.forEach((d) => { map[d.domain_id] = d.domain_name; });
          opts['domain'] = domains.value.data.map((d) => ({ value: d.domain_id, label: d.domain_name }));
        }
        if (categories.status === 'fulfilled') {
          categories.value.data.forEach((c) => { map[c.category_id] = c.category_name; });
          opts['category'] = categories.value.data.map((c) => ({ value: c.category_id, label: c.category_name }));
        }
        if (classifications.status === 'fulfilled') {
          classifications.value.data.forEach((c) => { map[c.classification_id] = c.classification_name; });
          opts['data_classification'] = classifications.value.data.map((c) => ({ value: c.classification_id, label: c.classification_name }));
        }
        if (termTypes.status === 'fulfilled') {
          termTypes.value.data.forEach((t) => { map[t.term_type_id] = t.type_name; });
          opts['term_type'] = termTypes.value.data.map((t) => ({ value: t.term_type_id, label: t.type_name }));
        }
        if (units.status === 'fulfilled') {
          units.value.data.forEach((u) => { map[u.unit_id] = u.unit_name; });
          opts['unit_of_measure'] = units.value.data.map((u) => ({ value: u.unit_id, label: u.unit_name }));
        }
      } else if (entityType === 'data_element') {
        const [domains, classifications] = await Promise.allSettled([
          glossaryApi.listDomains(),
          glossaryApi.listClassifications(),
        ]);
        if (domains.status === 'fulfilled') {
          domains.value.data.forEach((d) => { map[d.domain_id] = d.domain_name; });
          opts['domain'] = domains.value.data.map((d) => ({ value: d.domain_id, label: d.domain_name }));
        }
        if (classifications.status === 'fulfilled') {
          classifications.value.data.forEach((c) => { map[c.classification_id] = c.classification_name; });
          opts['data_classification'] = classifications.value.data.map((c) => ({ value: c.classification_id, label: c.classification_name }));
        }
      } else if (entityType === 'application') {
        const [classifications] = await Promise.allSettled([
          glossaryApi.listClassifications(),
        ]);
        if (classifications.status === 'fulfilled') {
          classifications.value.data.forEach((c) => { map[c.classification_id] = c.classification_name; });
          opts['data_classification'] = classifications.value.data.map((c) => ({ value: c.classification_id, label: c.classification_name }));
        }
      }
    } catch {
      // Non-critical
    }

    // Add static data_type options (not a UUID lookup, but a fixed enum dropdown)
    opts['data_type'] = DATA_TYPE_OPTIONS;

    setLookupNames(map);
    setLookupOptions(opts);
  }, [entityType]);

  const fetchSuggestions = useCallback(async () => {
    if (!entityId) return;
    setLoading(true);
    try {
      const response = await aiApi.listSuggestions(entityType, entityId);
      setSuggestions(response.data);
    } catch {
      // Silently fail — panel is supplementary
    } finally {
      setLoading(false);
      setInitialLoaded(true);
    }
  }, [entityType, entityId]);

  useEffect(() => {
    fetchLookupNames();
    fetchSuggestions();
  }, [fetchLookupNames, fetchSuggestions]);

  const handleEnrich = async () => {
    setEnriching(true);
    try {
      const response = await aiApi.enrich(entityType, entityId);
      const newSuggestions = response.data.suggestions;
      if (newSuggestions.length === 0) {
        message.info('AI found no improvements to suggest for this entity.');
      } else {
        message.success(
          `AI generated ${newSuggestions.length} suggestion${newSuggestions.length > 1 ? 's' : ''} (${response.data.provider}).`,
        );
      }
      // Refresh the full list
      await fetchSuggestions();
    } catch (error: unknown) {
      const errMsg =
        error && typeof error === 'object' && 'response' in error
          ? (error as { response?: { data?: { error?: { message?: string } } } }).response?.data
              ?.error?.message || 'AI enrichment failed.'
          : 'AI enrichment failed.';
      message.error(errMsg);
    } finally {
      setEnriching(false);
    }
  };

  const handleAccept = async (suggestionId: string) => {
    try {
      await aiApi.acceptSuggestion(suggestionId);
      message.success('Suggestion accepted and applied.');
      await fetchSuggestions();
      onSuggestionApplied?.();
    } catch {
      message.error('Failed to accept suggestion.');
    }
  };

  const handleReject = async (suggestionId: string) => {
    try {
      await aiApi.rejectSuggestion(suggestionId);
      message.success('Suggestion rejected.');
      await fetchSuggestions();
    } catch {
      message.error('Failed to reject suggestion.');
    }
  };

  const openModifyModal = (suggestion: AiSuggestion) => {
    setModifySuggestionId(suggestion.suggestion_id);
    setModifyValue(suggestion.suggested_value);
    setModifyFieldName(suggestion.field_name);
    setModifyModalOpen(true);
  };

  const handleModifySubmit = async () => {
    if (!modifySuggestionId) return;
    setModifyLoading(true);
    try {
      await aiApi.acceptSuggestion(modifySuggestionId, modifyValue);
      message.success('Modified suggestion accepted and applied.');
      setModifyModalOpen(false);
      await fetchSuggestions();
      onSuggestionApplied?.();
    } catch {
      message.error('Failed to apply modified suggestion.');
    } finally {
      setModifyLoading(false);
    }
  };

  const openFeedbackModal = (suggestionId: string) => {
    setFeedbackSuggestionId(suggestionId);
    setFeedbackRating(0);
    setFeedbackText('');
    setFeedbackModalOpen(true);
  };

  const handleFeedbackSubmit = async () => {
    if (!feedbackSuggestionId) return;
    if (feedbackRating === 0 && !feedbackText.trim()) {
      message.warning('Please provide a rating or feedback text.');
      return;
    }
    setFeedbackLoading(true);
    try {
      await aiApi.submitFeedback(
        feedbackSuggestionId,
        feedbackRating > 0 ? feedbackRating : undefined,
        feedbackText.trim() || undefined,
      );
      message.success('Feedback submitted. Thank you!');
      setFeedbackModalOpen(false);
    } catch {
      message.error('Failed to submit feedback.');
    } finally {
      setFeedbackLoading(false);
    }
  };

  const pendingSuggestions = suggestions.filter((s) => s.status === 'PENDING');
  const resolvedSuggestions = suggestions.filter((s) => s.status !== 'PENDING');

  const renderSuggestionItem = (suggestion: AiSuggestion) => {
    const isPending = suggestion.status === 'PENDING';

    return (
      <List.Item
        key={suggestion.suggestion_id}
        style={{
          padding: '12px 16px',
          borderLeft: isPending ? `3px solid ${confidenceColor(suggestion.confidence)}` : undefined,
          backgroundColor: isPending ? '#FAFAFA' : undefined,
        }}
      >
        <div style={{ width: '100%' }}>
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'flex-start',
              marginBottom: 8,
            }}
          >
            <Space size={8} align="center">
              <Text strong style={{ fontSize: 13 }}>
                {suggestion.field_name.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())}
              </Text>
              {statusTag(suggestion.status)}
              <Tooltip title={`Source: ${sourceLabel(suggestion.source)}${suggestion.model ? ` (${suggestion.model})` : ''}`}>
                <Tag color="default" style={{ fontSize: 11 }}>
                  <RobotOutlined /> {sourceLabel(suggestion.source)}
                </Tag>
              </Tooltip>
            </Space>
            <Tooltip title={`Confidence: ${Math.round(suggestion.confidence * 100)}%`}>
              <Progress
                type="circle"
                percent={Math.round(suggestion.confidence * 100)}
                size={36}
                strokeColor={confidenceColor(suggestion.confidence)}
                format={(p) => `${p}%`}
              />
            </Tooltip>
          </div>

          <div
            style={{
              background: isPending ? '#F0F5FF' : '#F5F5F5',
              borderRadius: 6,
              padding: '8px 12px',
              marginBottom: 8,
            }}
          >
            <Text style={{ whiteSpace: 'pre-wrap' }}>
              {LOOKUP_FIELDS.includes(suggestion.field_name) && lookupNames[suggestion.suggested_value]
                ? lookupNames[suggestion.suggested_value]
                : suggestion.suggested_value}
            </Text>
          </div>

          {suggestion.rationale && (
            <Paragraph
              type="secondary"
              style={{ fontSize: 12, marginBottom: 8 }}
              ellipsis={{ rows: 2, expandable: true, symbol: 'more' }}
            >
              <BulbOutlined style={{ marginRight: 4 }} />
              {suggestion.rationale}
            </Paragraph>
          )}

          {isPending && (
            <Space size={8}>
              <Button
                type="primary"
                size="small"
                icon={<CheckOutlined />}
                onClick={() => handleAccept(suggestion.suggestion_id)}
              >
                Accept
              </Button>
              <Button
                size="small"
                icon={<EditOutlined />}
                onClick={() => openModifyModal(suggestion)}
              >
                Modify
              </Button>
              <Button
                size="small"
                danger
                icon={<CloseOutlined />}
                onClick={() => handleReject(suggestion.suggestion_id)}
              >
                Reject
              </Button>
            </Space>
          )}

          {!isPending && (
            <Button
              type="link"
              size="small"
              icon={<StarOutlined />}
              onClick={() => openFeedbackModal(suggestion.suggestion_id)}
              style={{ padding: 0, fontSize: 12 }}
            >
              Rate this suggestion
            </Button>
          )}
        </div>
      </List.Item>
    );
  };

  return (
    <>
      <Card
        title={
          <Space>
            <RobotOutlined />
            <span>AI Metadata Enrichment</span>
          </Space>
        }
        extra={
          <Button
            type="primary"
            icon={<BulbOutlined />}
            onClick={handleEnrich}
            loading={enriching}
          >
            {enriching ? 'Analysing...' : 'AI Enrich'}
          </Button>
        }
        style={{ marginBottom: 24 }}
      >
        {!initialLoaded && loading ? (
          <div style={{ textAlign: 'center', padding: 24 }}>
            <Spin />
          </div>
        ) : (
          <>
            {enriching && (
              <Alert
                message="AI is analysing this entity and generating suggestions..."
                type="info"
                showIcon
                style={{ marginBottom: 16 }}
              />
            )}

            {pendingSuggestions.length > 0 && (
              <>
                <Text strong style={{ display: 'block', marginBottom: 8 }}>
                  Pending Review ({pendingSuggestions.length})
                </Text>
                <List
                  dataSource={pendingSuggestions}
                  renderItem={renderSuggestionItem}
                  bordered
                  size="small"
                  style={{ marginBottom: 16 }}
                />
              </>
            )}

            {pendingSuggestions.length === 0 && !enriching && initialLoaded && (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={
                  suggestions.length === 0
                    ? 'No AI suggestions yet. Click "AI Enrich" to generate suggestions.'
                    : 'No pending suggestions. All suggestions have been reviewed.'
                }
                style={{ margin: '16px 0' }}
              />
            )}

            {resolvedSuggestions.length > 0 && (
              <Collapse
                ghost
                items={[
                  {
                    key: 'resolved',
                    label: (
                      <Text type="secondary">
                        Previously reviewed ({resolvedSuggestions.length})
                      </Text>
                    ),
                    children: (
                      <List
                        dataSource={resolvedSuggestions}
                        renderItem={renderSuggestionItem}
                        size="small"
                      />
                    ),
                  },
                ]}
              />
            )}
          </>
        )}
      </Card>

      {/* Modify Modal */}
      <Modal
        title={`Modify Suggestion: ${modifyFieldName.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())}`}
        open={modifyModalOpen}
        onOk={handleModifySubmit}
        onCancel={() => setModifyModalOpen(false)}
        confirmLoading={modifyLoading}
        okText="Accept with Changes"
      >
        <div style={{ marginBottom: 12 }}>
          <Text type="secondary">
            Edit the suggested value below, then click accept. The modified value will be applied to
            the entity.
          </Text>
        </div>
        {LOOKUP_FIELDS.includes(modifyFieldName) && lookupOptions[modifyFieldName] ? (
          <Select
            style={{ width: '100%' }}
            value={modifyValue || undefined}
            onChange={(val) => setModifyValue(val)}
            options={lookupOptions[modifyFieldName]}
            placeholder={`Select ${modifyFieldName.replace(/_/g, ' ')}...`}
            showSearch
            optionFilterProp="label"
            allowClear
          />
        ) : (
          <Input.TextArea
            rows={6}
            value={modifyValue}
            onChange={(e) => setModifyValue(e.target.value)}
            placeholder="Modified value..."
          />
        )}
      </Modal>

      {/* Feedback Modal */}
      <Modal
        title="Rate AI Suggestion"
        open={feedbackModalOpen}
        onOk={handleFeedbackSubmit}
        onCancel={() => setFeedbackModalOpen(false)}
        confirmLoading={feedbackLoading}
        okText="Submit Feedback"
      >
        <div style={{ marginBottom: 16 }}>
          <Text>How useful was this suggestion?</Text>
          <div style={{ marginTop: 8 }}>
            <Rate value={feedbackRating} onChange={setFeedbackRating} />
          </div>
        </div>
        <Input.TextArea
          rows={3}
          placeholder="Additional feedback (optional)"
          value={feedbackText}
          onChange={(e) => setFeedbackText(e.target.value)}
        />
      </Modal>
    </>
  );
};

export default AiEnrichmentPanel;

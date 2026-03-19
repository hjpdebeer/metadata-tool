import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  Alert,
  Breadcrumb,
  Button,
  Card,
  Col,
  Divider,
  Form,
  Input,
  Row,
  Select,
  Space,
  Spin,
  Switch,
  Tag,
  Typography,
  message,
} from 'antd';
import { ArrowLeftOutlined, PlusOutlined, RobotOutlined } from '@ant-design/icons';
import { glossaryApi } from '../services/glossaryApi';
import { aiApi } from '../services/aiApi';
import { usersApi } from '../services/usersApi';
import type {
  CreateGlossaryTermRequest,
  DataClassificationRef,
  GlossaryCategory,
  GlossaryConfidenceLevel,
  GlossaryDomain,
  GlossaryLanguage,
  GlossaryReviewFrequency,
  GlossaryTerm,
  GlossaryTermDetailView,
  GlossaryTermListItem,
  GlossaryTermType,
  GlossaryUnitOfMeasure,
  GlossaryVisibilityLevel,
  UpdateGlossaryTermRequest,
} from '../services/glossaryApi';
import type { UserListItem } from '../services/usersApi';

const { Title, Text } = Typography;
const { TextArea } = Input;

const GlossaryTermForm: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const isEditing = Boolean(id);

  // Reference data
  const [domains, setDomains] = useState<GlossaryDomain[]>([]);
  const [categories, setCategories] = useState<GlossaryCategory[]>([]);
  const [termTypes, setTermTypes] = useState<GlossaryTermType[]>([]);
  const [unitsOfMeasure, setUnitsOfMeasure] = useState<GlossaryUnitOfMeasure[]>([]);
  const [classifications, setClassifications] = useState<DataClassificationRef[]>([]);
  const [reviewFrequencies, setReviewFrequencies] = useState<GlossaryReviewFrequency[]>([]);
  const [, setConfidenceLevels] = useState<GlossaryConfidenceLevel[]>([]);
  const [visibilityLevels, setVisibilityLevels] = useState<GlossaryVisibilityLevel[]>([]);
  const [languages, setLanguages] = useState<GlossaryLanguage[]>([]);
  const [users, setUsers] = useState<UserListItem[]>([]);

  const [allTerms, setAllTerms] = useState<GlossaryTermListItem[]>([]);
  const [aliases, setAliases] = useState<{ alias_id: string; alias_name: string; alias_type: string | null }[]>([]);
  const [addingAlias, setAddingAlias] = useState(false);
  const [newAliasName, setNewAliasName] = useState('');

  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [existingTerm, setExistingTerm] = useState<GlossaryTerm | null>(null);

  const fetchReferenceData = useCallback(async () => {
    const results = await Promise.allSettled([
      glossaryApi.listDomains(),
      glossaryApi.listCategories(),
      glossaryApi.listTermTypes(),
      glossaryApi.listUnitsOfMeasure(),
      glossaryApi.listClassifications(),
      glossaryApi.listReviewFrequencies(),
      glossaryApi.listConfidenceLevels(),
      glossaryApi.listVisibilityLevels(),
      glossaryApi.listLanguages(),
      usersApi.listUsers({ page_size: 500, is_active: true }),
      glossaryApi.listTerms({ page_size: 500 }),
    ]);

    if (results[0].status === 'fulfilled') setDomains(results[0].value.data);
    if (results[1].status === 'fulfilled') setCategories(results[1].value.data);
    if (results[2].status === 'fulfilled') setTermTypes(results[2].value.data);
    if (results[3].status === 'fulfilled') setUnitsOfMeasure(results[3].value.data);
    if (results[4].status === 'fulfilled') setClassifications(results[4].value.data);
    if (results[5].status === 'fulfilled') setReviewFrequencies(results[5].value.data);
    if (results[6].status === 'fulfilled') setConfidenceLevels(results[6].value.data);
    if (results[7].status === 'fulfilled') setVisibilityLevels(results[7].value.data);
    if (results[8].status === 'fulfilled') setLanguages(results[8].value.data);
    if (results[9].status === 'fulfilled') {
      const userData = results[9].value.data;
      // Handle paginated or flat response
      if (Array.isArray(userData)) {
        setUsers(userData);
      } else {
        setUsers((userData as unknown as { data: UserListItem[] }).data || []);
      }
    }
    if (results[10].status === 'fulfilled') {
      const termData = results[10].value.data;
      if (Array.isArray(termData)) {
        setAllTerms(termData);
      } else {
        setAllTerms((termData as unknown as { data: GlossaryTermListItem[] }).data || []);
      }
    }
  }, []);

  const fetchExistingTerm = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await glossaryApi.getTermDetail(id);
      const detail: GlossaryTermDetailView = response.data;
      // Also store as GlossaryTerm for change-diffing
      setExistingTerm(detail as unknown as GlossaryTerm);
      setAliases(detail.aliases || []);
      form.setFieldsValue({
        term_name: detail.term_name,
        definition: detail.definition,
        definition_notes: detail.definition_notes || undefined,
        counter_examples: detail.counter_examples || undefined,
        formula: detail.formula || undefined,
        business_context: detail.business_context || undefined,
        examples: detail.examples || undefined,
        abbreviation: detail.abbreviation || undefined,
        domain_id: detail.domain_id || undefined,
        category_id: detail.category_id || undefined,
        term_type_id: detail.term_type_id || undefined,
        unit_of_measure_id: detail.unit_of_measure_id || undefined,
        classification_id: detail.classification_id || undefined,
        owner_user_id: detail.owner_user_id || undefined,
        steward_user_id: detail.steward_user_id || undefined,
        domain_owner_user_id: detail.domain_owner_user_id || undefined,
        approver_user_id: detail.approver_user_id || undefined,
        organisational_unit: detail.organisational_unit || undefined,
        review_frequency_id: detail.review_frequency_id || undefined,
        is_cbt: detail.is_cbt,
        golden_source: detail.golden_source || undefined,
        confidence_level_id: detail.confidence_level_id || undefined,
        visibility_id: detail.visibility_id || undefined,
        language_id: detail.language_id || undefined,
        used_in_reports: detail.used_in_reports || undefined,
        used_in_policies: detail.used_in_policies || undefined,
        regulatory_reporting_usage: detail.regulatory_reporting_usage || undefined,
        source_reference: detail.source_reference || undefined,
        regulatory_reference: detail.regulatory_reference || undefined,
        external_reference: detail.external_reference || undefined,
        parent_term_id: detail.parent_term_id || undefined,
      });
    } catch {
      message.error('Failed to load term for editing.');
      navigate('/glossary');
    } finally {
      setLoading(false);
    }
  }, [id, form, navigate]);

  useEffect(() => {
    if (isEditing) {
      // Load lookups FIRST so Select options exist, THEN load term values.
      // Without this order, Selects show raw UUIDs because options aren't loaded yet.
      fetchReferenceData().then(() => fetchExistingTerm());
    }
  }, [isEditing, fetchReferenceData, fetchExistingTerm]);

  const handleCreateSubmit = async (values: { term_name: string; definition: string }) => {
    setSubmitting(true);
    try {
      const cleanData: CreateGlossaryTermRequest = {
        term_name: values.term_name.trim(),
        definition: values.definition.trim(),
      };

      const response = await glossaryApi.createTerm(cleanData);
      const newTermId = response.data.term_id;
      message.success('Term created. Generating AI suggestions...');

      // Trigger AI enrichment immediately after creation
      try {
        await aiApi.enrich('glossary_term', newTermId);
        message.success('AI suggestions ready for review.');
      } catch {
        message.info('Term created. AI enrichment unavailable — you can enrich later from the detail page.');
      }

      navigate(`/glossary/${newTermId}`);
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { error?: { message?: string } }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.error?.message || 'Validation error.');
      } else {
        message.error('Failed to create term.');
      }
    } finally {
      setSubmitting(false);
    }
  };

  const handleEditSubmit = async (values: Record<string, unknown>) => {
    if (!id) return;
    setSubmitting(true);
    try {
      const updateData: UpdateGlossaryTermRequest = {};

      // Build diff of changed fields
      const allFields = [
        'term_name', 'definition', 'definition_notes', 'counter_examples', 'formula',
        'business_context', 'examples', 'abbreviation', 'domain_id', 'category_id',
        'term_type_id', 'unit_of_measure_id', 'classification_id',
        'owner_user_id', 'steward_user_id', 'domain_owner_user_id', 'approver_user_id',
        'organisational_unit', 'review_frequency_id',
        'golden_source', 'confidence_level_id', 'visibility_id', 'language_id',
        'used_in_reports', 'used_in_policies', 'regulatory_reporting_usage',
        'source_reference', 'regulatory_reference', 'external_reference',
        'parent_term_id',
      ] as const;

      for (const field of allFields) {
        const newVal = values[field];
        const oldVal = existingTerm?.[field as keyof GlossaryTerm];
        if (newVal !== oldVal) {
          (updateData as Record<string, unknown>)[field] = newVal || undefined;
        }
      }

      // Handle boolean field separately
      if (values.is_cbt !== existingTerm?.is_cbt) {
        updateData.is_cbt = values.is_cbt as boolean;
      }

      const response = await glossaryApi.updateTerm(id, updateData);
      message.success('Term updated successfully.');
      navigate(`/glossary/${response.data.term_id}`);
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { error?: { message?: string } }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.error?.message || 'Validation error.');
      } else {
        message.error('Failed to update term.');
      }
    } finally {
      setSubmitting(false);
    }
  };

  if (loading) {
    return (
      <div style={{ textAlign: 'center', padding: 80 }}>
        <Spin size="large" />
      </div>
    );
  }

  // =====================================================================
  // CREATE MODE: Simplified form (name + definition only)
  // =====================================================================
  if (!isEditing) {
    return (
      <div>
        <Breadcrumb
          style={{ marginBottom: 16 }}
          items={[
            { title: <a onClick={() => navigate('/glossary')}>Business Glossary</a> },
            { title: 'New Term' },
          ]}
        />

        <Space align="center" style={{ marginBottom: 16 }}>
          <Button type="text" icon={<ArrowLeftOutlined />} onClick={() => navigate('/glossary')} />
          <Title level={3} style={{ margin: 0 }}>New Glossary Term</Title>
        </Space>

        <Card>
          <Alert
            message="AI-Assisted Creation"
            description="Enter the term name and definition. After creation, AI will automatically suggest values for business context, examples, abbreviation, domain, category, and references based on financial services standards. You'll review and accept each suggestion."
            type="info"
            showIcon
            icon={<RobotOutlined />}
            style={{ marginBottom: 24 }}
          />

          <Form
            form={form}
            layout="vertical"
            onFinish={handleCreateSubmit}
            onFinishFailed={(errorInfo) => {
              console.error('Form validation failed:', errorInfo);
            }}
            style={{ maxWidth: 800 }}
            scrollToFirstError
          >
            <Form.Item
              name="term_name"
              label="Term Name"
              rules={[
                { required: true, message: 'Term name is required' },
                { max: 255, message: 'Term name cannot exceed 255 characters' },
              ]}
            >
              <Input placeholder="e.g., Customer Due Diligence, Net Interest Margin, Know Your Customer" size="large" />
            </Form.Item>

            <Form.Item
              name="definition"
              label="Definition"
              rules={[
                { required: true, message: 'Definition is required' },
                { min: 10, message: 'Definition should be at least 10 characters' },
              ]}
            >
              <TextArea
                rows={4}
                placeholder="Provide a clear, concise definition of this business term. The AI will use this to suggest additional metadata."
                size="large"
              />
            </Form.Item>

            <Form.Item style={{ marginTop: 24 }}>
              <Space>
                <Button
                  type="primary"
                  htmlType="submit"
                  loading={submitting}
                  icon={<RobotOutlined />}
                  size="large"
                >
                  {submitting ? 'Creating & Enriching...' : 'Create & AI Enrich'}
                </Button>
                <Button onClick={() => navigate('/glossary')} size="large">
                  Cancel
                </Button>
              </Space>
            </Form.Item>
          </Form>

          <Text type="secondary" style={{ fontSize: 12 }}>
            AI suggestions are generated using Claude (Anthropic) based on DAMA DMBOK, BCBS 239, and ISO 8000 standards. All suggestions require your review before being applied.
          </Text>
        </Card>
      </div>
    );
  }

  // =====================================================================
  // EDIT MODE: Full form with all 45 fields in grouped sections
  // =====================================================================
  const domainOptions = domains.map((d) => ({ value: d.domain_id, label: d.domain_name }));
  const categoryOptions = categories.map((c) => ({ value: c.category_id, label: c.category_name }));
  const termTypeOptions = termTypes.map((t) => ({ value: t.term_type_id, label: t.type_name }));
  const unitOptions = unitsOfMeasure.map((u) => ({
    value: u.unit_id,
    label: u.unit_symbol ? `${u.unit_name} (${u.unit_symbol})` : u.unit_name,
  }));
  const classificationOptions = classifications.map((c) => ({
    value: c.classification_id,
    label: c.classification_name,
  }));
  const frequencyOptions = reviewFrequencies.map((f) => ({
    value: f.frequency_id,
    label: f.frequency_name,
  }));
  const visibilityOptions = visibilityLevels.map((v) => ({
    value: v.visibility_id,
    label: v.visibility_name,
  }));
  const languageOptions = languages.map((l) => ({
    value: l.language_id,
    label: l.language_name,
  }));
  const userOptions = users.map((u) => ({
    value: u.user_id,
    label: `${u.display_name} (${u.email})`,
  }));
  // Exclude current term from parent term options to prevent self-reference
  const parentTermOptions = allTerms
    .filter((t) => t.term_id !== id)
    .map((t) => ({ value: t.term_id, label: t.term_name }));

  const handleAddAlias = async () => {
    const name = newAliasName.trim();
    if (!name || !id) return;
    try {
      await glossaryApi.addAlias(id, name);
      // Re-fetch detail to get updated aliases with IDs
      const response = await glossaryApi.getTermDetail(id);
      setAliases(response.data.aliases || []);
      setNewAliasName('');
      setAddingAlias(false);
      message.success('Alias added.');
    } catch {
      message.error('Failed to add alias.');
    }
  };

  const handleRemoveAlias = async (aliasId: string) => {
    if (!id) return;
    try {
      await glossaryApi.removeAlias(id, aliasId);
      setAliases((prev) => prev.filter((a) => a.alias_id !== aliasId));
      message.success('Alias removed.');
    } catch {
      message.error('Failed to remove alias.');
    }
  };

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/glossary')}>Business Glossary</a> },
          ...(existingTerm
            ? [{ title: <a onClick={() => navigate(`/glossary/${id}`)}>{existingTerm.term_name}</a> }]
            : []),
          { title: 'Edit' },
        ]}
      />

      <Space align="center" style={{ marginBottom: 16 }}>
        <Button type="text" icon={<ArrowLeftOutlined />} onClick={() => navigate(`/glossary/${id}`)} />
        <Title level={3} style={{ margin: 0 }}>Edit Term</Title>
        {existingTerm?.term_code && (
          <Text type="secondary" code style={{ fontSize: 12 }}>
            {existingTerm.term_code}
          </Text>
        )}
      </Space>

      <Form
        form={form}
        layout="vertical"
        onFinish={handleEditSubmit}
        scrollToFirstError
        initialValues={{ is_cbt: false }}
      >
        {/* Section 1: Core Identity */}
        <Card
          title="Core Identity"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={16}>
              <Form.Item
                name="term_name"
                label="Term Name"
                rules={[
                  { required: true, message: 'Term name is required' },
                  { max: 255, message: 'Term name cannot exceed 255 characters' },
                ]}
              >
                <Input placeholder="Enter the business term name" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item
                name="abbreviation"
                label="Abbreviation"
                rules={[{ max: 50, message: 'Max 50 characters' }]}
              >
                <Input placeholder="e.g., CDD, NIM" />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* Section 2: Definition & Semantics */}
        <Card
          title="Definition & Semantics"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Form.Item
            name="definition"
            label="Definition"
            rules={[
              { required: true, message: 'Definition is required' },
              { min: 10, message: 'Definition should be at least 10 characters' },
            ]}
          >
            <TextArea rows={4} placeholder="Provide a clear, concise definition" />
          </Form.Item>

          <Form.Item name="definition_notes" label="Definition Notes">
            <TextArea rows={2} placeholder="Additional notes clarifying the definition" />
          </Form.Item>

          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="counter_examples" label="Counter-Examples">
                <TextArea rows={2} placeholder="Examples of what this term does NOT mean" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="examples" label="Examples">
                <TextArea rows={2} placeholder="Provide examples of this term in use" />
              </Form.Item>
            </Col>
          </Row>

          <Row gutter={16}>
            <Col xs={24} md={16}>
              <Form.Item name="formula" label="Formula">
                <Input
                  placeholder="e.g., (Interest Income - Interest Expense) / Average Earning Assets"
                  style={{}}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="unit_of_measure_id" label="Unit of Measure">
                <Select
                  placeholder="Select unit"
                  options={unitOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* Section 3: Classification */}
        <Card
          title="Classification"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="domain_id" label="Domain">
                <Select
                  placeholder="Select domain"
                  options={domainOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="category_id" label="Category">
                <Select
                  placeholder="Select category"
                  options={categoryOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="term_type_id" label="Term Type">
                <Select
                  placeholder="Select type"
                  options={termTypeOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="classification_id" label="Data Classification">
                <Select
                  placeholder="Select classification"
                  options={classificationOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>
          <Text type="secondary" style={{ fontSize: 12 }}>
            Regulatory tags and subject areas can be managed from the detail page after saving.
          </Text>
        </Card>

        {/* Section 4: Ownership */}
        <Card
          title="Ownership"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="owner_user_id" label="Business Term Owner">
                <Select
                  placeholder="Select owner"
                  options={userOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="steward_user_id" label="Data Steward">
                <Select
                  placeholder="Select steward"
                  options={userOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="domain_owner_user_id" label="Data Domain Owner">
                <Select
                  placeholder="Select domain owner"
                  options={userOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="approver_user_id" label="Approver">
                <Select
                  placeholder="Select approver"
                  options={userOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="organisational_unit" label="Organisational Unit">
                <Input placeholder="e.g., Group Risk, Retail Banking, Treasury" />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* Section 5: Lifecycle */}
        <Card
          title="Lifecycle"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="review_frequency_id" label="Review Frequency">
                <Select
                  placeholder="Select frequency"
                  options={frequencyOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>
          <Text type="secondary" style={{ fontSize: 12 }}>
            Next review date is calculated automatically based on the review frequency and approval date.
          </Text>
        </Card>

        {/* Section 6: Relationships */}
        <Card
          title="Relationships"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="parent_term_id" label="Parent Term">
                <Select
                  placeholder="Select parent term"
                  options={parentTermOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>

          <div style={{ marginBottom: 8 }}>
            <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>
              Synonyms / Aliases
            </Text>
            <Space wrap size={[4, 8]}>
              {aliases.map((a) => (
                <Tag
                  key={a.alias_id}
                  color="cyan"
                  closable
                  onClose={(e) => {
                    e.preventDefault();
                    handleRemoveAlias(a.alias_id);
                  }}
                >
                  {a.alias_name}
                </Tag>
              ))}
              {aliases.length === 0 && !addingAlias && (
                <Text type="secondary" style={{ fontSize: 12 }}>No aliases</Text>
              )}
              {addingAlias ? (
                <Space size={4}>
                  <Input
                    size="small"
                    placeholder="Alias name"
                    value={newAliasName}
                    onChange={(e) => setNewAliasName(e.target.value)}
                    onPressEnter={handleAddAlias}
                    style={{ width: 180 }}
                    autoFocus
                  />
                  <Button size="small" type="primary" onClick={handleAddAlias}>
                    Add
                  </Button>
                  <Button size="small" onClick={() => { setAddingAlias(false); setNewAliasName(''); }}>
                    Cancel
                  </Button>
                </Space>
              ) : (
                <Tag
                  style={{ borderStyle: 'dashed', cursor: 'pointer' }}
                  onClick={() => setAddingAlias(true)}
                >
                  <PlusOutlined /> Add Alias
                </Tag>
              )}
            </Space>
          </div>
        </Card>

        {/* Section 7: Usage & Context */}
        <Card
          title="Usage & Context"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Form.Item name="business_context" label="Business Rules">
            <TextArea rows={3} placeholder="Describe the business rules and context" />
          </Form.Item>
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="used_in_reports" label="Used in Reports">
                <TextArea rows={2} placeholder="Reports where this term is used" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="used_in_policies" label="Used in Policies">
                <TextArea rows={2} placeholder="Policies referencing this term" />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item name="regulatory_reporting_usage" label="Regulatory Reporting Usage">
            <TextArea rows={2} placeholder="Context for regulatory reporting" />
          </Form.Item>
        </Card>

        {/* Section 7: Quality */}
        <Card
          title="Quality"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16} align="middle">
            <Col xs={24} md={6}>
              <Form.Item name="is_cbt" label="Critical Business Term" valuePropName="checked">
                <Switch checkedChildren="CBT" unCheckedChildren="No" />
              </Form.Item>
            </Col>
            <Col xs={24} md={10}>
              <Form.Item name="golden_source" label="Golden Source">
                <Input placeholder="Authoritative source system" />
              </Form.Item>
            </Col>
            {/* Confidence Level is managed by the Data Quality module, not edited here */}
          </Row>
        </Card>

        {/* Section 8: Discoverability */}
        <Card
          title="Discoverability"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="visibility_id" label="Visibility">
                <Select
                  placeholder="Select visibility"
                  options={visibilityOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="language_id" label="Language">
                <Select
                  placeholder="Select language"
                  options={languageOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>

          <Divider style={{ margin: '12px 0' }} />

          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="source_reference" label="Source Reference">
                <Input placeholder="e.g., ISO 8583, Basel III framework" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="regulatory_reference" label="Regulatory Reference">
                <Input placeholder="e.g., BCBS 239, GDPR Article 4" />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item name="external_reference" label="External Reference">
            <Input placeholder="External documentation URL or standard reference" />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 12 }}>
            Tags and keywords can be managed from the detail page after saving.
          </Text>
        </Card>

        {/* Submit */}
        <Card size="small">
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Button type="primary" htmlType="submit" loading={submitting} size="large">
                Update Term
              </Button>
              <Button onClick={() => navigate(`/glossary/${id}`)} size="large">
                Cancel
              </Button>
            </Space>
          </Form.Item>
        </Card>
      </Form>
    </div>
  );
};

export default GlossaryTermForm;

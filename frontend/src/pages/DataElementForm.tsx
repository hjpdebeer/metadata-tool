import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  Alert,
  Breadcrumb,
  Button,
  Card,
  Col,
  Form,
  Input,
  InputNumber,
  Row,
  Select,
  Space,
  Spin,
  Switch,
  Typography,
  message,
} from 'antd';
import { ArrowLeftOutlined, RobotOutlined } from '@ant-design/icons';
import { dataDictionaryApi } from '../services/dataDictionaryApi';
import { glossaryApi } from '../services/glossaryApi';
import { aiApi } from '../services/aiApi';
import { usersApi } from '../services/usersApi';
import type {
  CreateDataElementRequest,
  DataClassification,
  DataElementFullView,
  UpdateDataElementRequest,
} from '../services/dataDictionaryApi';
import type { GlossaryDomain, GlossaryReviewFrequency, GlossaryTermListItem } from '../services/glossaryApi';
import type { OrganisationalUnit } from '../services/glossaryApi';
import type { UserListItem } from '../services/usersApi';

const { Title, Text } = Typography;
const { TextArea } = Input;

const DATA_TYPES = [
  'VARCHAR',
  'INTEGER',
  'DECIMAL',
  'DATE',
  'TIMESTAMP',
  'BOOLEAN',
  'TEXT',
  'JSON',
  'UUID',
];

const DataElementForm: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const isEditing = Boolean(id);

  const [domains, setDomains] = useState<GlossaryDomain[]>([]);
  const [classifications, setClassifications] = useState<DataClassification[]>([]);
  const [glossaryTerms, setGlossaryTerms] = useState<GlossaryTermListItem[]>([]);
  const [reviewFrequencies, setReviewFrequencies] = useState<GlossaryReviewFrequency[]>([]);
  const [users, setUsers] = useState<UserListItem[]>([]);
  const [allOrgUnits, setAllOrgUnits] = useState<OrganisationalUnit[]>([]);
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [existingElement, setExistingElement] = useState<DataElementFullView | null>(null);

  const fetchReferenceData = useCallback(async () => {
    const results = await Promise.allSettled([
      glossaryApi.listDomains(),
      dataDictionaryApi.listClassifications(),
      glossaryApi.listTerms({ page_size: 500 }),
      glossaryApi.listReviewFrequencies(),
      usersApi.lookupUsers(),
      glossaryApi.listOrganisationalUnits(),
    ]);

    if (results[0].status === 'fulfilled') setDomains(results[0].value.data);
    if (results[1].status === 'fulfilled') setClassifications(results[1].value.data);
    if (results[2].status === 'fulfilled') {
      const termsData = results[2].value.data;
      if (Array.isArray(termsData)) {
        setGlossaryTerms(termsData);
      } else {
        const paginated = termsData as unknown as { data: GlossaryTermListItem[] };
        setGlossaryTerms(paginated.data);
      }
    }
    if (results[3].status === 'fulfilled') setReviewFrequencies(results[3].value.data);
    if (results[4].status === 'fulfilled') setUsers(results[4].value.data);
    if (results[5].status === 'fulfilled') setAllOrgUnits(results[5].value.data);
  }, []);

  const fetchExistingElement = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await dataDictionaryApi.getElement(id);
      setExistingElement(response.data);
      form.setFieldsValue({
        element_name: response.data.element_name,
        element_code: response.data.element_code,
        description: response.data.description,
        business_definition: response.data.business_definition || undefined,
        business_rules: response.data.business_rules || undefined,
        data_type: response.data.data_type,
        max_length: response.data.max_length ?? undefined,
        numeric_precision: response.data.numeric_precision ?? undefined,
        numeric_scale: response.data.numeric_scale ?? undefined,
        format_pattern: response.data.format_pattern || undefined,
        allowed_values: response.data.allowed_values
          ? typeof response.data.allowed_values === 'string'
            ? response.data.allowed_values
            : JSON.stringify(response.data.allowed_values, null, 2)
          : undefined,
        default_value: response.data.default_value || undefined,
        is_nullable: response.data.is_nullable,
        is_pii: response.data.is_pii,
        glossary_term_id: response.data.glossary_term_id || undefined,
        domain_id: response.data.domain_id || undefined,
        classification_id: response.data.classification_id || undefined,
        owner_user_id: response.data.owner_user_id || undefined,
        steward_user_id: response.data.steward_user_id || undefined,
        approver_user_id: response.data.approver_user_id || undefined,
        organisational_unit: response.data.organisational_unit || undefined,
        review_frequency_id: response.data.review_frequency_id || undefined,
      });
    } catch {
      message.error('Failed to load element for editing.');
      navigate('/data-dictionary');
    } finally {
      setLoading(false);
    }
  }, [id, form, navigate]);

  useEffect(() => {
    if (isEditing) {
      // Load lookups FIRST so Select options exist, THEN load element values.
      fetchReferenceData().then(() => fetchExistingElement());
    }
  }, [isEditing, fetchReferenceData, fetchExistingElement]);

  const handleCreateSubmit = async (values: { element_name: string; description: string }) => {
    setSubmitting(true);
    try {
      const cleanData: CreateDataElementRequest = {
        element_name: values.element_name.trim(),
        description: values.description.trim(),
      };

      const response = await dataDictionaryApi.createElement(cleanData);
      const newElementId = response.data.element_id;
      message.success('Element created. Generating AI suggestions...');

      // Trigger AI enrichment immediately after creation
      try {
        await aiApi.enrich('data_element', newElementId);
        message.success('AI suggestions ready for review.');
      } catch {
        message.info('Element created. AI enrichment unavailable — you can enrich later from the detail page.');
      }

      navigate(`/data-dictionary/${newElementId}`);
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { error?: { message?: string } }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.error?.message || 'Validation error.');
      } else {
        message.error('Failed to create element.');
      }
    } finally {
      setSubmitting(false);
    }
  };

  const handleEditSubmit = async (values: Record<string, unknown>) => {
    if (!id) return;
    setSubmitting(true);
    try {
      const updateData: UpdateDataElementRequest = {};

      // Build diff of changed fields
      const allFields = [
        'element_name', 'description',
        'business_definition', 'business_rules', 'data_type',
        'max_length', 'numeric_precision', 'numeric_scale',
        'format_pattern', 'allowed_values', 'default_value',
        'glossary_term_id', 'domain_id', 'classification_id',
        'owner_user_id', 'steward_user_id', 'approver_user_id',
        'organisational_unit', 'review_frequency_id',
      ] as const;

      for (const field of allFields) {
        const newVal = values[field];
        const oldVal = existingElement?.[field as keyof DataElementFullView];
        if (newVal !== oldVal) {
          (updateData as Record<string, unknown>)[field] = newVal || undefined;
        }
      }

      // Handle boolean fields separately
      if (values.is_nullable !== existingElement?.is_nullable) {
        updateData.is_nullable = values.is_nullable as boolean;
      }
      if (values.is_pii !== existingElement?.is_pii) {
        updateData.is_pii = values.is_pii as boolean;
      }

      const response = await dataDictionaryApi.updateElement(id, updateData);
      message.success('Element updated successfully.');
      navigate(`/data-dictionary/${response.data.element_id}`);
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { error?: { message?: string } }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.error?.message || 'Validation error.');
      } else {
        message.error('Failed to update element.');
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
  // CREATE MODE: Simplified form (name + code + description + data type)
  // =====================================================================
  if (!isEditing) {
    return (
      <div>
        <Breadcrumb
          style={{ marginBottom: 16 }}
          items={[
            { title: <a onClick={() => navigate('/data-dictionary')}>Data Dictionary</a> },
            { title: 'New Element' },
          ]}
        />

        <Space align="center" style={{ marginBottom: 16 }}>
          <Button type="text" icon={<ArrowLeftOutlined />} onClick={() => navigate('/data-dictionary')} />
          <Title level={3} style={{ margin: 0 }}>New Data Element</Title>
        </Space>

        <Card>
          <Alert
            message="AI-Assisted Creation"
            description="Enter the element name and description. After creation, AI will automatically suggest values for business definition, business rules, format pattern, classification, and domain based on financial services standards. You'll review and accept each suggestion."
            type="info"
            showIcon
            icon={<RobotOutlined />}
            style={{ marginBottom: 24 }}
          />

          <Form
            form={form}
            layout="vertical"
            onFinish={handleCreateSubmit}
            style={{ maxWidth: 800 }}
            scrollToFirstError
          >
            <Form.Item
              name="element_name"
              label="Element Name"
              rules={[
                { required: true, message: 'Element name is required' },
                { max: 512, message: 'Element name cannot exceed 512 characters' },
              ]}
            >
              <Input placeholder="e.g., Customer Account Balance, Transaction Date" size="large" />
            </Form.Item>

            <Form.Item
              name="description"
              label="Description"
              rules={[
                { required: true, message: 'Description is required' },
                { min: 10, message: 'Description should be at least 10 characters' },
              ]}
            >
              <TextArea
                rows={4}
                placeholder="Provide a clear, concise description of this data element. The AI will use this to suggest additional metadata."
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
                <Button onClick={() => navigate('/data-dictionary')} size="large">
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
  // EDIT MODE: Full form with all fields in grouped sections
  // =====================================================================
  const domainOptions = domains.map((d) => ({
    value: d.domain_id,
    label: d.domain_name,
  }));
  const classificationOptions = classifications.map((c) => ({
    value: c.classification_id,
    label: c.classification_name,
  }));
  const glossaryTermOptions = glossaryTerms.map((t) => ({
    value: t.term_id,
    label: t.term_name,
  }));
  const dataTypeOptions = DATA_TYPES.map((t) => ({
    value: t,
    label: t,
  }));
  const frequencyOptions = reviewFrequencies.map((f) => ({
    value: f.frequency_id,
    label: f.frequency_name,
  }));
  const userOptions = users.map((u) => ({
    value: u.user_id,
    label: `${u.display_name} (${u.email})`,
  }));

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/data-dictionary')}>Data Dictionary</a> },
          ...(existingElement
            ? [{ title: <a onClick={() => navigate(`/data-dictionary/${id}`)}>{existingElement.element_name}</a> }]
            : []),
          { title: 'Edit' },
        ]}
      />

      <Space align="center" style={{ marginBottom: 16 }}>
        <Button type="text" icon={<ArrowLeftOutlined />} onClick={() => navigate(`/data-dictionary/${id}`)} />
        <Title level={3} style={{ margin: 0 }}>Edit Element</Title>
        {existingElement?.element_code && (
          <Text type="secondary" code style={{ fontSize: 12 }}>
            {existingElement.element_code}
          </Text>
        )}
      </Space>

      <Form
        form={form}
        layout="vertical"
        onFinish={handleEditSubmit}
        scrollToFirstError
        initialValues={{ is_nullable: true, is_pii: false }}
      >
        {/* Section 1: Core Identity */}
        <Card
          title="Core Identity"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item
                name="element_name"
                label="Element Name"
                rules={[
                  { required: true, message: 'Element name is required' },
                  { max: 512, message: 'Element name cannot exceed 512 characters' },
                ]}
              >
                <Input placeholder="Enter the data element name" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="element_code" label="Element Code">
                <Input disabled placeholder="Auto-generated" />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item
            name="description"
            label="Description"
            rules={[
              { required: true, message: 'Description is required' },
              { min: 10, message: 'Description should be at least 10 characters' },
            ]}
          >
            <TextArea rows={4} placeholder="Provide a clear, concise description" />
          </Form.Item>
        </Card>

        {/* Section 2: Definition & Rules */}
        <Card
          title="Definition & Rules"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Form.Item name="business_definition" label="Business Definition">
            <TextArea rows={3} placeholder="Formal business definition of this data element" />
          </Form.Item>
          <Form.Item name="business_rules" label="Business Rules">
            <TextArea rows={3} placeholder="Business rules that apply to this element" />
          </Form.Item>
        </Card>

        {/* Section 3: Technical Specification */}
        <Card
          title="Technical Specification"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item
                name="data_type"
                label="Data Type"
                tooltip="Logical data type — precision and scale are defined at the technical column level"
              >
                <Select
                  placeholder="Select or AI-suggested"
                  options={dataTypeOptions}
                  showSearch
                  allowClear
                />
              </Form.Item>
            </Col>
            <Form.Item noStyle shouldUpdate={(prev, cur) => prev.data_type !== cur.data_type}>
              {({ getFieldValue }) => {
                const dt = (getFieldValue('data_type') || '').toUpperCase();
                const showLength = ['VARCHAR', 'CHAR', 'TEXT'].includes(dt);
                const showPrecision = ['DECIMAL', 'NUMERIC'].includes(dt);
                return (
                  <>
                    {showLength && (
                      <Col xs={24} md={8}>
                        <Form.Item name="max_length" label="Max Length">
                          <InputNumber min={1} style={{ width: '100%' }} placeholder="e.g., 256" />
                        </Form.Item>
                      </Col>
                    )}
                    {showPrecision && (
                      <>
                        <Col xs={24} md={4}>
                          <Form.Item name="numeric_precision" label="Precision">
                            <InputNumber min={1} style={{ width: '100%' }} placeholder="e.g., 18" />
                          </Form.Item>
                        </Col>
                        <Col xs={24} md={4}>
                          <Form.Item name="numeric_scale" label="Scale">
                            <InputNumber min={0} style={{ width: '100%' }} placeholder="e.g., 2" />
                          </Form.Item>
                        </Col>
                      </>
                    )}
                  </>
                );
              }}
            </Form.Item>
            <Col xs={24} md={8}>
              <Form.Item name="format_pattern" label="Format Pattern">
                <Input placeholder="e.g., YYYY-MM-DD, ###.##" />
              </Form.Item>
            </Col>
            <Col xs={24} md={4}>
              <Form.Item name="is_nullable" label="Nullable" valuePropName="checked">
                <Switch checkedChildren="Yes" unCheckedChildren="No" />
              </Form.Item>
            </Col>
            <Col xs={24} md={4}>
              <Form.Item name="is_pii" label="PII" valuePropName="checked">
                <Switch checkedChildren="Yes" unCheckedChildren="No" />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item name="allowed_values" label="Allowed Values">
            <TextArea rows={3} placeholder='e.g., ["ACTIVE", "INACTIVE", "CLOSED"]' />
          </Form.Item>
          <Form.Item name="default_value" label="Default Value">
            <Input placeholder="Default value for this element" style={{ maxWidth: 300 }} />
          </Form.Item>
        </Card>

        {/* Section 4: Classification */}
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
              <Form.Item name="classification_id" label="Classification">
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
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="glossary_term_id" label="Glossary Term">
                <Select
                  placeholder="Link to a glossary term"
                  options={glossaryTermOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* Section 5: Ownership */}
        <Card
          title="Ownership"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="owner_user_id" label="Data Owner">
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
            <Col xs={24} md={12}>
              <Form.Item name="organisational_unit" label="Organisational Unit">
                <Select
                  placeholder="Select organisational unit"
                  options={allOrgUnits.map((u) => ({ value: u.unit_name, label: u.unit_name }))}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* Section 6: Lifecycle */}
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

        {/* Submit */}
        <Card size="small">
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Button type="primary" htmlType="submit" loading={submitting} size="large">
                Update Element
              </Button>
              <Button onClick={() => navigate(`/data-dictionary/${id}`)} size="large">
                Cancel
              </Button>
            </Space>
          </Form.Item>
        </Card>
      </Form>
    </div>
  );
};

export default DataElementForm;

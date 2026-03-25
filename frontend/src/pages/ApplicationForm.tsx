import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  Alert,
  Breadcrumb,
  Button,
  Card,
  Col,
  DatePicker,
  Form,
  Input,
  Row,
  Select,
  Space,
  Spin,
  Switch,
  Typography,
  message,
} from 'antd';
import { ArrowLeftOutlined, RobotOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import { applicationsApi } from '../services/applicationsApi';
import { aiApi } from '../services/aiApi';
import { glossaryApi } from '../services/glossaryApi';
import { usersApi } from '../services/usersApi';
import type {
  ApplicationClassification,
  ApplicationCriticalityTier,
  ApplicationFullView,
  ApplicationLifecycleStage,
  ApplicationRiskRating,
  CreateApplicationRequest,
  DisasterRecoveryTier,
  UpdateApplicationRequest,
} from '../services/applicationsApi';
import type { DataClassificationRef, GlossaryReviewFrequency } from '../services/glossaryApi';
import type { UserListItem } from '../services/usersApi';

const { Title, Text } = Typography;
const { TextArea } = Input;

const DEPLOYMENT_TYPES = [
  { value: 'ON_PREMISE', label: 'On-Premise' },
  { value: 'CLOUD', label: 'Cloud' },
  { value: 'HYBRID', label: 'Hybrid' },
  { value: 'SAAS', label: 'SaaS' },
];

const ApplicationForm: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const isEditing = Boolean(id);

  // Reference data
  const [classifications, setClassifications] = useState<ApplicationClassification[]>([]);
  const [drTiers, setDrTiers] = useState<DisasterRecoveryTier[]>([]);
  const [lifecycleStages, setLifecycleStages] = useState<ApplicationLifecycleStage[]>([]);
  const [criticalityTiers, setCriticalityTiers] = useState<ApplicationCriticalityTier[]>([]);
  const [riskRatings, setRiskRatings] = useState<ApplicationRiskRating[]>([]);
  const [dataClassifications, setDataClassifications] = useState<DataClassificationRef[]>([]);
  const [reviewFrequencies, setReviewFrequencies] = useState<GlossaryReviewFrequency[]>([]);
  const [users, setUsers] = useState<UserListItem[]>([]);

  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [existingApp, setExistingApp] = useState<ApplicationFullView | null>(null);
  const [isCba, setIsCba] = useState(false);

  const fetchReferenceData = useCallback(async () => {
    const results = await Promise.allSettled([
      applicationsApi.listClassifications(),
      applicationsApi.listDrTiers(),
      applicationsApi.listLifecycleStages(),
      applicationsApi.listCriticalityTiers(),
      applicationsApi.listRiskRatings(),
      glossaryApi.listClassifications(),
      glossaryApi.listReviewFrequencies(),
      usersApi.lookupUsers(),
    ]);

    if (results[0].status === 'fulfilled') setClassifications(results[0].value.data);
    if (results[1].status === 'fulfilled') setDrTiers(results[1].value.data);
    if (results[2].status === 'fulfilled') setLifecycleStages(results[2].value.data);
    if (results[3].status === 'fulfilled') setCriticalityTiers(results[3].value.data);
    if (results[4].status === 'fulfilled') setRiskRatings(results[4].value.data);
    if (results[5].status === 'fulfilled') setDataClassifications(results[5].value.data);
    if (results[6].status === 'fulfilled') setReviewFrequencies(results[6].value.data);
    if (results[7].status === 'fulfilled') {
      setUsers(results[7].value.data);
    }
  }, []);

  const fetchExistingApp = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await applicationsApi.getApplication(id);
      setExistingApp(response.data);
      setIsCba(response.data.is_cba);
      form.setFieldsValue({
        application_name: response.data.application_name,
        application_code: response.data.application_code,
        description: response.data.description,
        classification_id: response.data.classification_id || undefined,
        vendor: response.data.vendor || undefined,
        vendor_product_name: response.data.vendor_product_name || undefined,
        version: response.data.version || undefined,
        deployment_type: response.data.deployment_type || undefined,
        technology_stack: response.data.technology_stack
          ? typeof response.data.technology_stack === 'string'
            ? response.data.technology_stack
            : JSON.stringify(response.data.technology_stack, null, 2)
          : undefined,
        is_cba: response.data.is_cba,
        cba_rationale: response.data.cba_rationale || undefined,
        go_live_date: response.data.go_live_date ? dayjs(response.data.go_live_date) : undefined,
        retirement_date: response.data.retirement_date ? dayjs(response.data.retirement_date) : undefined,
        contract_end_date: response.data.contract_end_date ? dayjs(response.data.contract_end_date) : undefined,
        documentation_url: response.data.documentation_url || undefined,
        abbreviation: response.data.abbreviation || undefined,
        external_reference_id: response.data.external_reference_id || undefined,
        license_type: response.data.license_type || undefined,
        lifecycle_stage_id: response.data.lifecycle_stage_id || undefined,
        business_capability: response.data.business_capability || undefined,
        user_base: response.data.user_base || undefined,
        criticality_tier_id: response.data.criticality_tier_id || undefined,
        risk_rating_id: response.data.risk_rating_id || undefined,
        data_classification_id: response.data.data_classification_id || undefined,
        regulatory_scope: response.data.regulatory_scope || undefined,
        last_security_assessment: response.data.last_security_assessment ? dayjs(response.data.last_security_assessment) : undefined,
        support_model: response.data.support_model || undefined,
        dr_tier_id: response.data.dr_tier_id || undefined,
        review_frequency_id: response.data.review_frequency_id || undefined,
        business_owner_id: response.data.business_owner_id || undefined,
        technical_owner_id: response.data.technical_owner_id || undefined,
        steward_user_id: response.data.steward_user_id || undefined,
        organisational_unit: response.data.organisational_unit || undefined,
      });
    } catch {
      message.error('Failed to load application for editing.');
      navigate('/applications');
    } finally {
      setLoading(false);
    }
  }, [id, form, navigate]);

  useEffect(() => {
    if (isEditing) {
      fetchReferenceData().then(() => fetchExistingApp());
    }
  }, [isEditing, fetchReferenceData, fetchExistingApp]);

  const handleCreateSubmit = async (values: { application_name: string; description: string }) => {
    setSubmitting(true);
    try {
      const cleanData: CreateApplicationRequest = {
        application_name: values.application_name.trim(),
        description: values.description.trim(),
      };

      const response = await applicationsApi.createApplication(cleanData);
      const newAppId = response.data.application_id;
      message.success('Application created. Generating AI suggestions...');

      // Trigger AI enrichment immediately after creation
      try {
        await aiApi.enrich('application', newAppId);
        message.success('AI suggestions ready for review.');
      } catch {
        message.info('Application created. AI enrichment unavailable — you can enrich later from the detail page.');
      }

      navigate(`/applications/${newAppId}`);
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { error?: { message?: string } }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.error?.message || 'Validation error. Please check the form.');
      } else {
        message.error('Failed to create application.');
      }
    } finally {
      setSubmitting(false);
    }
  };

  const handleSubmit = async (values: Record<string, unknown>) => {
    setSubmitting(true);
    try {
      // Convert dayjs to ISO strings
      const goLiveDate = values.go_live_date
        ? (values.go_live_date as dayjs.Dayjs).toISOString()
        : undefined;
      const retirementDate = values.retirement_date
        ? (values.retirement_date as dayjs.Dayjs).toISOString()
        : undefined;
      const contractEndDate = values.contract_end_date
        ? (values.contract_end_date as dayjs.Dayjs).format('YYYY-MM-DD')
        : undefined;
      const lastSecAssessment = values.last_security_assessment
        ? (values.last_security_assessment as dayjs.Dayjs).format('YYYY-MM-DD')
        : undefined;

      if (isEditing && id) {
        const updateData: UpdateApplicationRequest = {};
        const fields = [
          'application_name', 'description', 'classification_id',
          'vendor', 'vendor_product_name', 'version', 'deployment_type', 'technology_stack',
          'cba_rationale', 'documentation_url',
          'abbreviation', 'external_reference_id',
          'business_capability', 'user_base', 'license_type',
          'lifecycle_stage_id', 'criticality_tier_id', 'risk_rating_id',
          'data_classification_id', 'regulatory_scope',
          'support_model', 'dr_tier_id', 'review_frequency_id',
          'business_owner_id', 'technical_owner_id',
          'steward_user_id', 'organisational_unit',
        ] as const;

        for (const field of fields) {
          const newVal = values[field];
          const oldVal = existingApp?.[field as keyof ApplicationFullView];
          if (newVal !== oldVal) {
            (updateData as Record<string, unknown>)[field] = newVal || undefined;
          }
        }

        if (values.is_cba !== existingApp?.is_cba) {
          updateData.is_cba = values.is_cba as boolean;
        }

        // Handle date fields separately
        if (goLiveDate !== undefined) updateData.go_live_date = goLiveDate;
        if (retirementDate !== undefined) updateData.retirement_date = retirementDate;
        if (contractEndDate !== undefined) updateData.contract_end_date = contractEndDate;
        if (lastSecAssessment !== undefined) updateData.last_security_assessment = lastSecAssessment;

        const response = await applicationsApi.updateApplication(id, updateData);
        message.success('Application updated successfully.');
        navigate(`/applications/${response.data.application_id}`);
      } else {
        // Create mode is handled by handleCreateSubmit — this branch should not be reached
        return;
      }
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { message?: string }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.message || 'Validation error. Please check the form.');
      } else {
        message.error(isEditing ? 'Failed to update application.' : 'Failed to create application.');
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
  // CREATE MODE: Simplified form (name + description only)
  // =====================================================================
  if (!isEditing) {
    return (
      <div>
        <Breadcrumb
          style={{ marginBottom: 16 }}
          items={[
            { title: <a onClick={() => navigate('/applications')}>Applications</a> },
            { title: 'New Application' },
          ]}
        />

        <Space align="center" style={{ marginBottom: 16 }}>
          <Button type="text" icon={<ArrowLeftOutlined />} onClick={() => navigate('/applications')} />
          <Title level={3} style={{ margin: 0 }}>New Application</Title>
        </Space>

        <Card>
          <Alert
            message="AI-Assisted Creation"
            description="Enter the application name and description. After creation, AI will automatically suggest values for classification, vendor details, technology stack, criticality, compliance scope, and operational parameters based on financial services standards. You'll review and accept each suggestion."
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
              name="application_name"
              label="Application Name"
              rules={[
                { required: true, message: 'Application name is required' },
                { max: 256, message: 'Application name cannot exceed 256 characters' },
              ]}
            >
              <Input placeholder="e.g., Core Banking System, Anti-Money Laundering Platform" size="large" />
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
                placeholder="Provide a clear, concise description of this application. The AI will use this to suggest additional metadata."
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
                <Button onClick={() => navigate('/applications')} size="large">
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
  const classificationOptions = classifications.map((c) => ({
    value: c.classification_id,
    label: c.classification_name,
  }));
  const lifecycleOptions = lifecycleStages.map((s) => ({
    value: s.stage_id,
    label: s.stage_name,
  }));
  const criticalityOptions = criticalityTiers.map((t) => ({
    value: t.tier_id,
    label: t.tier_name,
  }));
  const riskRatingOptions = riskRatings.map((r) => ({
    value: r.rating_id,
    label: r.rating_name,
  }));
  const dataClassificationOptions = dataClassifications.map((c) => ({
    value: c.classification_id,
    label: c.classification_name,
  }));
  const drOptions = drTiers.map((d) => ({
    value: d.dr_tier_id,
    label: `${d.tier_name} (RTO: ${d.rto_hours}h / RPO: ${d.rpo_minutes}min)`,
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
          { title: <a onClick={() => navigate('/applications')}>Applications</a> },
          ...(isEditing && existingApp
            ? [
                {
                  title: (
                    <a onClick={() => navigate(`/applications/${id}`)}>
                      {existingApp.application_name}
                    </a>
                  ),
                },
              ]
            : []),
          { title: isEditing ? 'Edit' : 'New Application' },
        ]}
      />

      <Space align="center" style={{ marginBottom: 16 }}>
        <Button
          type="text"
          icon={<ArrowLeftOutlined />}
          onClick={() => {
            if (isEditing && id) {
              navigate(`/applications/${id}`);
            } else {
              navigate('/applications');
            }
          }}
        />
        <Title level={3} style={{ margin: 0 }}>
          {isEditing ? 'Edit Application' : 'New Application'}
        </Title>
      </Space>

      <Form
        form={form}
        layout="vertical"
        onFinish={handleSubmit}
        scrollToFirstError
        initialValues={{ is_cba: false }}
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
                name="application_name"
                label="Application Name"
                rules={[
                  { required: true, message: 'Application name is required' },
                  { max: 256, message: 'Application name cannot exceed 256 characters' },
                ]}
              >
                <Input placeholder="Enter the application name" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item
                name="abbreviation"
                label="Abbreviation"
                rules={[{ max: 50, message: 'Max 50 characters' }]}
              >
                <Input placeholder="e.g., CBS" />
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item
                name="external_reference_id"
                label="External Reference ID"
                rules={[{ max: 128, message: 'Max 128 characters' }]}
              >
                <Input placeholder="External system reference identifier" />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item
            name="description"
            label="Description"
            rules={[
              { required: true, message: 'Description is required' },
              {
                min: 10,
                message: 'Description should be at least 10 characters for clarity',
              },
            ]}
          >
            <TextArea
              rows={4}
              placeholder="Provide a clear, concise description of this application"
            />
          </Form.Item>
        </Card>

        {/* Section 2: Classification */}
        <Card
          title="Classification"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="classification_id" label="Classification">
                <Select
                  placeholder="Select a classification"
                  options={classificationOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="deployment_type" label="Deployment Type">
                <Select
                  placeholder="Select deployment type"
                  options={DEPLOYMENT_TYPES}
                  allowClear
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="lifecycle_stage_id" label="Lifecycle Stage">
                <Select
                  placeholder="Select lifecycle stage"
                  options={lifecycleOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item
            name="technology_stack"
            label="Technology Stack"
            tooltip="JSON describing the technology stack (e.g., languages, frameworks, databases)"
          >
            <TextArea
              rows={3}
              placeholder='e.g., {"language": "Java", "framework": "Spring Boot", "database": "Oracle"}'
            />
          </Form.Item>
        </Card>

        {/* Section 3: Vendor & Product */}
        <Card
          title="Vendor & Product"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="vendor" label="Vendor">
                <Input placeholder="e.g., Oracle, SAP, Internal" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="vendor_product_name" label="Vendor Product Name">
                <Input placeholder="e.g., Oracle E-Business Suite" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="version" label="Version">
                <Input placeholder="e.g., 12.2.0.1" />
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="license_type" label="License Type">
                <Input placeholder="e.g., Commercial, Open Source, Internal" />
              </Form.Item>
            </Col>
            <Col xs={24} md={16}>
              <Form.Item
                name="documentation_url"
                label="Documentation URL"
                rules={[{ type: 'url', message: 'Please enter a valid URL' }]}
              >
                <Input placeholder="https://..." />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* Section 4: Business Context */}
        <Card
          title="Business Context"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="business_capability" label="Business Capability">
                <Input placeholder="e.g., Transaction Processing, Risk Management" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="user_base" label="User Base">
                <Input placeholder="e.g., 500 internal users, All branches" />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* Section 5: Ownership */}
        {isEditing && (
          <Card
            title="Ownership"
            size="small"
            style={{ marginBottom: 16 }}
            headStyle={{ backgroundColor: '#F8FAFC' }}
          >
            <Row gutter={16}>
              <Col xs={24} md={12}>
                <Form.Item name="business_owner_id" label="Business Owner">
                  <Select
                    placeholder="Select business owner"
                    options={userOptions}
                    allowClear
                    showSearch
                    optionFilterProp="label"
                  />
                </Form.Item>
              </Col>
              <Col xs={24} md={12}>
                <Form.Item name="technical_owner_id" label="Technical Owner">
                  <Select
                    placeholder="Select technical owner"
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
                <Form.Item name="organisational_unit" label="Organisational Unit">
                  <Input placeholder="e.g., Group Risk, Retail Banking, Treasury" />
                </Form.Item>
              </Col>
            </Row>
          </Card>
        )}

        {/* Section 6: Criticality & Risk */}
        <Card
          title="Criticality & Risk"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16} align="middle">
            <Col xs={24} md={6}>
              <Form.Item
                name="is_cba"
                label="Critical Business Application"
                valuePropName="checked"
              >
                <Switch
                  checkedChildren="CBA"
                  unCheckedChildren="No"
                  onChange={(checked) => setIsCba(checked)}
                />
              </Form.Item>
            </Col>
            {isCba && (
              <Col xs={24} md={18}>
                <Form.Item
                  name="cba_rationale"
                  label="CBA Rationale"
                  tooltip="Explain why this application is classified as a Critical Business Application"
                  rules={[
                    { required: true, message: 'Please provide a rationale for CBA designation' },
                  ]}
                >
                  <TextArea
                    rows={2}
                    placeholder="e.g., Core transaction processing system - downtime impacts all customer-facing operations"
                  />
                </Form.Item>
              </Col>
            )}
          </Row>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="criticality_tier_id" label="Criticality Tier">
                <Select
                  placeholder="Select tier"
                  options={criticalityOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="risk_rating_id" label="Risk Rating">
                <Select
                  placeholder="Select rating"
                  options={riskRatingOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="data_classification_id" label="Data Classification">
                <Select
                  placeholder="Select classification"
                  options={dataClassificationOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="regulatory_scope" label="Regulatory Scope">
                <Input placeholder="e.g., BCBS 239, GDPR, PCI-DSS" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="last_security_assessment" label="Last Security Assessment">
                <DatePicker style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* Section 7: Operational */}
        <Card
          title="Operational"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="support_model" label="Support Model">
                <Input placeholder="e.g., 24x7, Business Hours, Best Effort" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="dr_tier_id" label="DR Tier">
                <Select
                  placeholder="Select DR tier"
                  options={drOptions}
                  allowClear
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </Col>
          </Row>
        </Card>

        {/* Section 8: Lifecycle */}
        <Card
          title="Lifecycle"
          size="small"
          style={{ marginBottom: 16 }}
          headStyle={{ backgroundColor: '#F8FAFC' }}
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="go_live_date" label="Go-Live Date">
                <DatePicker style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="retirement_date" label="Retirement Date">
                <DatePicker style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="contract_end_date" label="Contract End Date">
                <DatePicker style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>
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
                Update Application
              </Button>
              <Button
                onClick={() => navigate(`/applications/${id}`)}
                size="large"
              >
                Cancel
              </Button>
            </Space>
          </Form.Item>
        </Card>
      </Form>
    </div>
  );
};

export default ApplicationForm;

import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  Breadcrumb,
  Button,
  Card,
  DatePicker,
  Form,
  Input,
  Select,
  Space,
  Spin,
  Switch,
  Typography,
  message,
} from 'antd';
import { ArrowLeftOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import { applicationsApi } from '../services/applicationsApi';
import type {
  ApplicationClassification,
  ApplicationFullView,
  CreateApplicationRequest,
  UpdateApplicationRequest,
} from '../services/applicationsApi';

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

  const [classifications, setClassifications] = useState<ApplicationClassification[]>([]);
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [existingApp, setExistingApp] = useState<ApplicationFullView | null>(null);
  const [isCritical, setIsCritical] = useState(false);

  const fetchReferenceData = useCallback(async () => {
    try {
      const response = await applicationsApi.listClassifications();
      setClassifications(response.data);
    } catch {
      // Non-critical; select will just be empty
    }
  }, []);

  const fetchExistingApp = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await applicationsApi.getApplication(id);
      setExistingApp(response.data);
      setIsCritical(response.data.is_critical);
      form.setFieldsValue({
        application_name: response.data.application_name,
        application_code: response.data.application_code,
        description: response.data.description,
        classification_id: response.data.classification_id || undefined,
        vendor: response.data.vendor || undefined,
        version: response.data.version || undefined,
        deployment_type: response.data.deployment_type || undefined,
        technology_stack: response.data.technology_stack
          ? typeof response.data.technology_stack === 'string'
            ? response.data.technology_stack
            : JSON.stringify(response.data.technology_stack, null, 2)
          : undefined,
        is_critical: response.data.is_critical,
        criticality_rationale: response.data.criticality_rationale || undefined,
        go_live_date: response.data.go_live_date ? dayjs(response.data.go_live_date) : undefined,
        documentation_url: response.data.documentation_url || undefined,
      });
    } catch {
      message.error('Failed to load application for editing.');
      navigate('/applications');
    } finally {
      setLoading(false);
    }
  }, [id, form, navigate]);

  useEffect(() => {
    fetchReferenceData();
  }, [fetchReferenceData]);

  useEffect(() => {
    if (isEditing) {
      fetchExistingApp();
    }
  }, [isEditing, fetchExistingApp]);

  const handleSubmit = async (values: CreateApplicationRequest & { is_critical: boolean; go_live_date?: dayjs.Dayjs }) => {
    setSubmitting(true);
    try {
      // Convert dayjs to ISO string
      const goLiveDate = values.go_live_date
        ? (values.go_live_date as unknown as dayjs.Dayjs).toISOString()
        : undefined;

      if (isEditing && id) {
        const updateData: UpdateApplicationRequest = {};
        const fields: (keyof CreateApplicationRequest)[] = [
          'application_name',
          'description',
          'classification_id',
          'vendor',
          'version',
          'deployment_type',
          'technology_stack',
          'criticality_rationale',
          'documentation_url',
        ];

        for (const field of fields) {
          const newVal = values[field];
          const oldVal = existingApp?.[field as keyof ApplicationFullView];
          if (newVal !== oldVal) {
            (updateData as Record<string, unknown>)[field] = newVal || undefined;
          }
        }

        if (values.is_critical !== existingApp?.is_critical) {
          updateData.is_critical = values.is_critical;
        }

        const response = await applicationsApi.updateApplication(id, updateData);
        message.success('Application updated successfully.');
        navigate(`/applications/${response.data.application_id}`);
      } else {
        const cleanData: CreateApplicationRequest = {
          application_name: values.application_name,
          application_code: values.application_code,
          description: values.description,
          classification_id: values.classification_id || undefined,
          vendor: values.vendor || undefined,
          version: values.version || undefined,
          deployment_type: values.deployment_type || undefined,
          technology_stack: values.technology_stack || undefined,
          is_critical: values.is_critical,
          criticality_rationale: values.criticality_rationale || undefined,
          go_live_date: goLiveDate,
          documentation_url: values.documentation_url || undefined,
        };

        const response = await applicationsApi.createApplication(cleanData);
        message.success('Application created successfully.');
        navigate(`/applications/${response.data.application_id}`);
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

  const classificationOptions = classifications.map((c) => ({
    value: c.classification_id,
    label: c.classification_name,
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

      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmit}
          style={{ maxWidth: 800 }}
          scrollToFirstError
          initialValues={{ is_critical: false }}
        >
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

          <Form.Item
            name="application_code"
            label="Application Code"
            rules={[
              { required: true, message: 'Application code is required' },
              { max: 64, message: 'Application code cannot exceed 64 characters' },
              {
                pattern: /^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*$/,
                message: 'Application code must be UPPER_SNAKE_CASE (e.g., CORE_BANKING_SYS)',
              },
            ]}
            tooltip="Must be UPPER_SNAKE_CASE (e.g., CORE_BANKING_SYS)"
          >
            <Input placeholder="e.g., CORE_BANKING_SYS" disabled={isEditing} />
          </Form.Item>

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

          <Form.Item name="classification_id" label="Classification">
            <Select
              placeholder="Select a classification"
              options={classificationOptions}
              allowClear
              showSearch
              optionFilterProp="label"
            />
          </Form.Item>

          <Form.Item name="vendor" label="Vendor">
            <Input placeholder="e.g., Oracle, SAP, Internal" />
          </Form.Item>

          <Form.Item name="version" label="Version">
            <Input placeholder="e.g., 12.2.0.1" style={{ maxWidth: 300 }} />
          </Form.Item>

          <Form.Item name="deployment_type" label="Deployment Type">
            <Select
              placeholder="Select deployment type"
              options={DEPLOYMENT_TYPES}
              allowClear
            />
          </Form.Item>

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

          <Form.Item
            name="is_critical"
            label="Critical Application"
            valuePropName="checked"
          >
            <Switch
              checkedChildren="Yes"
              unCheckedChildren="No"
              onChange={(checked) => setIsCritical(checked)}
            />
          </Form.Item>

          {isCritical && (
            <Form.Item
              name="criticality_rationale"
              label="Criticality Rationale"
              tooltip="Explain why this application is classified as critical"
              rules={[
                { required: true, message: 'Please provide a rationale for critical designation' },
              ]}
            >
              <TextArea
                rows={3}
                placeholder="e.g., Core transaction processing system - downtime impacts all customer-facing operations"
              />
            </Form.Item>
          )}

          <Form.Item
            name="go_live_date"
            label="Go-Live Date"
          >
            <DatePicker style={{ width: '100%', maxWidth: 300 }} />
          </Form.Item>

          <Form.Item
            name="documentation_url"
            label="Documentation URL"
            rules={[
              {
                type: 'url',
                message: 'Please enter a valid URL',
              },
            ]}
          >
            <Input placeholder="https://..." />
          </Form.Item>

          <Form.Item style={{ marginTop: 24 }}>
            <Space>
              <Button type="primary" htmlType="submit" loading={submitting}>
                {isEditing ? 'Update Application' : 'Create Application'}
              </Button>
              <Button
                onClick={() => {
                  if (isEditing && id) {
                    navigate(`/applications/${id}`);
                  } else {
                    navigate('/applications');
                  }
                }}
              >
                Cancel
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>

      {isCritical && !isEditing && (
        <div style={{ marginTop: 16 }}>
          <Text type="warning" style={{ fontSize: 13 }}>
            Note: Critical applications receive enhanced monitoring and governance requirements.
          </Text>
        </div>
      )}
    </div>
  );
};

export default ApplicationForm;

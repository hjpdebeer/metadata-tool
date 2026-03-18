import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  Breadcrumb,
  Button,
  Card,
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
import { dataDictionaryApi } from '../services/dataDictionaryApi';
import { glossaryApi } from '../services/glossaryApi';
import type {
  CreateDataElementRequest,
  DataClassification,
  DataElementFullView,
  UpdateDataElementRequest,
} from '../services/dataDictionaryApi';
import type { GlossaryDomain, GlossaryTermListItem } from '../services/glossaryApi';

const { Title } = Typography;
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

const SENSITIVITY_LEVELS = [
  { value: 'PUBLIC', label: 'Public' },
  { value: 'INTERNAL', label: 'Internal' },
  { value: 'CONFIDENTIAL', label: 'Confidential' },
  { value: 'RESTRICTED', label: 'Restricted' },
];

const DataElementForm: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const isEditing = Boolean(id);

  const [domains, setDomains] = useState<GlossaryDomain[]>([]);
  const [classifications, setClassifications] = useState<DataClassification[]>([]);
  const [glossaryTerms, setGlossaryTerms] = useState<GlossaryTermListItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [existingElement, setExistingElement] = useState<DataElementFullView | null>(null);

  const fetchReferenceData = useCallback(async () => {
    try {
      const [domainsRes, classificationsRes, termsRes] = await Promise.allSettled([
        glossaryApi.listDomains(),
        dataDictionaryApi.listClassifications(),
        glossaryApi.listTerms({ page_size: 500 }),
      ]);

      if (domainsRes.status === 'fulfilled') {
        setDomains(domainsRes.value.data);
      }
      if (classificationsRes.status === 'fulfilled') {
        setClassifications(classificationsRes.value.data);
      }
      if (termsRes.status === 'fulfilled') {
        const termsData = termsRes.value.data;
        if (Array.isArray(termsData)) {
          setGlossaryTerms(termsData);
        } else {
          const paginated = termsData as unknown as { data: GlossaryTermListItem[] };
          setGlossaryTerms(paginated.data);
        }
      }
    } catch {
      // Non-critical; selects will just be empty
    }
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
        format_pattern: response.data.format_pattern || undefined,
        allowed_values: response.data.allowed_values
          ? typeof response.data.allowed_values === 'string'
            ? response.data.allowed_values
            : JSON.stringify(response.data.allowed_values, null, 2)
          : undefined,
        default_value: response.data.default_value || undefined,
        is_nullable: response.data.is_nullable,
        glossary_term_id: response.data.glossary_term_id || undefined,
        domain_id: response.data.domain_id || undefined,
        classification_id: response.data.classification_id || undefined,
        sensitivity_level: response.data.sensitivity_level || undefined,
      });
    } catch {
      message.error('Failed to load element for editing.');
      navigate('/data-dictionary');
    } finally {
      setLoading(false);
    }
  }, [id, form, navigate]);

  useEffect(() => {
    fetchReferenceData();
  }, [fetchReferenceData]);

  useEffect(() => {
    if (isEditing) {
      fetchExistingElement();
    }
  }, [isEditing, fetchExistingElement]);

  const handleSubmit = async (values: CreateDataElementRequest & { is_nullable: boolean }) => {
    setSubmitting(true);
    try {
      if (isEditing && id) {
        // For update, only send changed fields
        const updateData: UpdateDataElementRequest = {};
        const fields: (keyof CreateDataElementRequest)[] = [
          'element_name',
          'element_code',
          'description',
          'business_definition',
          'business_rules',
          'data_type',
          'format_pattern',
          'allowed_values',
          'default_value',
          'glossary_term_id',
          'domain_id',
          'classification_id',
          'sensitivity_level',
        ];

        for (const field of fields) {
          const newVal = values[field];
          const oldVal = existingElement?.[field as keyof DataElementFullView];
          if (newVal !== oldVal) {
            (updateData as Record<string, unknown>)[field] = newVal || undefined;
          }
        }

        // Always send is_nullable since it's a boolean
        if (values.is_nullable !== existingElement?.is_nullable) {
          updateData.is_nullable = values.is_nullable;
        }

        const response = await dataDictionaryApi.updateElement(id, updateData);
        message.success('Element updated successfully.');
        navigate(`/data-dictionary/${response.data.element_id}`);
      } else {
        // Clean up empty strings to undefined
        const cleanData: CreateDataElementRequest = {
          element_name: values.element_name,
          element_code: values.element_code,
          description: values.description,
          business_definition: values.business_definition || undefined,
          business_rules: values.business_rules || undefined,
          data_type: values.data_type,
          format_pattern: values.format_pattern || undefined,
          allowed_values: values.allowed_values || undefined,
          default_value: values.default_value || undefined,
          is_nullable: values.is_nullable,
          glossary_term_id: values.glossary_term_id || undefined,
          domain_id: values.domain_id || undefined,
          classification_id: values.classification_id || undefined,
          sensitivity_level: values.sensitivity_level || undefined,
        };

        const response = await dataDictionaryApi.createElement(cleanData);
        message.success('Element created successfully.');
        navigate(`/data-dictionary/${response.data.element_id}`);
      }
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { message?: string }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.message || 'Validation error. Please check the form.');
      } else {
        message.error(isEditing ? 'Failed to update element.' : 'Failed to create element.');
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

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/data-dictionary')}>Data Dictionary</a> },
          ...(isEditing && existingElement
            ? [
                {
                  title: (
                    <a onClick={() => navigate(`/data-dictionary/${id}`)}>
                      {existingElement.element_name}
                    </a>
                  ),
                },
              ]
            : []),
          { title: isEditing ? 'Edit' : 'New Element' },
        ]}
      />

      <Space align="center" style={{ marginBottom: 16 }}>
        <Button
          type="text"
          icon={<ArrowLeftOutlined />}
          onClick={() => {
            if (isEditing && id) {
              navigate(`/data-dictionary/${id}`);
            } else {
              navigate('/data-dictionary');
            }
          }}
        />
        <Title level={3} style={{ margin: 0 }}>
          {isEditing ? 'Edit Element' : 'New Data Element'}
        </Title>
      </Space>

      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmit}
          style={{ maxWidth: 800 }}
          scrollToFirstError
          initialValues={{ is_nullable: true }}
        >
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

          <Form.Item
            name="element_code"
            label="Element Code"
            rules={[
              { required: true, message: 'Element code is required' },
              { max: 256, message: 'Element code cannot exceed 256 characters' },
              {
                pattern: /^[a-z][a-z0-9]*(_[a-z0-9]+)*$/,
                message: 'Element code must be snake_case (e.g., customer_account_balance)',
              },
            ]}
            tooltip="Must be snake_case (e.g., customer_account_balance)"
          >
            <Input placeholder="e.g., customer_account_balance" />
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
              placeholder="Provide a clear, concise description of this data element"
            />
          </Form.Item>

          <Form.Item
            name="business_definition"
            label="Business Definition"
            tooltip="The business meaning and context for this element"
          >
            <TextArea
              rows={3}
              placeholder="Formal business definition of this data element"
            />
          </Form.Item>

          <Form.Item
            name="business_rules"
            label="Business Rules"
            tooltip="Rules governing how this element should be used, validated, or transformed"
          >
            <TextArea
              rows={3}
              placeholder="Business rules that apply to this element"
            />
          </Form.Item>

          <Form.Item
            name="data_type"
            label="Data Type"
            rules={[{ required: true, message: 'Data type is required' }]}
          >
            <Select
              placeholder="Select data type"
              options={dataTypeOptions}
              showSearch
            />
          </Form.Item>

          <Form.Item
            name="format_pattern"
            label="Format Pattern"
            tooltip="Expected format pattern (e.g., YYYY-MM-DD, ###.##)"
          >
            <Input placeholder="e.g., YYYY-MM-DD, ###.##, [A-Z]{3}" />
          </Form.Item>

          <Form.Item
            name="allowed_values"
            label="Allowed Values"
            tooltip="JSON array of allowed values (e.g., [&quot;ACTIVE&quot;, &quot;INACTIVE&quot;, &quot;CLOSED&quot;])"
          >
            <TextArea
              rows={3}
              placeholder='e.g., ["ACTIVE", "INACTIVE", "CLOSED"]'
            />
          </Form.Item>

          <Form.Item
            name="default_value"
            label="Default Value"
          >
            <Input
              placeholder="Default value for this element"
              style={{ maxWidth: 300 }}
            />
          </Form.Item>

          <Form.Item
            name="is_nullable"
            label="Nullable"
            valuePropName="checked"
          >
            <Switch checkedChildren="Yes" unCheckedChildren="No" />
          </Form.Item>

          <Form.Item
            name="glossary_term_id"
            label="Glossary Term"
            tooltip="Link this element to a business glossary term"
          >
            <Select
              placeholder="Select a glossary term"
              options={glossaryTermOptions}
              allowClear
              showSearch
              optionFilterProp="label"
            />
          </Form.Item>

          <Form.Item name="domain_id" label="Domain">
            <Select
              placeholder="Select a domain"
              options={domainOptions}
              allowClear
              showSearch
              optionFilterProp="label"
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

          <Form.Item name="sensitivity_level" label="Sensitivity Level">
            <Select
              placeholder="Select sensitivity level"
              options={SENSITIVITY_LEVELS}
              allowClear
            />
          </Form.Item>

          <Form.Item style={{ marginTop: 24 }}>
            <Space>
              <Button type="primary" htmlType="submit" loading={submitting}>
                {isEditing ? 'Update Element' : 'Create Element'}
              </Button>
              <Button
                onClick={() => {
                  if (isEditing && id) {
                    navigate(`/data-dictionary/${id}`);
                  } else {
                    navigate('/data-dictionary');
                  }
                }}
              >
                Cancel
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>
    </div>
  );
};

export default DataElementForm;

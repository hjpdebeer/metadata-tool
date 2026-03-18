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
  Typography,
  message,
} from 'antd';
import { ArrowLeftOutlined } from '@ant-design/icons';
import { glossaryApi } from '../services/glossaryApi';
import type {
  CreateGlossaryTermRequest,
  GlossaryCategory,
  GlossaryDomain,
  GlossaryTerm,
  UpdateGlossaryTermRequest,
} from '../services/glossaryApi';

const { Title } = Typography;
const { TextArea } = Input;

const GlossaryTermForm: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const isEditing = Boolean(id);

  const [domains, setDomains] = useState<GlossaryDomain[]>([]);
  const [categories, setCategories] = useState<GlossaryCategory[]>([]);
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [existingTerm, setExistingTerm] = useState<GlossaryTerm | null>(null);

  const fetchReferenceData = useCallback(async () => {
    try {
      const [domainsRes, categoriesRes] = await Promise.allSettled([
        glossaryApi.listDomains(),
        glossaryApi.listCategories(),
      ]);

      if (domainsRes.status === 'fulfilled') {
        setDomains(domainsRes.value.data);
      }
      if (categoriesRes.status === 'fulfilled') {
        setCategories(categoriesRes.value.data);
      }
    } catch {
      // Non-critical; selects will just be empty
    }
  }, []);

  const fetchExistingTerm = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await glossaryApi.getTerm(id);
      setExistingTerm(response.data);
      form.setFieldsValue({
        term_name: response.data.term_name,
        definition: response.data.definition,
        business_context: response.data.business_context || undefined,
        examples: response.data.examples || undefined,
        abbreviation: response.data.abbreviation || undefined,
        domain_id: response.data.domain_id || undefined,
        category_id: response.data.category_id || undefined,
        source_reference: response.data.source_reference || undefined,
        regulatory_reference: response.data.regulatory_reference || undefined,
      });
    } catch {
      message.error('Failed to load term for editing.');
      navigate('/glossary');
    } finally {
      setLoading(false);
    }
  }, [id, form, navigate]);

  useEffect(() => {
    fetchReferenceData();
  }, [fetchReferenceData]);

  useEffect(() => {
    if (isEditing) {
      fetchExistingTerm();
    }
  }, [isEditing, fetchExistingTerm]);

  const handleSubmit = async (values: CreateGlossaryTermRequest) => {
    setSubmitting(true);
    try {
      if (isEditing && id) {
        // For update, only send changed fields
        const updateData: UpdateGlossaryTermRequest = {};
        const fields: (keyof CreateGlossaryTermRequest)[] = [
          'term_name',
          'definition',
          'business_context',
          'examples',
          'abbreviation',
          'domain_id',
          'category_id',
          'source_reference',
          'regulatory_reference',
        ];

        for (const field of fields) {
          const newVal = values[field];
          const oldVal = existingTerm?.[field as keyof GlossaryTerm];
          // Send the field if it changed (including clearing it)
          if (newVal !== oldVal) {
            (updateData as Record<string, unknown>)[field] = newVal || undefined;
          }
        }

        const response = await glossaryApi.updateTerm(id, updateData);
        message.success('Term updated successfully.');
        navigate(`/glossary/${response.data.term_id}`);
      } else {
        // Clean up empty strings to undefined
        const cleanData: CreateGlossaryTermRequest = {
          term_name: values.term_name,
          definition: values.definition,
          business_context: values.business_context || undefined,
          examples: values.examples || undefined,
          abbreviation: values.abbreviation || undefined,
          domain_id: values.domain_id || undefined,
          category_id: values.category_id || undefined,
          source_reference: values.source_reference || undefined,
          regulatory_reference: values.regulatory_reference || undefined,
        };

        const response = await glossaryApi.createTerm(cleanData);
        message.success('Term created successfully.');
        navigate(`/glossary/${response.data.term_id}`);
      }
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { message?: string }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.message || 'Validation error. Please check the form.');
      } else {
        message.error(isEditing ? 'Failed to update term.' : 'Failed to create term.');
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

  const categoryOptions = categories.map((c) => ({
    value: c.category_id,
    label: c.category_name,
  }));

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/glossary')}>Business Glossary</a> },
          ...(isEditing && existingTerm
            ? [
                {
                  title: (
                    <a onClick={() => navigate(`/glossary/${id}`)}>
                      {existingTerm.term_name}
                    </a>
                  ),
                },
              ]
            : []),
          { title: isEditing ? 'Edit' : 'New Term' },
        ]}
      />

      <Space align="center" style={{ marginBottom: 16 }}>
        <Button
          type="text"
          icon={<ArrowLeftOutlined />}
          onClick={() => {
            if (isEditing && id) {
              navigate(`/glossary/${id}`);
            } else {
              navigate('/glossary');
            }
          }}
        />
        <Title level={3} style={{ margin: 0 }}>
          {isEditing ? 'Edit Term' : 'New Glossary Term'}
        </Title>
      </Space>

      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmit}
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
            <Input placeholder="Enter the business term name" />
          </Form.Item>

          <Form.Item
            name="definition"
            label="Definition"
            rules={[
              { required: true, message: 'Definition is required' },
              {
                min: 10,
                message: 'Definition should be at least 10 characters for clarity',
              },
            ]}
          >
            <TextArea
              rows={4}
              placeholder="Provide a clear, concise definition of this business term"
            />
          </Form.Item>

          <Form.Item
            name="business_context"
            label="Business Context"
            tooltip="Describe how this term is used in business operations"
          >
            <TextArea
              rows={3}
              placeholder="Describe the business context in which this term is used"
            />
          </Form.Item>

          <Form.Item
            name="examples"
            label="Examples"
            tooltip="Provide concrete examples to aid understanding"
          >
            <TextArea
              rows={3}
              placeholder="Provide examples of how this term is used"
            />
          </Form.Item>

          <Form.Item
            name="abbreviation"
            label="Abbreviation"
            rules={[{ max: 50, message: 'Abbreviation cannot exceed 50 characters' }]}
          >
            <Input
              placeholder="Common abbreviation (if any)"
              style={{ maxWidth: 300 }}
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

          <Form.Item name="category_id" label="Category">
            <Select
              placeholder="Select a category"
              options={categoryOptions}
              allowClear
              showSearch
              optionFilterProp="label"
            />
          </Form.Item>

          <Form.Item
            name="source_reference"
            label="Source Reference"
            tooltip="Reference to the authoritative source for this term"
          >
            <Input placeholder="e.g., ISO 8583, Basel III framework" />
          </Form.Item>

          <Form.Item
            name="regulatory_reference"
            label="Regulatory Reference"
            tooltip="Reference to relevant regulatory requirements"
          >
            <Input placeholder="e.g., BCBS 239, GDPR Article 4" />
          </Form.Item>

          <Form.Item style={{ marginTop: 24 }}>
            <Space>
              <Button type="primary" htmlType="submit" loading={submitting}>
                {isEditing ? 'Update Term' : 'Create Term'}
              </Button>
              <Button
                onClick={() => {
                  if (isEditing && id) {
                    navigate(`/glossary/${id}`);
                  } else {
                    navigate('/glossary');
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

export default GlossaryTermForm;

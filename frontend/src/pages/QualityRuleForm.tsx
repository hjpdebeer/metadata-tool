import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  Breadcrumb,
  Button,
  Card,
  Form,
  Input,
  InputNumber,
  Select,
  Space,
  Spin,
  Switch,
  Typography,
  message,
} from 'antd';
import { ArrowLeftOutlined } from '@ant-design/icons';
import { dataQualityApi } from '../services/dataQualityApi';
import { dataDictionaryApi } from '../services/dataDictionaryApi';
import type {
  CreateQualityRuleRequest,
  QualityDimensionSummary,
  QualityRule,
  QualityRuleType,
  UpdateQualityRuleRequest,
} from '../services/dataQualityApi';
import type { DataElementListItem } from '../services/dataDictionaryApi';

const { Title } = Typography;
const { TextArea } = Input;

const SEVERITY_OPTIONS = [
  { value: 'LOW', label: 'Low' },
  { value: 'MEDIUM', label: 'Medium' },
  { value: 'HIGH', label: 'High' },
  { value: 'CRITICAL', label: 'Critical' },
];

const QualityRuleForm: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const isEditing = Boolean(id);

  const [dimensions, setDimensions] = useState<QualityDimensionSummary[]>([]);
  const [ruleTypes, setRuleTypes] = useState<QualityRuleType[]>([]);
  const [elements, setElements] = useState<DataElementListItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [existingRule, setExistingRule] = useState<QualityRule | null>(null);

  const fetchReferenceData = useCallback(async () => {
    try {
      const [dimensionsRes, ruleTypesRes, elementsRes] = await Promise.allSettled([
        dataQualityApi.listDimensions(),
        dataQualityApi.listRuleTypes(),
        dataDictionaryApi.listElements({ page_size: 500 }),
      ]);

      if (dimensionsRes.status === 'fulfilled') {
        setDimensions(dimensionsRes.value.data);
      }
      if (ruleTypesRes.status === 'fulfilled') {
        setRuleTypes(ruleTypesRes.value.data);
      }
      if (elementsRes.status === 'fulfilled') {
        const elemData = elementsRes.value.data;
        if (Array.isArray(elemData)) {
          setElements(elemData);
        } else {
          const paginated = elemData as unknown as { data: DataElementListItem[] };
          setElements(paginated.data);
        }
      }
    } catch {
      // Non-critical; selects will just be empty
    }
  }, []);

  const fetchExistingRule = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await dataQualityApi.getRule(id);
      setExistingRule(response.data);

      let ruleDefStr: string;
      try {
        ruleDefStr =
          typeof response.data.rule_definition === 'string'
            ? response.data.rule_definition
            : JSON.stringify(response.data.rule_definition, null, 2);
      } catch {
        ruleDefStr = String(response.data.rule_definition);
      }

      form.setFieldsValue({
        rule_name: response.data.rule_name,
        rule_code: response.data.rule_code,
        description: response.data.description,
        dimension_id: response.data.dimension_id,
        rule_type_id: response.data.rule_type_id,
        element_id: response.data.element_id || undefined,
        rule_definition: ruleDefStr,
        threshold_percentage: response.data.threshold_percentage,
        severity: response.data.severity,
        is_active: response.data.is_active,
      });
    } catch {
      message.error('Failed to load rule for editing.');
      navigate('/data-quality/rules');
    } finally {
      setLoading(false);
    }
  }, [id, form, navigate]);

  useEffect(() => {
    fetchReferenceData();
  }, [fetchReferenceData]);

  useEffect(() => {
    if (isEditing) {
      fetchExistingRule();
    }
  }, [isEditing, fetchExistingRule]);

  const handleSubmit = async (values: Record<string, unknown>) => {
    // Validate and parse rule_definition JSON
    let parsedDefinition: Record<string, unknown>;
    try {
      parsedDefinition = JSON.parse(values.rule_definition as string);
    } catch {
      message.error('Rule Definition must be valid JSON.');
      return;
    }

    setSubmitting(true);
    try {
      if (isEditing && id) {
        const updateData: UpdateQualityRuleRequest = {};
        const existingData = existingRule as Record<string, unknown> | null;

        if (values.rule_name !== existingData?.rule_name) {
          updateData.rule_name = values.rule_name as string;
        }
        if (values.rule_code !== existingData?.rule_code) {
          updateData.rule_code = values.rule_code as string;
        }
        if (values.description !== existingData?.description) {
          updateData.description = values.description as string;
        }
        if (values.dimension_id !== existingData?.dimension_id) {
          updateData.dimension_id = values.dimension_id as string;
        }
        if (values.rule_type_id !== existingData?.rule_type_id) {
          updateData.rule_type_id = values.rule_type_id as string;
        }
        if (values.element_id !== existingData?.element_id) {
          updateData.element_id = (values.element_id as string) || undefined;
        }
        if (values.threshold_percentage !== existingData?.threshold_percentage) {
          updateData.threshold_percentage = values.threshold_percentage as number;
        }
        if (values.severity !== existingData?.severity) {
          updateData.severity = values.severity as string;
        }
        if (values.is_active !== existingData?.is_active) {
          updateData.is_active = values.is_active as boolean;
        }
        // Always include rule_definition as it may have changed
        updateData.rule_definition = parsedDefinition;

        const response = await dataQualityApi.updateRule(id, updateData);
        message.success('Rule updated successfully.');
        navigate(`/data-quality/rules/${response.data.rule_id}`);
      } else {
        const cleanData: CreateQualityRuleRequest = {
          rule_name: values.rule_name as string,
          rule_code: values.rule_code as string,
          description: values.description as string,
          dimension_id: values.dimension_id as string,
          rule_type_id: values.rule_type_id as string,
          element_id: (values.element_id as string) || undefined,
          rule_definition: parsedDefinition,
          threshold_percentage: values.threshold_percentage as number,
          severity: values.severity as string,
          is_active: values.is_active as boolean,
        };

        const response = await dataQualityApi.createRule(cleanData);
        message.success('Rule created successfully.');
        navigate(`/data-quality/rules/${response.data.rule_id}`);
      }
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { message?: string }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.message || 'Validation error. Please check the form.');
      } else {
        message.error(isEditing ? 'Failed to update rule.' : 'Failed to create rule.');
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

  const dimensionOptions = dimensions.map((d) => ({
    value: d.dimension_id,
    label: d.dimension_name,
  }));

  const ruleTypeOptions = ruleTypes.map((t) => ({
    value: t.rule_type_id,
    label: t.type_name,
  }));

  const elementOptions = elements.map((e) => ({
    value: e.element_id,
    label: e.element_name,
  }));

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/data-quality')}>Data Quality</a> },
          { title: <a onClick={() => navigate('/data-quality/rules')}>Quality Rules</a> },
          ...(isEditing && existingRule
            ? [
                {
                  title: (
                    <a onClick={() => navigate(`/data-quality/rules/${id}`)}>
                      {existingRule.rule_name}
                    </a>
                  ),
                },
              ]
            : []),
          { title: isEditing ? 'Edit' : 'New Rule' },
        ]}
      />

      <Space align="center" style={{ marginBottom: 16 }}>
        <Button
          type="text"
          icon={<ArrowLeftOutlined />}
          onClick={() => {
            if (isEditing && id) {
              navigate(`/data-quality/rules/${id}`);
            } else {
              navigate('/data-quality/rules');
            }
          }}
        />
        <Title level={3} style={{ margin: 0 }}>
          {isEditing ? 'Edit Rule' : 'New Quality Rule'}
        </Title>
      </Space>

      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmit}
          style={{ maxWidth: 800 }}
          scrollToFirstError
          initialValues={{ threshold_percentage: 100, is_active: true }}
        >
          <Form.Item
            name="rule_name"
            label="Rule Name"
            rules={[
              { required: true, message: 'Rule name is required' },
              { max: 512, message: 'Rule name cannot exceed 512 characters' },
            ]}
          >
            <Input placeholder="Enter the quality rule name" />
          </Form.Item>

          <Form.Item
            name="rule_code"
            label="Rule Code"
            rules={[
              { required: true, message: 'Rule code is required' },
              { max: 256, message: 'Rule code cannot exceed 256 characters' },
              {
                pattern: /^[a-z][a-z0-9]*(_[a-z0-9]+)*$/,
                message: 'Rule code must be snake_case (e.g., completeness_customer_name)',
              },
            ]}
            tooltip="Must be snake_case (e.g., completeness_customer_name)"
          >
            <Input placeholder="e.g., completeness_customer_name" />
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
              placeholder="Describe what this quality rule checks and why it matters"
            />
          </Form.Item>

          <Form.Item
            name="dimension_id"
            label="Quality Dimension"
            rules={[{ required: true, message: 'Quality dimension is required' }]}
          >
            <Select
              placeholder="Select a quality dimension"
              options={dimensionOptions}
              showSearch
              optionFilterProp="label"
            />
          </Form.Item>

          <Form.Item
            name="rule_type_id"
            label="Rule Type"
            rules={[{ required: true, message: 'Rule type is required' }]}
          >
            <Select
              placeholder="Select a rule type"
              options={ruleTypeOptions}
              showSearch
              optionFilterProp="label"
            />
          </Form.Item>

          <Form.Item
            name="element_id"
            label="Data Element"
            tooltip="Optionally link this rule to a specific data element"
          >
            <Select
              placeholder="Select a data element (optional)"
              options={elementOptions}
              allowClear
              showSearch
              optionFilterProp="label"
            />
          </Form.Item>

          <Form.Item
            name="rule_definition"
            label="Rule Definition"
            rules={[
              { required: true, message: 'Rule definition is required' },
              {
                validator: async (_, value) => {
                  if (!value) return;
                  try {
                    JSON.parse(value);
                  } catch {
                    throw new Error('Must be valid JSON');
                  }
                },
              },
            ]}
            tooltip="JSON definition of the rule logic"
          >
            <TextArea
              rows={6}
              placeholder={`{
  "type": "not_null",
  "column": "customer_name",
  "condition": "IS NOT NULL"
}`}
              style={{
                fontFamily: "'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace",
                fontSize: 13,
              }}
            />
          </Form.Item>

          <Form.Item
            name="threshold_percentage"
            label="Threshold %"
            rules={[
              { required: true, message: 'Threshold is required' },
              { type: 'number', min: 0, max: 100, message: 'Must be between 0 and 100' },
            ]}
            tooltip="Minimum acceptable score percentage for this rule"
          >
            <InputNumber
              min={0}
              max={100}
              style={{ width: 200 }}
              addonAfter="%"
              placeholder="100"
            />
          </Form.Item>

          <Form.Item
            name="severity"
            label="Severity"
            rules={[{ required: true, message: 'Severity is required' }]}
          >
            <Select
              placeholder="Select severity level"
              options={SEVERITY_OPTIONS}
            />
          </Form.Item>

          <Form.Item
            name="is_active"
            label="Active"
            valuePropName="checked"
          >
            <Switch checkedChildren="Active" unCheckedChildren="Inactive" />
          </Form.Item>

          <Form.Item style={{ marginTop: 24 }}>
            <Space>
              <Button type="primary" htmlType="submit" loading={submitting}>
                {isEditing ? 'Update Rule' : 'Create Rule'}
              </Button>
              <Button
                onClick={() => {
                  if (isEditing && id) {
                    navigate(`/data-quality/rules/${id}`);
                  } else {
                    navigate('/data-quality/rules');
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

export default QualityRuleForm;

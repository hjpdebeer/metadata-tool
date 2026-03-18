import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  Alert,
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
import { ArrowLeftOutlined, WarningOutlined } from '@ant-design/icons';
import { processesApi } from '../services/processesApi';
import type {
  BusinessProcessFullView,
  BusinessProcessListItem,
  CreateProcessRequest,
  ProcessCategory,
  UpdateProcessRequest,
} from '../services/processesApi';

const { Title } = Typography;
const { TextArea } = Input;

const FREQUENCIES = [
  { value: 'DAILY', label: 'Daily' },
  { value: 'WEEKLY', label: 'Weekly' },
  { value: 'MONTHLY', label: 'Monthly' },
  { value: 'QUARTERLY', label: 'Quarterly' },
  { value: 'ANNUAL', label: 'Annual' },
  { value: 'ON_DEMAND', label: 'On Demand' },
];

const ProcessForm: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const isEditing = Boolean(id);

  const [categories, setCategories] = useState<ProcessCategory[]>([]);
  const [parentProcesses, setParentProcesses] = useState<BusinessProcessListItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [existingProcess, setExistingProcess] = useState<BusinessProcessFullView | null>(null);
  const [isCritical, setIsCritical] = useState(false);

  const fetchReferenceData = useCallback(async () => {
    try {
      const [categoriesRes, processesRes] = await Promise.allSettled([
        processesApi.listCategories(),
        processesApi.listProcesses({ page_size: 500 }),
      ]);

      if (categoriesRes.status === 'fulfilled') {
        setCategories(categoriesRes.value.data);
      }
      if (processesRes.status === 'fulfilled') {
        const data = processesRes.value.data;
        if (Array.isArray(data)) {
          // Filter out the current process (can't be its own parent)
          setParentProcesses(data.filter((p) => p.process_id !== id));
        } else {
          const paginated = data as unknown as { data: BusinessProcessListItem[] };
          setParentProcesses(paginated.data.filter((p) => p.process_id !== id));
        }
      }
    } catch {
      // Non-critical; selects will just be empty
    }
  }, [id]);

  const fetchExistingProcess = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const response = await processesApi.getProcess(id);
      setExistingProcess(response.data);
      setIsCritical(response.data.is_critical);
      form.setFieldsValue({
        process_name: response.data.process_name,
        process_code: response.data.process_code,
        description: response.data.description,
        detailed_description: response.data.detailed_description || undefined,
        category_id: response.data.category_id || undefined,
        parent_process_id: response.data.parent_process_id || undefined,
        is_critical: response.data.is_critical,
        criticality_rationale: response.data.criticality_rationale || undefined,
        frequency: response.data.frequency || undefined,
        regulatory_requirement: response.data.regulatory_requirement || undefined,
        sla_description: response.data.sla_description || undefined,
        documentation_url: response.data.documentation_url || undefined,
      });
    } catch {
      message.error('Failed to load process for editing.');
      navigate('/processes');
    } finally {
      setLoading(false);
    }
  }, [id, form, navigate]);

  useEffect(() => {
    fetchReferenceData();
  }, [fetchReferenceData]);

  useEffect(() => {
    if (isEditing) {
      fetchExistingProcess();
    }
  }, [isEditing, fetchExistingProcess]);

  const handleSubmit = async (values: CreateProcessRequest & { is_critical: boolean }) => {
    setSubmitting(true);
    try {
      if (isEditing && id) {
        const updateData: UpdateProcessRequest = {};
        const fields: (keyof CreateProcessRequest)[] = [
          'process_name',
          'process_code',
          'description',
          'detailed_description',
          'category_id',
          'parent_process_id',
          'criticality_rationale',
          'frequency',
          'regulatory_requirement',
          'sla_description',
          'documentation_url',
        ];

        for (const field of fields) {
          const newVal = values[field];
          const oldVal = existingProcess?.[field as keyof BusinessProcessFullView];
          if (newVal !== oldVal) {
            (updateData as Record<string, unknown>)[field] = newVal || undefined;
          }
        }

        if (values.is_critical !== existingProcess?.is_critical) {
          updateData.is_critical = values.is_critical;
        }

        const response = await processesApi.updateProcess(id, updateData);
        message.success('Process updated successfully.');
        navigate(`/processes/${response.data.process_id}`);
      } else {
        const cleanData: CreateProcessRequest = {
          process_name: values.process_name,
          process_code: values.process_code,
          description: values.description,
          detailed_description: values.detailed_description || undefined,
          category_id: values.category_id || undefined,
          parent_process_id: values.parent_process_id || undefined,
          is_critical: values.is_critical,
          criticality_rationale: values.criticality_rationale || undefined,
          frequency: values.frequency || undefined,
          regulatory_requirement: values.regulatory_requirement || undefined,
          sla_description: values.sla_description || undefined,
          documentation_url: values.documentation_url || undefined,
        };

        const response = await processesApi.createProcess(cleanData);
        message.success('Process created successfully.');
        navigate(`/processes/${response.data.process_id}`);
      }
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { message?: string }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.message || 'Validation error. Please check the form.');
      } else {
        message.error(isEditing ? 'Failed to update process.' : 'Failed to create process.');
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

  const categoryOptions = categories.map((c) => ({
    value: c.category_id,
    label: c.category_name,
  }));

  const parentProcessOptions = parentProcesses.map((p) => ({
    value: p.process_id,
    label: `${p.process_name} (${p.process_code})`,
  }));

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/processes')}>Business Processes</a> },
          ...(isEditing && existingProcess
            ? [
                {
                  title: (
                    <a onClick={() => navigate(`/processes/${id}`)}>
                      {existingProcess.process_name}
                    </a>
                  ),
                },
              ]
            : []),
          { title: isEditing ? 'Edit' : 'New Process' },
        ]}
      />

      <Space align="center" style={{ marginBottom: 16 }}>
        <Button
          type="text"
          icon={<ArrowLeftOutlined />}
          onClick={() => {
            if (isEditing && id) {
              navigate(`/processes/${id}`);
            } else {
              navigate('/processes');
            }
          }}
        />
        <Title level={3} style={{ margin: 0 }}>
          {isEditing ? 'Edit Process' : 'New Business Process'}
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
            name="process_name"
            label="Process Name"
            rules={[
              { required: true, message: 'Process name is required' },
              { max: 256, message: 'Process name cannot exceed 256 characters' },
            ]}
          >
            <Input placeholder="Enter the business process name" />
          </Form.Item>

          <Form.Item
            name="process_code"
            label="Process Code"
            rules={[
              { required: true, message: 'Process code is required' },
              { max: 64, message: 'Process code cannot exceed 64 characters' },
              {
                pattern: /^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*$/,
                message: 'Process code must be UPPER_SNAKE_CASE (e.g., CUSTOMER_ONBOARDING)',
              },
            ]}
            tooltip="Must be UPPER_SNAKE_CASE (e.g., CUSTOMER_ONBOARDING)"
          >
            <Input placeholder="e.g., CUSTOMER_ONBOARDING" disabled={isEditing} />
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
              placeholder="Provide a clear, concise description of this business process"
            />
          </Form.Item>

          <Form.Item
            name="detailed_description"
            label="Detailed Description"
            tooltip="Extended description with step-by-step details, constraints, and edge cases"
          >
            <TextArea
              rows={4}
              placeholder="Provide detailed process documentation"
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
            name="parent_process_id"
            label="Parent Process"
            tooltip="Select a parent process if this is a sub-process"
          >
            <Select
              placeholder="Select a parent process"
              options={parentProcessOptions}
              allowClear
              showSearch
              optionFilterProp="label"
            />
          </Form.Item>

          <Form.Item
            name="is_critical"
            label="Critical Business Process"
            valuePropName="checked"
          >
            <Switch
              checkedChildren="Yes"
              unCheckedChildren="No"
              onChange={(checked) => setIsCritical(checked)}
            />
          </Form.Item>

          {isCritical && (
            <>
              <Alert
                type="warning"
                showIcon
                icon={<WarningOutlined />}
                message="CDE Auto-Designation"
                description="All data elements linked to this process will be automatically designated as Critical Data Elements (CDEs). This cannot be undone by simply unlinking — CDE status must be removed manually from each element."
                style={{ marginBottom: 16 }}
              />
              <Form.Item
                name="criticality_rationale"
                label="Criticality Rationale"
                tooltip="Explain why this process is classified as critical"
                rules={[
                  { required: true, message: 'Please provide a rationale for critical designation' },
                ]}
              >
                <TextArea
                  rows={3}
                  placeholder="e.g., Regulatory reporting process mandated by the central bank — failure to execute impacts compliance"
                />
              </Form.Item>
            </>
          )}

          <Form.Item name="frequency" label="Frequency">
            <Select
              placeholder="Select process frequency"
              options={FREQUENCIES}
              allowClear
            />
          </Form.Item>

          <Form.Item
            name="regulatory_requirement"
            label="Regulatory Requirement"
            tooltip="Reference to the regulation that mandates or governs this process"
          >
            <TextArea
              rows={3}
              placeholder="e.g., Basel III Pillar 3 disclosure requirement, BCBS 239 compliance"
            />
          </Form.Item>

          <Form.Item
            name="sla_description"
            label="SLA Description"
            tooltip="Service Level Agreement details for this process"
          >
            <TextArea
              rows={2}
              placeholder="e.g., Must complete by T+1 end of business, 99.9% uptime"
            />
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
                {isEditing ? 'Update Process' : 'Create Process'}
              </Button>
              <Button
                onClick={() => {
                  if (isEditing && id) {
                    navigate(`/processes/${id}`);
                  } else {
                    navigate('/processes');
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

export default ProcessForm;

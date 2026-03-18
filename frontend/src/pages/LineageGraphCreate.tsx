import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Breadcrumb,
  Button,
  Card,
  Form,
  Input,
  Select,
  Space,
  Typography,
  message,
} from 'antd';
import { ArrowLeftOutlined } from '@ant-design/icons';
import { lineageApi } from '../services/lineageApi';
import type { CreateLineageGraphRequest } from '../services/lineageApi';

const { Title } = Typography;
const { TextArea } = Input;

const graphTypeOptions = [
  { value: 'BUSINESS', label: 'Business Lineage' },
  { value: 'TECHNICAL', label: 'Technical Lineage' },
];

const LineageGraphCreate: React.FC = () => {
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (values: CreateLineageGraphRequest) => {
    setSubmitting(true);
    try {
      const response = await lineageApi.createGraph({
        graph_name: values.graph_name,
        graph_type: values.graph_type,
        description: values.description || undefined,
      });
      message.success('Lineage graph created successfully.');
      navigate(`/lineage/${response.data.graph_id}`);
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { message?: string }; status?: number } };
      if (axiosErr.response?.status === 422) {
        message.error(axiosErr.response.data?.message || 'Validation error. Please check the form.');
      } else {
        message.error('Failed to create lineage graph.');
      }
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/lineage')}>Data Lineage</a> },
          { title: 'New Graph' },
        ]}
      />

      <Space align="center" style={{ marginBottom: 16 }}>
        <Button
          type="text"
          icon={<ArrowLeftOutlined />}
          onClick={() => navigate('/lineage')}
        />
        <Title level={3} style={{ margin: 0 }}>
          New Lineage Graph
        </Title>
      </Space>

      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmit}
          style={{ maxWidth: 600 }}
          scrollToFirstError
        >
          <Form.Item
            name="graph_name"
            label="Graph Name"
            rules={[
              { required: true, message: 'Graph name is required' },
              { max: 256, message: 'Graph name cannot exceed 256 characters' },
            ]}
          >
            <Input placeholder="e.g., Customer Data Flow, Risk Reporting Pipeline" />
          </Form.Item>

          <Form.Item
            name="graph_type"
            label="Graph Type"
            rules={[{ required: true, message: 'Graph type is required' }]}
            tooltip="Business lineage maps data flows at a conceptual level. Technical lineage captures system-level detail."
          >
            <Select placeholder="Select graph type" options={graphTypeOptions} />
          </Form.Item>

          <Form.Item
            name="description"
            label="Description"
            rules={[
              {
                min: 10,
                message: 'Description should be at least 10 characters for clarity',
              },
            ]}
          >
            <TextArea
              rows={4}
              placeholder="Describe the scope and purpose of this lineage graph"
            />
          </Form.Item>

          <Form.Item style={{ marginTop: 24 }}>
            <Space>
              <Button type="primary" htmlType="submit" loading={submitting}>
                Create Graph
              </Button>
              <Button onClick={() => navigate('/lineage')}>Cancel</Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>
    </div>
  );
};

export default LineageGraphCreate;

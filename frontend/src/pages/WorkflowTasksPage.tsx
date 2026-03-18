import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Card, Empty, Table, Tag, Typography, message } from 'antd';
import { EyeOutlined } from '@ant-design/icons';
import { workflowApi } from '../services/glossaryApi';
import type { PendingTask } from '../services/glossaryApi';

const { Title, Text } = Typography;

/** Map entity_type strings to route prefixes. */
const entityRoutes: Record<string, string> = {
  glossary_term: '/glossary',
  data_element: '/data-dictionary',
  quality_rule: '/data-quality',
  application: '/applications',
  process: '/processes',
};

const entityTypeLabels: Record<string, string> = {
  glossary_term: 'Glossary Term',
  data_element: 'Data Element',
  quality_rule: 'Quality Rule',
  application: 'Application',
  process: 'Business Process',
};

const WorkflowTasksPage: React.FC = () => {
  const navigate = useNavigate();
  const [tasks, setTasks] = useState<PendingTask[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchTasks = useCallback(async () => {
    setLoading(true);
    try {
      const response = await workflowApi.getPendingTasks();
      setTasks(response.data);
    } catch {
      message.error('Failed to load pending tasks.');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchTasks();
  }, [fetchTasks]);

  const navigateToEntity = (task: PendingTask) => {
    const routePrefix = entityRoutes[task.entity_type] || '/glossary';
    navigate(`${routePrefix}/${task.entity_id}`);
  };

  const isOverdue = (dueDate: string | null) => {
    if (!dueDate) return false;
    return new Date(dueDate) < new Date();
  };

  const columns = [
    {
      title: 'Entity Type',
      dataIndex: 'entity_type',
      key: 'entity_type',
      width: 160,
      render: (type: string) => (
        <Tag color="blue">{entityTypeLabels[type] || type}</Tag>
      ),
    },
    {
      title: 'Entity Name',
      dataIndex: 'entity_name',
      key: 'entity_name',
      render: (name: string, record: PendingTask) => (
        <a onClick={() => navigateToEntity(record)}>{name}</a>
      ),
    },
    {
      title: 'Workflow',
      dataIndex: 'workflow_name',
      key: 'workflow_name',
    },
    {
      title: 'Task',
      key: 'task_name',
      render: (_: unknown, record: PendingTask) => record.task.task_name,
    },
    {
      title: 'Submitted By',
      dataIndex: 'submitted_by',
      key: 'submitted_by',
    },
    {
      title: 'Submitted At',
      dataIndex: 'submitted_at',
      key: 'submitted_at',
      width: 160,
      render: (date: string) => {
        if (!date) return '-';
        return new Date(date).toLocaleDateString('en-ZA', {
          year: 'numeric',
          month: 'short',
          day: 'numeric',
        });
      },
    },
    {
      title: 'Due Date',
      key: 'due_date',
      width: 160,
      render: (_: unknown, record: PendingTask) => {
        const dueDate = record.task.due_date;
        if (!dueDate) return <Text type="secondary">No due date</Text>;

        const overdue = isOverdue(dueDate);
        const formatted = new Date(dueDate).toLocaleDateString('en-ZA', {
          year: 'numeric',
          month: 'short',
          day: 'numeric',
        });

        return overdue ? (
          <Text type="danger" strong>
            {formatted} (overdue)
          </Text>
        ) : (
          <Text>{formatted}</Text>
        );
      },
    },
    {
      title: 'Actions',
      key: 'actions',
      width: 100,
      render: (_: unknown, record: PendingTask) => (
        <Button
          type="link"
          icon={<EyeOutlined />}
          onClick={() => navigateToEntity(record)}
        >
          Review
        </Button>
      ),
    },
  ];

  return (
    <div>
      <Title level={3} style={{ marginBottom: 16 }}>
        My Tasks
      </Title>
      <Card>
        {!loading && tasks.length === 0 ? (
          <Empty
            description="No pending tasks assigned to you"
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        ) : (
          <Table
            columns={columns}
            dataSource={tasks}
            rowKey={(record) => record.task.task_id}
            loading={loading}
            pagination={{
              pageSize: 20,
              showSizeChanger: true,
              showTotal: (total) => `${total} pending tasks`,
            }}
            rowClassName={(record) =>
              isOverdue(record.task.due_date) ? 'overdue-row' : ''
            }
          />
        )}
      </Card>

      <style>{`
        .overdue-row {
          background-color: #FFF1F0;
        }
        .overdue-row:hover > td {
          background-color: #FFCCC7 !important;
        }
      `}</style>
    </div>
  );
};

export default WorkflowTasksPage;

import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Button,
  Card,
  Col,
  List,
  Row,
  Skeleton,
  Statistic,
  Tag,
  Typography,
} from 'antd';
import {
  AppstoreOutlined,
  BookOutlined,
  CheckSquareOutlined,
  DatabaseOutlined,
  EyeOutlined,
  SafetyCertificateOutlined,
} from '@ant-design/icons';
import { glossaryApi, statsApi, workflowApi } from '../services/glossaryApi';
import type { GlossaryTermListItem, PendingTask, Stats } from '../services/glossaryApi';

const { Title, Text } = Typography;

const statusColors: Record<string, string> = {
  DRAFT: 'default',
  PROPOSED: 'processing',
  UNDER_REVIEW: 'warning',
  REVISED: 'orange',
  ACCEPTED: 'success',
  REJECTED: 'error',
  DEPRECATED: 'default',
};

const entityRoutes: Record<string, string> = {
  glossary_term: '/glossary',
  data_element: '/data-dictionary',
  quality_rule: '/data-quality',
  application: '/applications',
  process: '/processes',
};

const Dashboard: React.FC = () => {
  const navigate = useNavigate();

  const [stats, setStats] = useState<Stats | null>(null);
  const [pendingTasks, setPendingTasks] = useState<PendingTask[]>([]);
  const [recentTerms, setRecentTerms] = useState<GlossaryTermListItem[]>([]);
  const [statsLoading, setStatsLoading] = useState(true);
  const [tasksLoading, setTasksLoading] = useState(true);
  const [termsLoading, setTermsLoading] = useState(true);

  const fetchStats = useCallback(async () => {
    setStatsLoading(true);
    try {
      const response = await statsApi.getStats();
      setStats(response.data);
    } catch {
      // Stats endpoint may not be implemented; show zeros
      setStats({
        glossary_terms: 0,
        data_elements: 0,
        critical_data_elements: 0,
        quality_rules: 0,
        applications: 0,
        pending_tasks: 0,
      });
    } finally {
      setStatsLoading(false);
    }
  }, []);

  const fetchPendingTasks = useCallback(async () => {
    setTasksLoading(true);
    try {
      const response = await workflowApi.getPendingTasks();
      setPendingTasks(response.data.slice(0, 5));
    } catch {
      setPendingTasks([]);
    } finally {
      setTasksLoading(false);
    }
  }, []);

  const fetchRecentTerms = useCallback(async () => {
    setTermsLoading(true);
    try {
      const response = await glossaryApi.listTerms({ page: 1, page_size: 5 });
      const data = response.data;
      if (Array.isArray(data)) {
        setRecentTerms(data.slice(0, 5));
      } else {
        const paginated = data as unknown as { data: GlossaryTermListItem[] };
        setRecentTerms(paginated.data.slice(0, 5));
      }
    } catch {
      setRecentTerms([]);
    } finally {
      setTermsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStats();
    fetchPendingTasks();
    fetchRecentTerms();
  }, [fetchStats, fetchPendingTasks, fetchRecentTerms]);

  const statCards = [
    {
      title: 'Glossary Terms',
      value: stats?.glossary_terms ?? 0,
      icon: <BookOutlined />,
      path: '/glossary',
    },
    {
      title: 'Data Elements',
      value: stats?.data_elements ?? 0,
      icon: <DatabaseOutlined />,
      path: '/data-dictionary',
    },
    {
      title: 'Critical Data Elements',
      value: stats?.critical_data_elements ?? 0,
      icon: <DatabaseOutlined />,
      path: '/data-dictionary',
      valueStyle: { color: '#FF4D4F' },
    },
    {
      title: 'Quality Rules',
      value: stats?.quality_rules ?? 0,
      icon: <SafetyCertificateOutlined />,
      path: '/data-quality',
    },
    {
      title: 'Applications',
      value: stats?.applications ?? 0,
      icon: <AppstoreOutlined />,
      path: '/applications',
    },
    {
      title: 'Pending Tasks',
      value: stats?.pending_tasks ?? 0,
      icon: <CheckSquareOutlined />,
      path: '/workflow',
      valueStyle: { color: '#FAAD14' },
    },
  ];

  return (
    <div>
      <Title level={3}>Dashboard</Title>
      <Row gutter={[16, 16]}>
        {statCards.map((card) => (
          <Col xs={24} sm={12} lg={8} xl={4} key={card.title}>
            <Card
              hoverable
              onClick={() => navigate(card.path)}
              style={{ cursor: 'pointer' }}
            >
              {statsLoading ? (
                <Skeleton active paragraph={false} title={{ width: '60%' }} />
              ) : (
                <Statistic
                  title={card.title}
                  value={card.value}
                  prefix={card.icon}
                  valueStyle={card.valueStyle}
                />
              )}
            </Card>
          </Col>
        ))}
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 24 }}>
        <Col xs={24} lg={12}>
          <Card
            title="Recent Terms"
            extra={
              <Button type="link" size="small" onClick={() => navigate('/glossary')}>
                View All
              </Button>
            }
          >
            {termsLoading ? (
              <Skeleton active paragraph={{ rows: 4 }} />
            ) : recentTerms.length === 0 ? (
              <Text type="secondary">
                No glossary terms yet. Start by creating one.
              </Text>
            ) : (
              <List
                dataSource={recentTerms}
                renderItem={(item) => (
                  <List.Item
                    actions={[
                      <Button
                        key="view"
                        type="link"
                        size="small"
                        icon={<EyeOutlined />}
                        onClick={() => navigate(`/glossary/${item.term_id}`)}
                      >
                        View
                      </Button>,
                    ]}
                  >
                    <List.Item.Meta
                      title={
                        <a onClick={() => navigate(`/glossary/${item.term_id}`)}>
                          {item.term_name}
                        </a>
                      }
                      description={
                        <div>
                          <Text ellipsis style={{ maxWidth: 300, display: 'inline-block' }}>
                            {item.definition}
                          </Text>
                          <br />
                          <Tag
                            color={statusColors[item.status_code] || 'default'}
                            style={{ marginTop: 4 }}
                          >
                            {item.status_code}
                          </Tag>
                          <Text
                            type="secondary"
                            style={{ fontSize: 12, marginLeft: 8 }}
                          >
                            {new Date(item.updated_at).toLocaleDateString('en-ZA', {
                              month: 'short',
                              day: 'numeric',
                            })}
                          </Text>
                        </div>
                      }
                    />
                  </List.Item>
                )}
              />
            )}
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card
            title="My Pending Tasks"
            extra={
              <Button type="link" size="small" onClick={() => navigate('/workflow')}>
                View All
              </Button>
            }
          >
            {tasksLoading ? (
              <Skeleton active paragraph={{ rows: 4 }} />
            ) : pendingTasks.length === 0 ? (
              <Text type="secondary">No pending tasks assigned to you.</Text>
            ) : (
              <List
                dataSource={pendingTasks}
                renderItem={(item) => {
                  const routePrefix =
                    entityRoutes[item.entity_type] || '/glossary';
                  return (
                    <List.Item
                      actions={[
                        <Button
                          key="review"
                          type="link"
                          size="small"
                          icon={<EyeOutlined />}
                          onClick={() =>
                            navigate(`${routePrefix}/${item.entity_id}`)
                          }
                        >
                          Review
                        </Button>,
                      ]}
                    >
                      <List.Item.Meta
                        title={
                          <a
                            onClick={() =>
                              navigate(`${routePrefix}/${item.entity_id}`)
                            }
                          >
                            {item.entity_name}
                          </a>
                        }
                        description={
                          <div>
                            <Tag color="blue">{item.entity_type}</Tag>
                            <Text type="secondary" style={{ fontSize: 12 }}>
                              {item.task.task_name}
                              {' - submitted by '}
                              {item.submitted_by}
                            </Text>
                            {item.task.due_date && (
                              <>
                                <br />
                                <Text
                                  type={
                                    new Date(item.task.due_date) < new Date()
                                      ? 'danger'
                                      : 'secondary'
                                  }
                                  style={{ fontSize: 12 }}
                                >
                                  Due:{' '}
                                  {new Date(item.task.due_date).toLocaleDateString(
                                    'en-ZA',
                                    {
                                      month: 'short',
                                      day: 'numeric',
                                    },
                                  )}
                                </Text>
                              </>
                            )}
                          </div>
                        }
                      />
                    </List.Item>
                  );
                }}
              />
            )}
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default Dashboard;

import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Breadcrumb,
  Button,
  Card,
  Col,
  Empty,
  Row,
  Space,
  Spin,
  Table,
  Tag,
  Tooltip,
  Tree,
  Typography,
  message,
} from 'antd';
import type { TreeDataNode } from 'antd';
import {
  ArrowLeftOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  DatabaseOutlined,
  FolderOutlined,
  KeyOutlined,
  LinkOutlined,
  TableOutlined,
} from '@ant-design/icons';
import { dataDictionaryApi } from '../services/dataDictionaryApi';
import type {
  SourceSystem,
  TechnicalColumn,
  TechnicalSchema,
  TechnicalTable,
} from '../services/dataDictionaryApi';

const { Title, Text } = Typography;

interface TreeNodeData {
  type: 'system' | 'schema' | 'table';
  systemId?: string;
  schemaId?: string;
  tableId?: string;
}

const TechnicalMetadataPage: React.FC = () => {
  const navigate = useNavigate();

  const [systems, setSystems] = useState<SourceSystem[]>([]);
  const [schemasMap, setSchemasMap] = useState<Record<string, TechnicalSchema[]>>({});
  const [tablesMap, setTablesMap] = useState<Record<string, TechnicalTable[]>>({});
  const [columns, setColumns] = useState<TechnicalColumn[]>([]);
  const [selectedTable, setSelectedTable] = useState<TechnicalTable | null>(null);
  const [loadingSystems, setLoadingSystems] = useState(false);
  const [loadingColumns, setLoadingColumns] = useState(false);
  const [expandedKeys, setExpandedKeys] = useState<React.Key[]>([]);
  const [treeNodeDataMap] = useState<Record<string, TreeNodeData>>({});

  const fetchSystems = useCallback(async () => {
    setLoadingSystems(true);
    try {
      const response = await dataDictionaryApi.listSourceSystems();
      setSystems(response.data);
    } catch {
      message.error('Failed to load source systems.');
    } finally {
      setLoadingSystems(false);
    }
  }, []);

  useEffect(() => {
    fetchSystems();
  }, [fetchSystems]);

  const fetchSchemas = async (systemId: string) => {
    if (schemasMap[systemId]) return; // Already loaded
    try {
      const response = await dataDictionaryApi.listSchemas(systemId);
      setSchemasMap((prev) => ({ ...prev, [systemId]: response.data }));
    } catch {
      message.error('Failed to load schemas.');
    }
  };

  const fetchTables = async (schemaId: string) => {
    if (tablesMap[schemaId]) return; // Already loaded
    try {
      const response = await dataDictionaryApi.listTables(schemaId);
      setTablesMap((prev) => ({ ...prev, [schemaId]: response.data }));
    } catch {
      message.error('Failed to load tables.');
    }
  };

  const fetchColumns = async (tableId: string) => {
    setLoadingColumns(true);
    try {
      const response = await dataDictionaryApi.listColumns(tableId);
      setColumns(response.data);
    } catch {
      message.error('Failed to load columns.');
      setColumns([]);
    } finally {
      setLoadingColumns(false);
    }
  };

  const onTreeExpand = (keys: React.Key[]) => {
    setExpandedKeys(keys);

    // Load child data when a node is expanded
    keys.forEach((key) => {
      const keyStr = String(key);
      const nodeData = treeNodeDataMap[keyStr];
      if (!nodeData) return;

      if (nodeData.type === 'system' && nodeData.systemId) {
        fetchSchemas(nodeData.systemId);
      } else if (nodeData.type === 'schema' && nodeData.schemaId) {
        fetchTables(nodeData.schemaId);
      }
    });
  };

  const onTreeSelect = (selectedKeys: React.Key[]) => {
    if (selectedKeys.length === 0) return;

    const keyStr = String(selectedKeys[0]);
    const nodeData = treeNodeDataMap[keyStr];
    if (!nodeData) return;

    if (nodeData.type === 'table' && nodeData.tableId) {
      // Find the table in the tablesMap
      for (const tables of Object.values(tablesMap)) {
        const table = tables.find((t) => t.table_id === nodeData.tableId);
        if (table) {
          setSelectedTable(table);
          fetchColumns(table.table_id);
          break;
        }
      }
    }
  };

  const buildTreeData = (): TreeDataNode[] => {
    return systems.map((system) => {
      const systemKey = `system-${system.system_id}`;
      treeNodeDataMap[systemKey] = { type: 'system', systemId: system.system_id };

      const schemas = schemasMap[system.system_id] || [];
      const schemaChildren: TreeDataNode[] = schemas.map((schema) => {
        const schemaKey = `schema-${schema.schema_id}`;
        treeNodeDataMap[schemaKey] = { type: 'schema', schemaId: schema.schema_id };

        const tables = tablesMap[schema.schema_id] || [];
        const tableChildren: TreeDataNode[] = tables.map((table) => {
          const tableKey = `table-${table.table_id}`;
          treeNodeDataMap[tableKey] = { type: 'table', tableId: table.table_id };

          return {
            key: tableKey,
            title: (
              <Space size={4}>
                <Text style={{ fontSize: 13 }}>{table.table_name}</Text>
                <Tag style={{ fontSize: 11 }}>{table.table_type}</Tag>
              </Space>
            ),
            icon: <TableOutlined />,
            isLeaf: true,
          };
        });

        return {
          key: schemaKey,
          title: schema.schema_name,
          icon: <FolderOutlined />,
          children: tableChildren,
        };
      });

      return {
        key: systemKey,
        title: (
          <Space size={4}>
            <Text strong style={{ fontSize: 13 }}>{system.system_name}</Text>
            <Tag style={{ fontSize: 11 }}>{system.system_type}</Tag>
          </Space>
        ),
        icon: <DatabaseOutlined />,
        children: schemaChildren,
      };
    });
  };

  const formatBytes = (bytes: number | null): string => {
    if (bytes === null || bytes === undefined) return '-';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  };

  const columnTableColumns = [
    {
      title: 'Column Name',
      dataIndex: 'column_name',
      key: 'column_name',
      render: (name: string) => (
        <Text code style={{ fontSize: 12 }}>
          {name}
        </Text>
      ),
    },
    {
      title: '#',
      dataIndex: 'ordinal_position',
      key: 'ordinal_position',
      width: 50,
      align: 'center' as const,
    },
    {
      title: 'Data Type',
      dataIndex: 'data_type',
      key: 'data_type',
      width: 120,
      render: (type: string, record: TechnicalColumn) => {
        let display = type;
        if (record.max_length) display += `(${record.max_length})`;
        else if (record.numeric_precision) display += `(${record.numeric_precision})`;
        return display;
      },
    },
    {
      title: 'Nullable',
      dataIndex: 'is_nullable',
      key: 'is_nullable',
      width: 80,
      align: 'center' as const,
      render: (val: boolean) => (val ? 'Yes' : 'No'),
    },
    {
      title: 'Keys',
      key: 'keys',
      width: 100,
      render: (_: unknown, record: TechnicalColumn) => (
        <Space size={4}>
          {record.is_primary_key && (
            <Tooltip title="Primary Key">
              <Tag color="gold" icon={<KeyOutlined />}>
                PK
              </Tag>
            </Tooltip>
          )}
          {record.is_foreign_key && (
            <Tooltip title="Foreign Key">
              <Tag color="blue" icon={<LinkOutlined />}>
                FK
              </Tag>
            </Tooltip>
          )}
        </Space>
      ),
    },
    {
      title: 'Linked Element',
      dataIndex: 'element_name',
      key: 'element_name',
      render: (name: string | null, record: TechnicalColumn) =>
        name ? (
          <a onClick={() => navigate(`/data-dictionary/${record.element_id}`)}>{name}</a>
        ) : (
          <Text type="secondary">-</Text>
        ),
    },
    {
      title: 'Naming Compliance',
      key: 'naming_compliance',
      width: 170,
      render: (_: unknown, record: TechnicalColumn) => {
        if (record.naming_standard_compliant === null) {
          return <Text type="secondary">-</Text>;
        }
        return record.naming_standard_compliant ? (
          <Tag icon={<CheckCircleOutlined />} color="success">
            Compliant
          </Tag>
        ) : (
          <Tooltip title={record.naming_standard_violation || 'Naming violation'}>
            <Tag icon={<CloseCircleOutlined />} color="error">
              Violation
            </Tag>
          </Tooltip>
        );
      },
    },
  ];

  const treeData = buildTreeData();

  return (
    <div>
      <Breadcrumb
        style={{ marginBottom: 16 }}
        items={[
          { title: <a onClick={() => navigate('/data-dictionary')}>Data Dictionary</a> },
          { title: 'Technical Metadata' },
        ]}
      />

      <Space align="center" style={{ marginBottom: 16 }}>
        <Button
          type="text"
          icon={<ArrowLeftOutlined />}
          onClick={() => navigate('/data-dictionary')}
        />
        <Title level={3} style={{ margin: 0 }}>
          Technical Metadata Browser
        </Title>
      </Space>

      <Row gutter={16}>
        <Col xs={24} md={8} lg={6}>
          <Card
            title="Source Systems"
            size="small"
            style={{ height: 'calc(100vh - 200px)', overflow: 'auto' }}
          >
            {loadingSystems ? (
              <div style={{ textAlign: 'center', padding: 40 }}>
                <Spin />
              </div>
            ) : systems.length === 0 ? (
              <Empty
                description="No source systems registered"
                image={Empty.PRESENTED_IMAGE_SIMPLE}
              />
            ) : (
              <Tree
                showIcon
                treeData={treeData}
                expandedKeys={expandedKeys}
                onExpand={onTreeExpand}
                onSelect={onTreeSelect}
                style={{ fontSize: 13 }}
              />
            )}
          </Card>
        </Col>
        <Col xs={24} md={16} lg={18}>
          <Card
            title={
              selectedTable ? (
                <Space>
                  <TableOutlined />
                  <Text strong>{selectedTable.table_name}</Text>
                  <Tag>{selectedTable.table_type}</Tag>
                  {selectedTable.row_count !== null && (
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {selectedTable.row_count?.toLocaleString()} rows
                    </Text>
                  )}
                  {selectedTable.size_bytes !== null && (
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      ({formatBytes(selectedTable.size_bytes)})
                    </Text>
                  )}
                </Space>
              ) : (
                'Columns'
              )
            }
            size="small"
          >
            {!selectedTable ? (
              <Empty
                description="Select a table from the tree to view its columns"
                image={Empty.PRESENTED_IMAGE_SIMPLE}
              />
            ) : (
              <>
                {selectedTable.description && (
                  <div style={{ marginBottom: 16 }}>
                    <Text type="secondary">{selectedTable.description}</Text>
                  </div>
                )}
                <Table
                  columns={columnTableColumns}
                  dataSource={columns}
                  rowKey="column_id"
                  loading={loadingColumns}
                  pagination={false}
                  size="small"
                />
              </>
            )}
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default TechnicalMetadataPage;

import React, { useCallback, useEffect, useRef, useState } from 'react';
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
  Statistic,
  Table,
  Tag,
  Tooltip,
  Tree,
  Typography,
  message,
} from 'antd';
import type { TreeDataNode } from 'antd';
import type { EventDataNode } from 'antd/es/tree';
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

  // Use a ref for the node data map to avoid mutating state during render
  const treeNodeDataMapRef = useRef<Record<string, TreeNodeData>>({});

  // Track which systems/schemas have been loaded
  const [loadedSystems, setLoadedSystems] = useState<Set<string>>(new Set());
  const [loadedSchemas, setLoadedSchemas] = useState<Set<string>>(new Set());

  // Summary counts
  const totalSchemas = Object.values(schemasMap).reduce((sum, arr) => sum + arr.length, 0);
  const totalTables = Object.values(tablesMap).reduce((sum, arr) => sum + arr.length, 0);
  const totalColumns = columns.length;

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

  const fetchSchemas = async (systemId: string): Promise<TechnicalSchema[]> => {
    if (schemasMap[systemId]) return schemasMap[systemId];
    try {
      const response = await dataDictionaryApi.listSchemas(systemId);
      setSchemasMap((prev) => ({ ...prev, [systemId]: response.data }));
      setLoadedSystems((prev) => new Set([...prev, systemId]));
      return response.data;
    } catch {
      message.error('Failed to load schemas.');
      return [];
    }
  };

  const fetchTables = async (schemaId: string): Promise<TechnicalTable[]> => {
    if (tablesMap[schemaId]) return tablesMap[schemaId];
    try {
      const response = await dataDictionaryApi.listTables(schemaId);
      setTablesMap((prev) => ({ ...prev, [schemaId]: response.data }));
      setLoadedSchemas((prev) => new Set([...prev, schemaId]));
      return response.data;
    } catch {
      message.error('Failed to load tables.');
      return [];
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

  // Async load handler for Ant Design Tree's loadData prop
  const onLoadData = async (treeNode: EventDataNode<TreeDataNode>): Promise<void> => {
    const keyStr = String(treeNode.key);
    const nodeData = treeNodeDataMapRef.current[keyStr];
    if (!nodeData) return;

    if (nodeData.type === 'system' && nodeData.systemId) {
      await fetchSchemas(nodeData.systemId);
    } else if (nodeData.type === 'schema' && nodeData.schemaId) {
      await fetchTables(nodeData.schemaId);
    }
  };

  const onTreeExpand = (keys: React.Key[]) => {
    setExpandedKeys(keys);
  };

  const onTreeSelect = (selectedKeys: React.Key[]) => {
    if (selectedKeys.length === 0) return;

    const keyStr = String(selectedKeys[0]);
    const nodeData = treeNodeDataMapRef.current[keyStr];
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
    const map = treeNodeDataMapRef.current;

    return systems.map((system) => {
      const systemKey = `system-${system.system_id}`;
      map[systemKey] = { type: 'system', systemId: system.system_id };

      const systemLoaded = loadedSystems.has(system.system_id);
      const schemas = schemasMap[system.system_id] || [];

      const schemaChildren: TreeDataNode[] = schemas.map((schema) => {
        const schemaKey = `schema-${schema.schema_id}`;
        map[schemaKey] = { type: 'schema', schemaId: schema.schema_id };

        const schemaLoaded = loadedSchemas.has(schema.schema_id);
        const tables = tablesMap[schema.schema_id] || [];

        const tableChildren: TreeDataNode[] = tables.map((table) => {
          const tableKey = `table-${table.table_id}`;
          map[tableKey] = { type: 'table', tableId: table.table_id };

          const tooltipLines: string[] = [];
          if (table.description) tooltipLines.push(table.description);
          if (table.row_count !== null && table.row_count !== undefined) {
            tooltipLines.push(`${table.row_count.toLocaleString()} rows`);
          }
          if (table.size_bytes !== null && table.size_bytes !== undefined) {
            tooltipLines.push(formatBytes(table.size_bytes));
          }
          const tooltipText = tooltipLines.length > 0 ? tooltipLines.join(' | ') : undefined;

          const titleNode = (
            <Space size={4}>
              <Text style={{ fontSize: 13 }}>{table.table_name}</Text>
              <Tag style={{ fontSize: 11 }}>{table.table_type}</Tag>
              {table.row_count !== null && table.row_count !== undefined && (
                <Text type="secondary" style={{ fontSize: 11 }}>
                  ({table.row_count.toLocaleString()})
                </Text>
              )}
            </Space>
          );

          return {
            key: tableKey,
            title: tooltipText ? (
              <Tooltip title={tooltipText}>{titleNode}</Tooltip>
            ) : (
              titleNode
            ),
            icon: <TableOutlined />,
            isLeaf: true,
          };
        });

        const schemaTitleNode = (
          <Text style={{ fontSize: 13 }}>{schema.schema_name}</Text>
        );

        return {
          key: schemaKey,
          title: schema.description ? (
            <Tooltip title={schema.description}>{schemaTitleNode}</Tooltip>
          ) : (
            schemaTitleNode
          ),
          icon: <FolderOutlined />,
          children: schemaLoaded ? tableChildren : undefined,
          isLeaf: false,
        };
      });

      // Build system title with application info
      const systemInfoParts: string[] = [];
      if (system.environment) systemInfoParts.push(system.environment);
      if (system.vendor) systemInfoParts.push(system.vendor);

      return {
        key: systemKey,
        title: (
          <Space size={4}>
            <Text strong style={{ fontSize: 13 }}>
              {system.system_name}
            </Text>
            <Tag style={{ fontSize: 11 }}>{system.system_type}</Tag>
            {systemInfoParts.length > 0 && (
              <Text type="secondary" style={{ fontSize: 11 }}>
                ({systemInfoParts.join(', ')})
              </Text>
            )}
          </Space>
        ),
        icon: <DatabaseOutlined />,
        children: systemLoaded ? schemaChildren : undefined,
        isLeaf: false,
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

      {/* Summary statistics */}
      <Row gutter={16} style={{ marginBottom: 16 }}>
        <Col xs={6}>
          <Card size="small">
            <Statistic
              title="Source Systems"
              value={systems.length}
              prefix={<DatabaseOutlined />}
              valueStyle={{ fontSize: 20 }}
            />
          </Card>
        </Col>
        <Col xs={6}>
          <Card size="small">
            <Statistic
              title="Schemas"
              value={totalSchemas}
              prefix={<FolderOutlined />}
              valueStyle={{ fontSize: 20 }}
            />
          </Card>
        </Col>
        <Col xs={6}>
          <Card size="small">
            <Statistic
              title="Tables"
              value={totalTables}
              prefix={<TableOutlined />}
              valueStyle={{ fontSize: 20 }}
            />
          </Card>
        </Col>
        <Col xs={6}>
          <Card size="small">
            <Statistic
              title="Columns (selected)"
              value={totalColumns}
              valueStyle={{ fontSize: 20 }}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={16}>
        <Col xs={24} md={8} lg={6}>
          <Card
            title="Source Systems"
            size="small"
            style={{ height: 'calc(100vh - 320px)', overflow: 'auto' }}
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
                loadData={onLoadData}
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

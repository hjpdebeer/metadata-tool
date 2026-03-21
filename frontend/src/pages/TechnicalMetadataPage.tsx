import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Breadcrumb,
  Button,
  Card,
  Col,
  Descriptions,
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
import type { EventDataNode } from 'antd/es/tree';
import {
  ArrowLeftOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  DatabaseOutlined,
  FolderOutlined,
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
  const [, setSelectedSchema] = useState<string | null>(null);
  const [selectedSystem, setSelectedSystem] = useState<SourceSystem | null>(null);
  const [loadingSystems, setLoadingSystems] = useState(false);
  const [loadingColumns, setLoadingColumns] = useState(false);
  const [expandedKeys, setExpandedKeys] = useState<React.Key[]>([]);

  const treeNodeDataMapRef = useRef<Record<string, TreeNodeData>>({});
  const [loadedSystems, setLoadedSystems] = useState<Set<string>>(new Set());
  const [loadedSchemas, setLoadedSchemas] = useState<Set<string>>(new Set());

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

    if (nodeData.type === 'system' && nodeData.systemId) {
      const system = systems.find((s) => s.system_id === nodeData.systemId);
      if (system) {
        setSelectedSystem(system);
        setSelectedSchema(null);
        setSelectedTable(null);
        setColumns([]);
      }
    } else if (nodeData.type === 'schema' && nodeData.schemaId) {
      const schema = Object.values(schemasMap).flat().find((s) => s.schema_id === nodeData.schemaId);
      if (schema) {
        setSelectedSchema(schema.schema_name);
        setSelectedTable(null);
        setColumns([]);
      }
    } else if (nodeData.type === 'table' && nodeData.tableId) {
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

          return {
            key: tableKey,
            title: table.table_name,
            icon: <TableOutlined style={{ color: '#1B3A5C' }} />,
            isLeaf: true,
          };
        });

        return {
          key: schemaKey,
          title: schema.schema_name,
          icon: <FolderOutlined style={{ color: '#D4A017' }} />,
          children: schemaLoaded ? tableChildren : undefined,
          isLeaf: false,
        };
      });

      const envTag = system.environment
        ? ` (${system.environment.charAt(0) + system.environment.slice(1).toLowerCase()})`
        : '';

      return {
        key: systemKey,
        title: `${system.system_name}${envTag}`,
        icon: <DatabaseOutlined style={{ color: '#52C41A' }} />,
        children: systemLoaded ? schemaChildren : undefined,
        isLeaf: false,
      };
    });
  };

  const columnTableColumns = [
    {
      title: '#',
      dataIndex: 'ordinal_position',
      key: 'ordinal_position',
      width: 45,
      align: 'center' as const,
      render: (val: number) => <Text type="secondary" style={{ fontSize: 12 }}>{val}</Text>,
    },
    {
      title: 'Column Name',
      dataIndex: 'column_name',
      key: 'column_name',
      render: (name: string, record: TechnicalColumn) => (
        <Space size={4}>
          <Text code style={{ fontSize: 12 }}>{name}</Text>
          {record.is_primary_key && <Tag color="gold" style={{ fontSize: 10 }}>PK</Tag>}
          {record.is_foreign_key && <Tag color="blue" style={{ fontSize: 10 }}>FK</Tag>}
        </Space>
      ),
    },
    {
      title: 'Data Type',
      key: 'data_type',
      width: 140,
      render: (_: unknown, record: TechnicalColumn) => {
        let display = record.data_type;
        if (record.max_length) display += `(${record.max_length})`;
        else if (record.numeric_precision) {
          display += `(${record.numeric_precision}`;
          if (record.numeric_scale) display += `,${record.numeric_scale}`;
          display += ')';
        }
        return <Text style={{ fontSize: 12, fontFamily: 'monospace' }}>{display}</Text>;
      },
    },
    {
      title: 'Nullable',
      dataIndex: 'is_nullable',
      key: 'is_nullable',
      width: 70,
      align: 'center' as const,
      render: (val: boolean) => val ? <Text type="secondary">Yes</Text> : <Text strong>No</Text>,
    },
    {
      title: 'Linked Element',
      dataIndex: 'element_name',
      key: 'element_name',
      render: (name: string | null, record: TechnicalColumn) =>
        name ? (
          <a onClick={() => navigate(`/data-dictionary/${record.element_id}`)}>
            <Tag color="purple" style={{ cursor: 'pointer' }}><LinkOutlined /> {name}</Tag>
          </a>
        ) : (
          <Text type="secondary" style={{ fontSize: 12 }}>—</Text>
        ),
    },
    {
      title: 'Naming',
      key: 'naming_compliance',
      width: 100,
      align: 'center' as const,
      render: (_: unknown, record: TechnicalColumn) => {
        if (record.naming_standard_compliant === null || record.naming_standard_compliant === undefined) {
          return <Text type="secondary">—</Text>;
        }
        return record.naming_standard_compliant ? (
          <Tag icon={<CheckCircleOutlined />} color="success" style={{ fontSize: 11 }}>OK</Tag>
        ) : (
          <Tooltip title={record.naming_standard_violation || 'Naming violation'}>
            <Tag icon={<CloseCircleOutlined />} color="error" style={{ fontSize: 11 }}>Violation</Tag>
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

      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <Space align="center">
          <Button type="text" icon={<ArrowLeftOutlined />} onClick={() => navigate('/data-dictionary')} />
          <Title level={3} style={{ margin: 0 }}>Technical Metadata Browser</Title>
        </Space>
        <Text type="secondary">{systems.length} source system{systems.length !== 1 ? 's' : ''} registered</Text>
      </div>

      <Row gutter={16}>
        {/* Tree Panel — wider */}
        <Col xs={24} md={10} lg={8}>
          <Card
            size="small"
            title={<Text strong>Source Systems</Text>}
            style={{ height: 'calc(100vh - 180px)', overflow: 'auto' }}
          >
            {loadingSystems ? (
              <div style={{ textAlign: 'center', padding: 40 }}><Spin /></div>
            ) : systems.length === 0 ? (
              <Empty description="No source systems registered" image={Empty.PRESENTED_IMAGE_SIMPLE} />
            ) : (
              <Tree
                showIcon
                showLine={{ showLeafIcon: false }}
                treeData={treeData}
                expandedKeys={expandedKeys}
                onExpand={onTreeExpand}
                onSelect={onTreeSelect}
                loadData={onLoadData}
                style={{ fontSize: 13 }}
              />
            )}
          </Card>
        </Col>

        {/* Detail Panel */}
        <Col xs={24} md={14} lg={16}>
          {!selectedTable && !selectedSystem ? (
            <Card size="small" style={{ height: 'calc(100vh - 180px)' }}>
              <Empty
                description="Select a system, schema, or table from the tree"
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                style={{ paddingTop: 80 }}
              />
            </Card>
          ) : selectedTable ? (
            <Card
              size="small"
              title={
                <Space>
                  <TableOutlined />
                  <Text strong>{selectedTable.table_name}</Text>
                  <Tag color={selectedTable.table_type === 'VIEW' ? 'blue' : 'default'}>{selectedTable.table_type}</Tag>
                  {selectedTable.row_count != null && (
                    <Text type="secondary" style={{ fontSize: 12 }}>{selectedTable.row_count.toLocaleString()} rows</Text>
                  )}
                  {selectedTable.is_pii && <Tag color="red">PII</Tag>}
                </Space>
              }
              style={{ height: 'calc(100vh - 180px)', overflow: 'auto' }}
            >
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
                scroll={{ y: 'calc(100vh - 340px)' }}
              />
            </Card>
          ) : selectedSystem ? (
            <Card
              size="small"
              title={
                <Space>
                  <DatabaseOutlined />
                  <Text strong>{selectedSystem.system_name}</Text>
                </Space>
              }
              style={{ height: 'calc(100vh - 180px)', overflow: 'auto' }}
            >
              <Descriptions column={2} bordered size="small">
                <Descriptions.Item label="System Code">{selectedSystem.system_code}</Descriptions.Item>
                <Descriptions.Item label="System Type"><Tag>{selectedSystem.system_type}</Tag></Descriptions.Item>
                <Descriptions.Item label="Environment">
                  {selectedSystem.environment ? <Tag color="blue">{selectedSystem.environment}</Tag> : '—'}
                </Descriptions.Item>
                <Descriptions.Item label="Vendor">{selectedSystem.vendor || '—'}</Descriptions.Item>
                <Descriptions.Item label="Description" span={2}>{selectedSystem.description || '—'}</Descriptions.Item>
              </Descriptions>
            </Card>
          ) : null}
        </Col>
      </Row>
    </div>
  );
};

export default TechnicalMetadataPage;

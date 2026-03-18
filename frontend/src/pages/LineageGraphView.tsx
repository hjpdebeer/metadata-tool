import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  Breadcrumb,
  Button,
  Descriptions,
  Divider,
  Drawer,
  Empty,
  Form,
  Input,
  List,
  Modal,
  Radio,
  Select,
  Space,
  Spin,
  Tag,
  Tooltip,
  Typography,
  message,
} from 'antd';
import {
  ArrowLeftOutlined,
  PlusOutlined,
  ApartmentOutlined,
  NodeIndexOutlined,
  BranchesOutlined,
  ThunderboltOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import {
  ReactFlow,
  Controls,
  Background,
  MiniMap,
  Panel,
  useNodesState,
  useEdgesState,
  addEdge,
  MarkerType,
  BackgroundVariant,
} from '@xyflow/react';
import type {
  Connection,
  Node,
  Edge,
  OnNodeDrag,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { lineageApi } from '../services/lineageApi';
import type {
  LineageGraphView as LineageGraphViewType,
  LineageNodeView,
  LineageEdge,
  LineageNodeType,
  ImpactAnalysis,
  AddLineageNodeRequest,
  AddLineageEdgeRequest,
} from '../services/lineageApi';
import LineageCustomNode, { NODE_TYPE_COLORS } from '../components/lineage/LineageCustomNode';

const { Title, Text } = Typography;
const { TextArea } = Input;

// ---------------------------------------------------------------------------
// Custom node type registration
// ---------------------------------------------------------------------------

const nodeTypes = {
  lineageNode: LineageCustomNode,
};

// ---------------------------------------------------------------------------
// Edge type labels
// ---------------------------------------------------------------------------

const EDGE_TYPES = [
  { value: 'FLOW', label: 'Data Flow' },
  { value: 'DERIVES', label: 'Derives From' },
  { value: 'REFERENCES', label: 'References' },
  { value: 'TRANSFORMS', label: 'Transforms' },
  { value: 'FEEDS', label: 'Feeds Into' },
  { value: 'COPIES', label: 'Copies' },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const DEFAULT_NODE_COLOR = '#1B3A5C';

/** Convert API nodes to React Flow nodes */
function toFlowNodes(nodes: LineageNodeView[], impactedIds?: Set<string>): Node[] {
  return nodes.map((n) => ({
    id: n.node_id,
    type: 'lineageNode',
    position: { x: n.position_x ?? 0, y: n.position_y ?? 0 },
    data: {
      label: n.node_name,
      nodeType: n.node_type_code,
      nodeTypeName: n.node_type_name,
      description: n.description || undefined,
      iconName: n.icon_name || undefined,
      isImpacted: impactedIds?.has(n.node_id) || false,
    },
  }));
}

/** Convert API edges to React Flow edges */
function toFlowEdges(edges: LineageEdge[], impactedIds?: Set<string>): Edge[] {
  return edges.map((e) => {
    const isImpacted = impactedIds?.has(e.edge_id) || false;
    return {
      id: e.edge_id,
      source: e.source_node_id,
      target: e.target_node_id,
      label: e.transformation_logic || e.edge_type,
      markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
      animated: e.edge_type === 'FLOW',
      style: isImpacted
        ? { stroke: '#ff4d4f', strokeWidth: 3 }
        : { strokeWidth: 1.5 },
      labelStyle: { fontSize: 10, fill: '#6B7280' },
      labelBgStyle: { fill: '#ffffff', fillOpacity: 0.85 },
      labelBgPadding: [4, 2] as [number, number],
    };
  });
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const LineageGraphViewPage: React.FC = () => {
  const { id: graphId } = useParams<{ id: string }>();
  const navigate = useNavigate();

  // Graph data
  const [graphData, setGraphData] = useState<LineageGraphViewType | null>(null);
  const [loading, setLoading] = useState(true);
  const [nodeTypesList, setNodeTypesList] = useState<LineageNodeType[]>([]);

  // React Flow state
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([] as Node[]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([] as Edge[]);

  // Impact analysis state
  const [impactResult, setImpactResult] = useState<ImpactAnalysis | null>(null);
  const [impactDirection, setImpactDirection] = useState<'UPSTREAM' | 'DOWNSTREAM'>(
    'DOWNSTREAM',
  );
  const [impactLoading, setImpactLoading] = useState(false);
  const [impactNodeId, setImpactNodeId] = useState<string | null>(null);
  const [impactDrawerOpen, setImpactDrawerOpen] = useState(false);

  // Modals
  const [addNodeModalOpen, setAddNodeModalOpen] = useState(false);
  const [addEdgeModalOpen, setAddEdgeModalOpen] = useState(false);
  const [graphInfoOpen, setGraphInfoOpen] = useState(false);
  const [addingNode, setAddingNode] = useState(false);
  const [addingEdge, setAddingEdge] = useState(false);
  const [nodeForm] = Form.useForm();
  const [edgeForm] = Form.useForm();

  // Debounce timer for position updates
  const positionTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // -------------------------------------------------------------------------
  // Data fetching
  // -------------------------------------------------------------------------

  const fetchGraph = useCallback(async () => {
    if (!graphId) return;
    setLoading(true);
    try {
      const response = await lineageApi.getGraph(graphId);
      setGraphData(response.data);
      setNodes(toFlowNodes(response.data.nodes));
      setEdges(toFlowEdges(response.data.edges));
    } catch {
      message.error('Failed to load lineage graph.');
      navigate('/lineage');
    } finally {
      setLoading(false);
    }
  }, [graphId, navigate, setNodes, setEdges]);

  const fetchNodeTypes = useCallback(async () => {
    try {
      const response = await lineageApi.listNodeTypes();
      setNodeTypesList(response.data);
    } catch {
      // Non-critical: user can still view graph
    }
  }, []);

  useEffect(() => {
    fetchGraph();
  }, [fetchGraph]);

  useEffect(() => {
    fetchNodeTypes();
  }, [fetchNodeTypes]);

  // -------------------------------------------------------------------------
  // Node drag -> persist position (debounced)
  // -------------------------------------------------------------------------

  const onNodeDragStop: OnNodeDrag = useCallback(
    (_event, node) => {
      if (positionTimerRef.current) {
        clearTimeout(positionTimerRef.current);
      }
      positionTimerRef.current = setTimeout(() => {
        lineageApi
          .updateNodePosition(node.id, {
            position_x: node.position.x,
            position_y: node.position.y,
          })
          .catch(() => {
            // Silent fail — position will resync on next load
          });
      }, 500);
    },
    [],
  );

  // -------------------------------------------------------------------------
  // Edge creation via drag-connect
  // -------------------------------------------------------------------------

  const onConnect = useCallback(
    (connection: Connection) => {
      if (!graphId || !connection.source || !connection.target) return;

      const newEdge: Edge = {
        id: `temp-${Date.now()}`,
        source: connection.source,
        target: connection.target,
        label: 'FLOW',
        markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
        animated: true,
        style: { strokeWidth: 1.5 },
        labelStyle: { fontSize: 10, fill: '#6B7280' },
      };

      setEdges((eds) => addEdge(newEdge, eds));

      // Persist to backend
      lineageApi
        .addEdge(graphId, {
          source_node_id: connection.source,
          target_node_id: connection.target,
          edge_type: 'FLOW',
        })
        .then((response) => {
          // Replace temp edge with real one
          setEdges((eds) =>
            eds.map((e) =>
              e.id === newEdge.id
                ? {
                    ...e,
                    id: response.data.edge_id,
                  }
                : e,
            ),
          );
          message.success('Edge created.');
        })
        .catch(() => {
          // Remove temp edge on failure
          setEdges((eds) => eds.filter((e) => e.id !== newEdge.id));
          message.error('Failed to create edge.');
        });
    },
    [graphId, setEdges],
  );

  // -------------------------------------------------------------------------
  // Add node
  // -------------------------------------------------------------------------

  const handleAddNode = async (values: {
    node_type_id: string;
    node_name: string;
    description?: string;
  }) => {
    if (!graphId) return;
    setAddingNode(true);

    // Place new nodes in the center-ish of the viewport
    const centerX = 300 + Math.random() * 200;
    const centerY = 200 + Math.random() * 200;

    try {
      const request: AddLineageNodeRequest = {
        node_type_id: values.node_type_id,
        node_name: values.node_name,
        description: values.description || undefined,
        position_x: centerX,
        position_y: centerY,
      };

      const response = await lineageApi.addNode(graphId, request);
      const newNode = response.data;

      // Find the matching node type for display
      const nodeType = nodeTypesList.find((nt) => nt.node_type_id === newNode.node_type_id);

      const flowNode: Node = {
        id: newNode.node_id,
        type: 'lineageNode',
        position: { x: newNode.position_x ?? centerX, y: newNode.position_y ?? centerY },
        data: {
          label: newNode.node_name,
          nodeType: nodeType?.type_code || '',
          nodeTypeName: nodeType?.type_name || '',
          description: newNode.description || undefined,
          isImpacted: false,
        },
      };

      setNodes((nds) => [...nds, flowNode]);
      message.success('Node added.');
      setAddNodeModalOpen(false);
      nodeForm.resetFields();
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { message?: string } } };
      message.error(axiosErr.response?.data?.message || 'Failed to add node.');
    } finally {
      setAddingNode(false);
    }
  };

  // -------------------------------------------------------------------------
  // Add edge via modal
  // -------------------------------------------------------------------------

  const handleAddEdge = async (values: {
    source_node_id: string;
    target_node_id: string;
    edge_type: string;
    transformation_logic?: string;
    description?: string;
  }) => {
    if (!graphId) return;
    setAddingEdge(true);
    try {
      const request: AddLineageEdgeRequest = {
        source_node_id: values.source_node_id,
        target_node_id: values.target_node_id,
        edge_type: values.edge_type,
        transformation_logic: values.transformation_logic || undefined,
        description: values.description || undefined,
      };

      const response = await lineageApi.addEdge(graphId, request);
      const newEdge = response.data;

      const flowEdge: Edge = {
        id: newEdge.edge_id,
        source: newEdge.source_node_id,
        target: newEdge.target_node_id,
        label: newEdge.transformation_logic || newEdge.edge_type,
        markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
        animated: newEdge.edge_type === 'FLOW',
        style: { strokeWidth: 1.5 },
        labelStyle: { fontSize: 10, fill: '#6B7280' },
        labelBgStyle: { fill: '#ffffff', fillOpacity: 0.85 },
        labelBgPadding: [4, 2] as [number, number],
      };

      setEdges((eds) => [...eds, flowEdge]);
      message.success('Edge created.');
      setAddEdgeModalOpen(false);
      edgeForm.resetFields();
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { message?: string } } };
      message.error(axiosErr.response?.data?.message || 'Failed to add edge.');
    } finally {
      setAddingEdge(false);
    }
  };

  // -------------------------------------------------------------------------
  // Impact analysis
  // -------------------------------------------------------------------------

  const runImpactAnalysis = useCallback(
    async (nodeId: string, direction: 'UPSTREAM' | 'DOWNSTREAM') => {
      setImpactLoading(true);
      setImpactNodeId(nodeId);
      setImpactDirection(direction);
      try {
        const response = await lineageApi.impactAnalysis(nodeId, direction);
        setImpactResult(response.data);

        // Highlight impacted nodes and edges
        const impactedNodeIds = new Set(response.data.impacted_nodes.map((n) => n.node_id));
        impactedNodeIds.add(nodeId); // Include the source node
        const impactedEdgeIds = new Set(response.data.impacted_edges.map((e) => e.edge_id));

        if (graphData) {
          setNodes(toFlowNodes(graphData.nodes, impactedNodeIds));
          setEdges(toFlowEdges(graphData.edges, impactedEdgeIds));
        }

        setImpactDrawerOpen(true);
      } catch {
        message.error('Failed to run impact analysis.');
      } finally {
        setImpactLoading(false);
      }
    },
    [graphData, setNodes, setEdges],
  );

  const clearImpactAnalysis = useCallback(() => {
    setImpactResult(null);
    setImpactNodeId(null);
    setImpactDrawerOpen(false);
    if (graphData) {
      setNodes(toFlowNodes(graphData.nodes));
      setEdges(toFlowEdges(graphData.edges));
    }
  }, [graphData, setNodes, setEdges]);

  // -------------------------------------------------------------------------
  // Node options for edge form
  // -------------------------------------------------------------------------

  const nodeOptions = useMemo(
    () =>
      nodes.map((n) => ({
        value: n.id,
        label: (n.data as { label?: string })?.label || n.id,
      })),
    [nodes],
  );

  const nodeTypeOptions = useMemo(
    () =>
      nodeTypesList.map((nt) => ({
        value: nt.node_type_id,
        label: nt.type_name,
      })),
    [nodeTypesList],
  );

  // -------------------------------------------------------------------------
  // MiniMap node color
  // -------------------------------------------------------------------------

  const miniMapNodeColor = useCallback((node: Node) => {
    const nodeType = (node.data as { nodeType?: string })?.nodeType || '';
    return NODE_TYPE_COLORS[nodeType] || DEFAULT_NODE_COLOR;
  }, []);

  // -------------------------------------------------------------------------
  // Selected node for context actions
  // -------------------------------------------------------------------------

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  const onNodeClick = useCallback((_event: React.MouseEvent, node: Node) => {
    setSelectedNodeId(node.id);
  }, []);

  const onPaneClick = useCallback(() => {
    setSelectedNodeId(null);
  }, []);

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  if (loading) {
    return (
      <div style={{ textAlign: 'center', padding: 80 }}>
        <Spin size="large" />
      </div>
    );
  }

  if (!graphData) {
    return (
      <div style={{ textAlign: 'center', padding: 80 }}>
        <Empty description="Graph not found" />
        <Button type="link" onClick={() => navigate('/lineage')}>
          Back to Lineage
        </Button>
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: 'calc(100vh - 112px)' }}>
      {/* Header row */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 12,
          flexShrink: 0,
        }}
      >
        <Space>
          <Button
            type="text"
            icon={<ArrowLeftOutlined />}
            onClick={() => navigate('/lineage')}
          />
          <div>
            <Breadcrumb
              style={{ marginBottom: 2 }}
              items={[
                {
                  title: (
                    <a onClick={() => navigate('/lineage')}>Data Lineage</a>
                  ),
                },
                { title: graphData.graph_name },
              ]}
            />
            <Space size="small">
              <Title level={4} style={{ margin: 0 }}>
                {graphData.graph_name}
              </Title>
              <Tag color={graphData.graph_type === 'BUSINESS' ? 'blue' : 'green'}>
                {graphData.graph_type}
              </Tag>
              {graphData.is_current && <Tag color="success">Current</Tag>}
            </Space>
          </div>
        </Space>

        <Space>
          <Tooltip title="Graph Info">
            <Button
              icon={<InfoCircleOutlined />}
              onClick={() => setGraphInfoOpen(true)}
            />
          </Tooltip>
          <Button
            icon={<NodeIndexOutlined />}
            onClick={() => setAddNodeModalOpen(true)}
          >
            Add Node
          </Button>
          <Button
            icon={<BranchesOutlined />}
            onClick={() => setAddEdgeModalOpen(true)}
          >
            Add Edge
          </Button>
          {selectedNodeId && (
            <Button
              icon={<ThunderboltOutlined />}
              type="primary"
              loading={impactLoading}
              onClick={() => runImpactAnalysis(selectedNodeId, impactDirection)}
            >
              Impact Analysis
            </Button>
          )}
          {impactResult && (
            <Button danger onClick={clearImpactAnalysis}>
              Clear Impact
            </Button>
          )}
        </Space>
      </div>

      {/* React Flow canvas */}
      <div
        style={{
          flex: 1,
          borderRadius: 8,
          border: '1px solid #E5E7EB',
          overflow: 'hidden',
          background: '#fafafa',
        }}
      >
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          onNodeDragStop={onNodeDragStop}
          onNodeClick={onNodeClick}
          onPaneClick={onPaneClick}
          nodeTypes={nodeTypes}
          fitView
          fitViewOptions={{ padding: 0.2 }}
          minZoom={0.1}
          maxZoom={2}
          defaultEdgeOptions={{
            type: 'smoothstep',
          }}
          proOptions={{ hideAttribution: true }}
        >
          <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#d0d0d0" />
          <Controls
            showInteractive={false}
            style={{ borderRadius: 8, border: '1px solid #E5E7EB' }}
          />
          <MiniMap
            nodeColor={miniMapNodeColor}
            nodeStrokeWidth={2}
            style={{
              borderRadius: 8,
              border: '1px solid #E5E7EB',
              background: '#ffffff',
            }}
            maskColor="rgba(27, 58, 92, 0.08)"
          />

          {/* Floating toolbar panel */}
          <Panel position="top-right">
            <div
              style={{
                background: 'rgba(255, 255, 255, 0.92)',
                backdropFilter: 'blur(8px)',
                borderRadius: 8,
                border: '1px solid #E5E7EB',
                padding: '8px 12px',
                fontSize: 12,
                color: '#6B7280',
                maxWidth: 220,
              }}
            >
              <div style={{ fontWeight: 600, color: '#1F2937', marginBottom: 4 }}>
                Graph Controls
              </div>
              <div style={{ marginBottom: 2 }}>
                <strong>{graphData.nodes.length}</strong> nodes,{' '}
                <strong>{graphData.edges.length}</strong> edges
              </div>
              <Divider style={{ margin: '6px 0' }} />
              <div>Drag between handles to connect nodes</div>
              <div>Click a node, then use Impact Analysis</div>
              {selectedNodeId && (
                <>
                  <Divider style={{ margin: '6px 0' }} />
                  <div>
                    <Text strong style={{ fontSize: 12 }}>
                      Selected:{' '}
                    </Text>
                    <Text style={{ fontSize: 12 }}>
                      {(nodes.find((n) => n.id === selectedNodeId)?.data as { label?: string })
                        ?.label || selectedNodeId}
                    </Text>
                  </div>
                  <Space style={{ marginTop: 4 }}>
                    <Radio.Group
                      size="small"
                      value={impactDirection}
                      onChange={(e) => setImpactDirection(e.target.value)}
                    >
                      <Radio.Button value="DOWNSTREAM">Downstream</Radio.Button>
                      <Radio.Button value="UPSTREAM">Upstream</Radio.Button>
                    </Radio.Group>
                  </Space>
                </>
              )}
            </div>
          </Panel>

          {/* Empty state overlay */}
          {nodes.length === 0 && (
            <Panel position="top-center">
              <div
                style={{
                  background: 'rgba(255,255,255,0.95)',
                  borderRadius: 8,
                  padding: '24px 32px',
                  textAlign: 'center',
                  marginTop: 120,
                  border: '1px solid #E5E7EB',
                }}
              >
                <ApartmentOutlined style={{ fontSize: 48, color: '#d9d9d9', display: 'block', marginBottom: 12 }} />
                <Title level={5} style={{ margin: 0 }}>
                  This graph is empty
                </Title>
                <Text type="secondary" style={{ display: 'block', marginBottom: 12 }}>
                  Start by adding nodes to build your data lineage
                </Text>
                <Button
                  type="primary"
                  icon={<PlusOutlined />}
                  onClick={() => setAddNodeModalOpen(true)}
                >
                  Add First Node
                </Button>
              </div>
            </Panel>
          )}
        </ReactFlow>
      </div>

      {/* ================================================================= */}
      {/* Add Node Modal */}
      {/* ================================================================= */}

      <Modal
        title="Add Node"
        open={addNodeModalOpen}
        onCancel={() => {
          setAddNodeModalOpen(false);
          nodeForm.resetFields();
        }}
        footer={null}
        destroyOnClose
      >
        <Form
          form={nodeForm}
          layout="vertical"
          onFinish={handleAddNode}
          style={{ marginTop: 16 }}
        >
          <Form.Item
            name="node_type_id"
            label="Node Type"
            rules={[{ required: true, message: 'Node type is required' }]}
          >
            <Select
              placeholder="Select node type"
              options={nodeTypeOptions}
              showSearch
              optionFilterProp="label"
            />
          </Form.Item>
          <Form.Item
            name="node_name"
            label="Node Name"
            rules={[
              { required: true, message: 'Node name is required' },
              { max: 256, message: 'Node name cannot exceed 256 characters' },
            ]}
          >
            <Input placeholder="e.g., Customer Database, ETL Pipeline" />
          </Form.Item>
          <Form.Item name="description" label="Description">
            <TextArea rows={3} placeholder="Describe this node" />
          </Form.Item>
          <Form.Item style={{ marginBottom: 0, textAlign: 'right' }}>
            <Space>
              <Button
                onClick={() => {
                  setAddNodeModalOpen(false);
                  nodeForm.resetFields();
                }}
              >
                Cancel
              </Button>
              <Button type="primary" htmlType="submit" loading={addingNode}>
                Add Node
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* ================================================================= */}
      {/* Add Edge Modal */}
      {/* ================================================================= */}

      <Modal
        title="Add Edge"
        open={addEdgeModalOpen}
        onCancel={() => {
          setAddEdgeModalOpen(false);
          edgeForm.resetFields();
        }}
        footer={null}
        destroyOnClose
      >
        <Form
          form={edgeForm}
          layout="vertical"
          onFinish={handleAddEdge}
          style={{ marginTop: 16 }}
        >
          <Form.Item
            name="source_node_id"
            label="Source Node"
            rules={[{ required: true, message: 'Source node is required' }]}
          >
            <Select
              placeholder="Select source node"
              options={nodeOptions}
              showSearch
              optionFilterProp="label"
            />
          </Form.Item>
          <Form.Item
            name="target_node_id"
            label="Target Node"
            rules={[{ required: true, message: 'Target node is required' }]}
          >
            <Select
              placeholder="Select target node"
              options={nodeOptions}
              showSearch
              optionFilterProp="label"
            />
          </Form.Item>
          <Form.Item
            name="edge_type"
            label="Edge Type"
            rules={[{ required: true, message: 'Edge type is required' }]}
            initialValue="FLOW"
          >
            <Select placeholder="Select edge type" options={EDGE_TYPES} />
          </Form.Item>
          <Form.Item name="transformation_logic" label="Transformation Logic">
            <TextArea
              rows={2}
              placeholder="e.g., JOIN on customer_id, filtered by status='ACTIVE'"
            />
          </Form.Item>
          <Form.Item name="description" label="Description">
            <TextArea rows={2} placeholder="Describe this relationship" />
          </Form.Item>
          <Form.Item style={{ marginBottom: 0, textAlign: 'right' }}>
            <Space>
              <Button
                onClick={() => {
                  setAddEdgeModalOpen(false);
                  edgeForm.resetFields();
                }}
              >
                Cancel
              </Button>
              <Button type="primary" htmlType="submit" loading={addingEdge}>
                Add Edge
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* ================================================================= */}
      {/* Impact Analysis Drawer */}
      {/* ================================================================= */}

      <Drawer
        title={
          <Space>
            <ThunderboltOutlined style={{ color: '#ff4d4f' }} />
            <span>Impact Analysis</span>
          </Space>
        }
        open={impactDrawerOpen}
        onClose={clearImpactAnalysis}
        width={380}
        extra={
          <Radio.Group
            size="small"
            value={impactDirection}
            onChange={(e) => {
              if (impactNodeId) {
                runImpactAnalysis(impactNodeId, e.target.value);
              }
            }}
          >
            <Radio.Button value="DOWNSTREAM">Downstream</Radio.Button>
            <Radio.Button value="UPSTREAM">Upstream</Radio.Button>
          </Radio.Group>
        }
      >
        {impactResult && (
          <div>
            <Descriptions column={1} size="small" style={{ marginBottom: 16 }}>
              <Descriptions.Item label="Direction">
                <Tag color={impactResult.direction === 'DOWNSTREAM' ? 'orange' : 'blue'}>
                  {impactResult.direction}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label="Max Depth">
                {impactResult.max_depth_reached}
              </Descriptions.Item>
              <Descriptions.Item label="Impacted Nodes">
                <Text strong>{impactResult.impacted_nodes.length}</Text>
              </Descriptions.Item>
              <Descriptions.Item label="Impacted Edges">
                <Text strong>{impactResult.impacted_edges.length}</Text>
              </Descriptions.Item>
            </Descriptions>

            <Divider orientation="left" plain style={{ fontSize: 13 }}>
              Impacted Nodes
            </Divider>

            {impactResult.impacted_nodes.length === 0 ? (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description="No impacted nodes found"
              />
            ) : (
              <List
                size="small"
                dataSource={impactResult.impacted_nodes}
                renderItem={(node) => (
                  <List.Item>
                    <List.Item.Meta
                      avatar={
                        <div
                          style={{
                            width: 8,
                            height: 8,
                            borderRadius: '50%',
                            marginTop: 6,
                            background:
                              NODE_TYPE_COLORS[node.node_type_code] || DEFAULT_NODE_COLOR,
                          }}
                        />
                      }
                      title={
                        <Space size={4}>
                          <span style={{ fontSize: 13 }}>{node.node_name}</span>
                          <Tag
                            style={{ fontSize: 10, lineHeight: '16px' }}
                            color={
                              NODE_TYPE_COLORS[node.node_type_code]
                                ? undefined
                                : 'default'
                            }
                          >
                            {node.node_type_name}
                          </Tag>
                        </Space>
                      }
                      description={
                        <Text type="secondary" style={{ fontSize: 11 }}>
                          Depth: {node.depth}
                          {node.description ? ` - ${node.description}` : ''}
                        </Text>
                      }
                    />
                  </List.Item>
                )}
              />
            )}
          </div>
        )}
      </Drawer>

      {/* ================================================================= */}
      {/* Graph Info Drawer */}
      {/* ================================================================= */}

      <Drawer
        title="Graph Details"
        open={graphInfoOpen}
        onClose={() => setGraphInfoOpen(false)}
        width={400}
      >
        <Descriptions column={1} bordered size="small">
          <Descriptions.Item label="Name">{graphData.graph_name}</Descriptions.Item>
          <Descriptions.Item label="Type">
            <Tag color={graphData.graph_type === 'BUSINESS' ? 'blue' : 'green'}>
              {graphData.graph_type}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Description">
            {graphData.description || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="Version">{graphData.version_number}</Descriptions.Item>
          <Descriptions.Item label="Current">
            {graphData.is_current ? 'Yes' : 'No'}
          </Descriptions.Item>
          <Descriptions.Item label="Nodes">{graphData.nodes.length}</Descriptions.Item>
          <Descriptions.Item label="Edges">{graphData.edges.length}</Descriptions.Item>
          <Descriptions.Item label="Created">
            {new Date(graphData.created_at).toLocaleDateString('en-ZA', {
              year: 'numeric',
              month: 'short',
              day: 'numeric',
              hour: '2-digit',
              minute: '2-digit',
            })}
          </Descriptions.Item>
          <Descriptions.Item label="Updated">
            {new Date(graphData.updated_at).toLocaleDateString('en-ZA', {
              year: 'numeric',
              month: 'short',
              day: 'numeric',
              hour: '2-digit',
              minute: '2-digit',
            })}
          </Descriptions.Item>
        </Descriptions>

        {graphData.nodes.length > 0 && (
          <>
            <Divider orientation="left" plain style={{ fontSize: 13 }}>
              Node Types
            </Divider>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
              {Object.entries(
                graphData.nodes.reduce(
                  (acc, n) => {
                    const key = n.node_type_code;
                    acc[key] = (acc[key] || 0) + 1;
                    return acc;
                  },
                  {} as Record<string, number>,
                ),
              ).map(([code, count]) => (
                <Tag key={code} color={NODE_TYPE_COLORS[code] || DEFAULT_NODE_COLOR}>
                  {code.replace(/_/g, ' ')}: {count}
                </Tag>
              ))}
            </div>
          </>
        )}
      </Drawer>
    </div>
  );
};

export default LineageGraphViewPage;

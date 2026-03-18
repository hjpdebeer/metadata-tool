import type { AxiosResponse } from 'axios';
import api from './api';

// ---------------------------------------------------------------------------
// Core entities
// ---------------------------------------------------------------------------

export interface LineageGraph {
  graph_id: string;
  graph_name: string;
  graph_type: string; // BUSINESS | TECHNICAL
  description: string | null;
  scope_type: string | null;
  scope_entity_id: string | null;
  version_number: number;
  is_current: boolean;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface LineageGraphListItem {
  graph_id: string;
  graph_name: string;
  graph_type: string;
  description: string | null;
  is_current: boolean;
  node_count: number;
  edge_count: number;
  created_by_name: string | null;
  created_at: string;
  updated_at: string;
}

export interface LineageNode {
  node_id: string;
  graph_id: string;
  node_type_id: string;
  node_name: string;
  node_label: string | null;
  description: string | null;
  system_id: string | null;
  table_id: string | null;
  element_id: string | null;
  application_id: string | null;
  process_id: string | null;
  position_x: number | null;
  position_y: number | null;
  properties: Record<string, unknown> | null;
}

export interface LineageNodeView extends LineageNode {
  node_type_code: string;
  node_type_name: string;
  icon_name: string | null;
}

export interface LineageEdge {
  edge_id: string;
  graph_id: string;
  source_node_id: string;
  target_node_id: string;
  edge_type: string;
  transformation_logic: string | null;
  description: string | null;
  properties: Record<string, unknown> | null;
}

export interface LineageGraphView extends LineageGraph {
  nodes: LineageNodeView[];
  edges: LineageEdge[];
}

export interface LineageNodeType {
  node_type_id: string;
  type_code: string;
  type_name: string;
  description: string | null;
  icon_name: string | null;
}

// ---------------------------------------------------------------------------
// Impact analysis
// ---------------------------------------------------------------------------

export interface ImpactedNode {
  node_id: string;
  graph_id: string;
  node_type_id: string;
  node_name: string;
  node_label: string | null;
  description: string | null;
  node_type_code: string;
  node_type_name: string;
  icon_name: string | null;
  depth: number;
}

export interface ImpactedEdge {
  edge_id: string;
  graph_id: string;
  source_node_id: string;
  target_node_id: string;
  edge_type: string;
  transformation_logic: string | null;
  description: string | null;
  depth: number;
}

export interface ImpactAnalysis {
  source_node_id: string;
  direction: string; // UPSTREAM | DOWNSTREAM
  impacted_nodes: ImpactedNode[];
  impacted_edges: ImpactedEdge[];
  max_depth_reached: number;
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

export interface CreateLineageGraphRequest {
  graph_name: string;
  graph_type: string;
  description?: string;
  scope_type?: string;
  scope_entity_id?: string;
}

export interface AddLineageNodeRequest {
  node_type_id: string;
  node_name: string;
  node_label?: string;
  description?: string;
  system_id?: string;
  table_id?: string;
  element_id?: string;
  position_x?: number;
  position_y?: number;
  properties?: Record<string, unknown>;
}

export interface AddLineageEdgeRequest {
  source_node_id: string;
  target_node_id: string;
  edge_type: string;
  transformation_logic?: string;
  description?: string;
}

export interface UpdateNodePositionRequest {
  position_x: number;
  position_y: number;
}

export interface ListGraphsParams {
  query?: string;
  graph_type?: string;
  page?: number;
  page_size?: number;
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

export const lineageApi = {
  listGraphs(params?: ListGraphsParams): Promise<AxiosResponse<LineageGraphListItem[]>> {
    return api.get('/lineage/graphs', { params });
  },

  getGraph(graphId: string): Promise<AxiosResponse<LineageGraphView>> {
    return api.get(`/lineage/graphs/${graphId}`);
  },

  createGraph(data: CreateLineageGraphRequest): Promise<AxiosResponse<LineageGraph>> {
    return api.post('/lineage/graphs', data);
  },

  addNode(graphId: string, data: AddLineageNodeRequest): Promise<AxiosResponse<LineageNode>> {
    return api.post(`/lineage/graphs/${graphId}/nodes`, data);
  },

  addEdge(graphId: string, data: AddLineageEdgeRequest): Promise<AxiosResponse<LineageEdge>> {
    return api.post(`/lineage/graphs/${graphId}/edges`, data);
  },

  updateNodePosition(
    nodeId: string,
    data: UpdateNodePositionRequest,
  ): Promise<AxiosResponse<void>> {
    return api.put(`/lineage/nodes/${nodeId}/position`, data);
  },

  deleteNode(graphId: string, nodeId: string): Promise<AxiosResponse<void>> {
    return api.delete(`/lineage/graphs/${graphId}/nodes/${nodeId}`);
  },

  deleteEdge(graphId: string, edgeId: string): Promise<AxiosResponse<void>> {
    return api.delete(`/lineage/graphs/${graphId}/edges/${edgeId}`);
  },

  listNodeTypes(): Promise<AxiosResponse<LineageNodeType[]>> {
    return api.get('/lineage/node-types');
  },

  impactAnalysis(
    nodeId: string,
    direction?: string,
    maxDepth?: number,
  ): Promise<AxiosResponse<ImpactAnalysis>> {
    return api.get(`/lineage/impact/${nodeId}`, {
      params: { direction, max_depth: maxDepth },
    });
  },
};

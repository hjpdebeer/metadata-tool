import React, { memo } from 'react';
import { Handle, Position } from '@xyflow/react';
import type { NodeProps } from '@xyflow/react';
import {
  DatabaseOutlined,
  ApiOutlined,
  CloudServerOutlined,
  TableOutlined,
  ThunderboltOutlined,
  BarChartOutlined,
  AppstoreOutlined,
  PartitionOutlined,
  NodeIndexOutlined,
} from '@ant-design/icons';

// ---------------------------------------------------------------------------
// Node type colour mapping
// ---------------------------------------------------------------------------

export const NODE_TYPE_COLORS: Record<string, string> = {
  SOURCE_SYSTEM: '#4CAF50',
  DATABASE: '#2196F3',
  TABLE: '#03A9F4',
  API: '#9C27B0',
  ETL_JOB: '#FF9800',
  REPORT: '#E91E63',
  APPLICATION: '#607D8B',
  PROCESS: '#795548',
};

const DEFAULT_NODE_COLOR = '#1B3A5C';

// ---------------------------------------------------------------------------
// Icon mapping
// ---------------------------------------------------------------------------

const NODE_TYPE_ICONS: Record<string, React.ReactNode> = {
  SOURCE_SYSTEM: <CloudServerOutlined style={{ fontSize: 14 }} />,
  DATABASE: <DatabaseOutlined style={{ fontSize: 14 }} />,
  TABLE: <TableOutlined style={{ fontSize: 14 }} />,
  API: <ApiOutlined style={{ fontSize: 14 }} />,
  ETL_JOB: <ThunderboltOutlined style={{ fontSize: 14 }} />,
  REPORT: <BarChartOutlined style={{ fontSize: 14 }} />,
  APPLICATION: <AppstoreOutlined style={{ fontSize: 14 }} />,
  PROCESS: <PartitionOutlined style={{ fontSize: 14 }} />,
};

// ---------------------------------------------------------------------------
// Node data shape
// ---------------------------------------------------------------------------

export interface LineageNodeData {
  label: string;
  nodeType: string;
  nodeTypeName?: string;
  description?: string;
  iconName?: string;
  isImpacted?: boolean;
  [key: string]: unknown;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const LineageCustomNode: React.FC<NodeProps> = ({ data, selected }) => {
  const nodeData = data as unknown as LineageNodeData;
  const nodeType = nodeData.nodeType || '';
  const color = NODE_TYPE_COLORS[nodeType] || DEFAULT_NODE_COLOR;
  const icon = NODE_TYPE_ICONS[nodeType] || <NodeIndexOutlined style={{ fontSize: 14 }} />;
  const isImpacted = nodeData.isImpacted || false;

  return (
    <div
      style={{
        minWidth: 180,
        maxWidth: 260,
        borderRadius: 8,
        overflow: 'hidden',
        border: isImpacted
          ? '2px solid #ff4d4f'
          : selected
            ? `2px solid ${color}`
            : '1px solid #d9d9d9',
        boxShadow: isImpacted
          ? '0 0 12px rgba(255, 77, 79, 0.5)'
          : selected
            ? `0 0 8px ${color}40`
            : '0 1px 4px rgba(0, 0, 0, 0.08)',
        background: '#ffffff',
        transition: 'box-shadow 0.3s, border-color 0.3s',
      }}
    >
      {/* Header */}
      <div
        style={{
          background: color,
          padding: '6px 12px',
          display: 'flex',
          alignItems: 'center',
          gap: 6,
        }}
      >
        <span style={{ color: '#ffffff', display: 'flex', alignItems: 'center' }}>
          {icon}
        </span>
        <span
          style={{
            color: '#ffffff',
            fontSize: 10,
            fontWeight: 600,
            textTransform: 'uppercase',
            letterSpacing: '0.5px',
          }}
        >
          {nodeData.nodeTypeName || nodeType.replace(/_/g, ' ')}
        </span>
      </div>

      {/* Body */}
      <div style={{ padding: '8px 12px' }}>
        <div
          style={{
            fontSize: 13,
            fontWeight: 600,
            color: '#1F2937',
            lineHeight: '1.3',
            wordBreak: 'break-word',
          }}
        >
          {nodeData.label}
        </div>
        {nodeData.description && (
          <div
            style={{
              fontSize: 11,
              color: '#6B7280',
              marginTop: 4,
              lineHeight: '1.3',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              display: '-webkit-box',
              WebkitLineClamp: 2,
              WebkitBoxOrient: 'vertical',
            }}
          >
            {nodeData.description}
          </div>
        )}
      </div>

      {/* Handles */}
      <Handle
        type="target"
        position={Position.Left}
        style={{
          width: 8,
          height: 8,
          background: color,
          border: '2px solid #ffffff',
        }}
      />
      <Handle
        type="source"
        position={Position.Right}
        style={{
          width: 8,
          height: 8,
          background: color,
          border: '2px solid #ffffff',
        }}
      />
    </div>
  );
};

export default memo(LineageCustomNode);

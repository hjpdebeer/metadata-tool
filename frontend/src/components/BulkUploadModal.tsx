import React, { useState } from 'react';
import {
  Alert,
  Button,
  Modal,
  Space,
  Table,
  Typography,
  Upload,
  message,
} from 'antd';
import {
  CheckCircleOutlined,
  DownloadOutlined,
  InboxOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import type { UploadFile } from 'antd';
import { glossaryApi } from '../services/glossaryApi';
import type { BulkUploadError, BulkUploadResult } from '../services/glossaryApi';

const { Text, Title } = Typography;
const { Dragger } = Upload;

interface BulkUploadModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

const BulkUploadModal: React.FC<BulkUploadModalProps> = ({
  open,
  onClose,
  onSuccess,
}) => {
  const [uploading, setUploading] = useState(false);
  const [result, setResult] = useState<BulkUploadResult | null>(null);
  const [fileList, setFileList] = useState<UploadFile[]>([]);
  const [downloadingTemplate, setDownloadingTemplate] = useState(false);

  const handleDownloadTemplate = async () => {
    setDownloadingTemplate(true);
    try {
      await glossaryApi.downloadBulkUploadTemplate();
      message.success('Template downloaded.');
    } catch {
      message.error('Failed to download template.');
    } finally {
      setDownloadingTemplate(false);
    }
  };

  const handleUpload = async () => {
    if (fileList.length === 0) {
      message.warning('Please select a file first.');
      return;
    }

    const file = fileList[0]?.originFileObj;
    if (!file) {
      message.warning('No file selected.');
      return;
    }

    setUploading(true);
    setResult(null);
    try {
      const response = await glossaryApi.uploadBulkTerms(file);
      setResult(response.data);
      if (response.data.successful > 0) {
        onSuccess();
      }
      if (response.data.failed === 0) {
        message.success(
          `All ${response.data.successful} terms uploaded successfully.`
        );
      } else if (response.data.successful > 0) {
        message.warning(
          `${response.data.successful} of ${response.data.total_rows} terms uploaded. ${response.data.failed} failed.`
        );
      } else {
        message.error('All rows failed validation. See errors below.');
      }
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { error?: { message?: string } } } };
      const errorMessage =
        axiosErr?.response?.data?.error?.message || 'Upload failed. Please check the file and try again.';
      message.error(errorMessage);
    } finally {
      setUploading(false);
    }
  };

  const handleClose = () => {
    setResult(null);
    setFileList([]);
    onClose();
  };

  const errorColumns = [
    {
      title: 'Row',
      dataIndex: 'row',
      key: 'row',
      width: 70,
      sorter: (a: BulkUploadError, b: BulkUploadError) => a.row - b.row,
    },
    {
      title: 'Field',
      dataIndex: 'field',
      key: 'field',
      width: 180,
      render: (field: string | null) => field || '-',
    },
    {
      title: 'Error',
      dataIndex: 'message',
      key: 'message',
    },
  ];

  return (
    <Modal
      title="Bulk Upload Glossary Terms"
      open={open}
      onCancel={handleClose}
      width={720}
      footer={[
        <Button key="close" onClick={handleClose}>
          Close
        </Button>,
      ]}
    >
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        {/* Section 1: Download Template */}
        <div>
          <Title level={5} style={{ marginTop: 0, marginBottom: 8 }}>
            Step 1: Download Template
          </Title>
          <Text type="secondary">
            Download the Excel template with instructions and valid dropdown
            values. Fill in your terms, then upload below.
          </Text>
          <div style={{ marginTop: 12 }}>
            <Button
              icon={<DownloadOutlined />}
              onClick={handleDownloadTemplate}
              loading={downloadingTemplate}
            >
              Download Template
            </Button>
          </div>
        </div>

        {/* Section 2: Upload File */}
        <div>
          <Title level={5} style={{ marginTop: 0, marginBottom: 8 }}>
            Step 2: Upload Filled Template
          </Title>
          <Text type="secondary">
            Max file size: 10 MB. Max rows: 1,000. Only .xlsx files accepted.
          </Text>
          <div style={{ marginTop: 12 }}>
            <Dragger
              accept=".xlsx"
              maxCount={1}
              fileList={fileList}
              beforeUpload={() => false}
              onChange={(info) => setFileList(info.fileList)}
              disabled={uploading}
            >
              <p className="ant-upload-drag-icon">
                <InboxOutlined />
              </p>
              <p className="ant-upload-text">
                Click or drag an .xlsx file to this area
              </p>
            </Dragger>
            <div style={{ marginTop: 12, textAlign: 'right' }}>
              <Button
                type="primary"
                onClick={handleUpload}
                loading={uploading}
                disabled={fileList.length === 0}
              >
                {uploading ? 'Uploading...' : 'Upload'}
              </Button>
            </div>
          </div>
        </div>

        {/* Section 3: Results */}
        {result && (
          <div>
            <Title level={5} style={{ marginTop: 0, marginBottom: 8 }}>
              Results
            </Title>

            {result.failed === 0 ? (
              <Alert
                type="success"
                showIcon
                icon={<CheckCircleOutlined />}
                message={`All ${result.successful} terms uploaded successfully`}
                description={`${result.successful} glossary terms were created in Draft status with workflow instances initiated.`}
              />
            ) : result.successful > 0 ? (
              <Alert
                type="warning"
                showIcon
                icon={<WarningOutlined />}
                message={`${result.successful} of ${result.total_rows} terms uploaded`}
                description={`${result.failed} row(s) failed validation. See the error details below.`}
              />
            ) : (
              <Alert
                type="error"
                showIcon
                message="Upload failed"
                description={`All ${result.total_rows} rows failed validation. See the error details below.`}
              />
            )}

            {result.errors.length > 0 && (
              <div style={{ marginTop: 16 }}>
                <Table
                  size="small"
                  columns={errorColumns}
                  dataSource={result.errors}
                  rowKey={(_, idx) => String(idx)}
                  pagination={{
                    pageSize: 10,
                    showTotal: (total) => `${total} error(s)`,
                  }}
                  scroll={{ y: 300 }}
                />
              </div>
            )}
          </div>
        )}
      </Space>
    </Modal>
  );
};

export default BulkUploadModal;

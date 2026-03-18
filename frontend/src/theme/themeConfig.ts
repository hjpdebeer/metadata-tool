import type { ThemeConfig } from 'antd';

/**
 * Professional, clean theme for metadata management.
 * Neutral palette with a deep navy primary — no brand affiliation.
 */
const themeConfig: ThemeConfig = {
  token: {
    // Primary: deep navy — authoritative, professional
    colorPrimary: '#1B3A5C',
    colorInfo: '#1B3A5C',

    // Success / Warning / Error
    colorSuccess: '#52C41A',
    colorWarning: '#FAAD14',
    colorError: '#FF4D4F',

    // Background and surfaces
    colorBgContainer: '#FFFFFF',
    colorBgLayout: '#F5F7FA',
    colorBgElevated: '#FFFFFF',

    // Text
    colorText: '#1F2937',
    colorTextSecondary: '#6B7280',
    colorTextTertiary: '#9CA3AF',

    // Borders
    colorBorder: '#E5E7EB',
    colorBorderSecondary: '#F3F4F6',

    // Typography
    fontFamily:
      "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', sans-serif",
    fontSize: 14,

    // Radius — slightly rounded for a modern look
    borderRadius: 6,

    // Spacing
    marginLG: 24,
    paddingLG: 24,
  },
  components: {
    Layout: {
      headerBg: '#0F2440',
      headerColor: '#FFFFFF',
      siderBg: '#FFFFFF',
      bodyBg: '#F5F7FA',
    },
    Menu: {
      itemSelectedBg: '#E8EEF5',
      itemSelectedColor: '#1B3A5C',
      itemHoverBg: '#F0F4F8',
    },
    Table: {
      headerBg: '#F8FAFC',
      headerColor: '#374151',
      rowHoverBg: '#F0F4F8',
    },
    Button: {
      primaryShadow: '0 2px 4px rgba(27, 58, 92, 0.15)',
    },
    Card: {
      boxShadowTertiary: '0 1px 3px rgba(0, 0, 0, 0.08)',
    },
  },
};

export default themeConfig;

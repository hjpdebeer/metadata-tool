import React from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ConfigProvider } from 'antd';
import themeConfig from './theme/themeConfig';
import AppLayout from './layouts/AppLayout';
import Dashboard from './pages/Dashboard';
import GlossaryPage from './pages/GlossaryPage';
import PlaceholderPage from './pages/PlaceholderPage';

const App: React.FC = () => {
  return (
    <ConfigProvider theme={themeConfig}>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<Navigate to="/dashboard" replace />} />
            <Route path="dashboard" element={<Dashboard />} />
            <Route path="glossary" element={<GlossaryPage />} />
            <Route
              path="data-dictionary"
              element={
                <PlaceholderPage
                  title="Data Dictionary"
                  description="Data elements, technical metadata, and critical data element management"
                />
              }
            />
            <Route
              path="data-quality"
              element={
                <PlaceholderPage
                  title="Data Quality"
                  description="Quality dimensions, rules, assessments, and scores"
                />
              }
            />
            <Route
              path="lineage"
              element={
                <PlaceholderPage
                  title="Data Lineage"
                  description="Business and technical data lineage visualization"
                />
              }
            />
            <Route
              path="applications"
              element={
                <PlaceholderPage
                  title="Business Application Registry"
                  description="Application inventory, classification, and data element links"
                />
              }
            />
            <Route
              path="processes"
              element={
                <PlaceholderPage
                  title="Business Process Registry"
                  description="Business process documentation and critical process management"
                />
              }
            />
            <Route
              path="workflow"
              element={
                <PlaceholderPage
                  title="My Tasks"
                  description="Workflow tasks assigned to you"
                />
              }
            />
          </Route>
        </Routes>
      </BrowserRouter>
    </ConfigProvider>
  );
};

export default App;

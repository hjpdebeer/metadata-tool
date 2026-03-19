import React from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { App as AntApp, ConfigProvider, Spin } from 'antd';
import themeConfig from './theme/themeConfig';
import { AuthProvider, useAuth } from './hooks/useAuth';
import AppLayout from './layouts/AppLayout';
import Dashboard from './pages/Dashboard';
import GlossaryPage from './pages/GlossaryPage';
import GlossaryTermDetail from './pages/GlossaryTermDetail';
import GlossaryTermForm from './pages/GlossaryTermForm';
import DataDictionaryPage from './pages/DataDictionaryPage';
import DataElementDetail from './pages/DataElementDetail';
import DataElementForm from './pages/DataElementForm';
import CdePage from './pages/CdePage';
import TechnicalMetadataPage from './pages/TechnicalMetadataPage';
import WorkflowTasksPage from './pages/WorkflowTasksPage';
import DataQualityDashboard from './pages/DataQualityDashboard';
import QualityRulesPage from './pages/QualityRulesPage';
import QualityRuleDetail from './pages/QualityRuleDetail';
import QualityRuleForm from './pages/QualityRuleForm';
import ApplicationsPage from './pages/ApplicationsPage';
import ApplicationDetail from './pages/ApplicationDetail';
import ApplicationForm from './pages/ApplicationForm';
import ProcessesPage from './pages/ProcessesPage';
import ProcessDetail from './pages/ProcessDetail';
import ProcessForm from './pages/ProcessForm';
import CriticalProcessesPage from './pages/CriticalProcessesPage';
import LineageGraphList from './pages/LineageGraphList';
import LineageGraphCreate from './pages/LineageGraphCreate';
import LineageGraphView from './pages/LineageGraphView';
import LoginPage from './pages/LoginPage';
import AdminPanel from './pages/AdminPanel';

const RequireAdmin: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { user } = useAuth();
  if (!user?.roles?.includes('ADMIN')) {
    return <Navigate to="/dashboard" replace />;
  }
  return <>{children}</>;
};

const ProtectedRoute: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { isAuthenticated, isLoading } = useAuth();

  if (isLoading) {
    return (
      <div
        style={{
          minHeight: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <Spin size="large" />
      </div>
    );
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return <>{children}</>;
};

const PublicRoute: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { isAuthenticated, isLoading } = useAuth();

  if (isLoading) {
    return (
      <div
        style={{
          minHeight: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <Spin size="large" />
      </div>
    );
  }

  if (isAuthenticated) {
    return <Navigate to="/dashboard" replace />;
  }

  return <>{children}</>;
};

const App: React.FC = () => {
  return (
    <ConfigProvider theme={themeConfig}>
      <AntApp>
      <BrowserRouter>
        <AuthProvider>
          <Routes>
            <Route
              path="/login"
              element={
                <PublicRoute>
                  <LoginPage />
                </PublicRoute>
              }
            />
            <Route
              path="/"
              element={
                <ProtectedRoute>
                  <AppLayout />
                </ProtectedRoute>
              }
            >
              <Route index element={<Navigate to="/dashboard" replace />} />
              <Route path="dashboard" element={<Dashboard />} />
              <Route path="glossary" element={<GlossaryPage />} />
              <Route path="glossary/new" element={<GlossaryTermForm />} />
              <Route path="glossary/:id" element={<GlossaryTermDetail />} />
              <Route path="glossary/:id/edit" element={<GlossaryTermForm />} />
              <Route path="data-dictionary" element={<DataDictionaryPage />} />
              <Route path="data-dictionary/new" element={<DataElementForm />} />
              <Route path="data-dictionary/cde" element={<CdePage />} />
              <Route path="data-dictionary/technical" element={<TechnicalMetadataPage />} />
              <Route path="data-dictionary/:id" element={<DataElementDetail />} />
              <Route path="data-dictionary/:id/edit" element={<DataElementForm />} />
              <Route path="data-quality" element={<DataQualityDashboard />} />
              <Route path="data-quality/rules" element={<QualityRulesPage />} />
              <Route path="data-quality/rules/new" element={<QualityRuleForm />} />
              <Route path="data-quality/rules/:id" element={<QualityRuleDetail />} />
              <Route path="data-quality/rules/:id/edit" element={<QualityRuleForm />} />
              <Route path="lineage" element={<LineageGraphList />} />
              <Route path="lineage/new" element={<LineageGraphCreate />} />
              <Route path="lineage/:id" element={<LineageGraphView />} />
              <Route path="applications" element={<ApplicationsPage />} />
              <Route path="applications/new" element={<ApplicationForm />} />
              <Route path="applications/:id" element={<ApplicationDetail />} />
              <Route path="applications/:id/edit" element={<ApplicationForm />} />
              <Route path="processes" element={<ProcessesPage />} />
              <Route path="processes/new" element={<ProcessForm />} />
              <Route path="processes/critical" element={<CriticalProcessesPage />} />
              <Route path="processes/:id" element={<ProcessDetail />} />
              <Route path="processes/:id/edit" element={<ProcessForm />} />
              <Route path="workflow" element={<WorkflowTasksPage />} />
              <Route path="admin" element={<RequireAdmin><AdminPanel /></RequireAdmin>} />
            </Route>
          </Routes>
        </AuthProvider>
      </BrowserRouter>
      </AntApp>
    </ConfigProvider>
  );
};

export default App;

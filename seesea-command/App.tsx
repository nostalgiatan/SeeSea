
import React, { useState, useEffect } from 'react';
import { NavigationTab } from './types';
import Sidebar from './components/Sidebar';
import Dashboard from './components/Dashboard';
import EnginesPanel from './components/EnginesPanel';
import CachePanel from './components/CachePanel';
import ConfigPanel from './components/ConfigPanel';
import LogsPanel from './components/LogsPanel';
import Header from './components/Header';
import FallbackUI from './components/FallbackUI';
import { fetchHealth } from './services/api';

const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState<NavigationTab>(NavigationTab.OVERVIEW);
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [status, setStatus] = useState<'loading' | 'online' | 'error'>('loading');

  useEffect(() => {
    checkSystem();
  }, []);

  const checkSystem = async () => {
    setStatus('loading');
    try {
      await fetchHealth();
      setStatus('online');
    } catch (err) {
      console.error("Failed to connect to backend API:", err);
      setStatus('error');
    }
  };

  const handleManualRefresh = () => {
    setIsRefreshing(true);
    checkSystem();
    setTimeout(() => setIsRefreshing(false), 800);
  };

  if (status === 'loading') return <div className="bg-slate-950 min-h-screen flex items-center justify-center"><FallbackUI type="loading" onRetry={checkSystem} /></div>;
  if (status === 'error') return <div className="bg-slate-950 min-h-screen flex items-center justify-center"><FallbackUI type="error" onRetry={checkSystem} message="无法连接到后端 API，请确保服务器已启动" /></div>;

  return (
    <div className="flex min-h-screen bg-slate-950 text-slate-100 selection:bg-blue-500/30">
      {isSidebarOpen && (
        <div 
          className="fixed inset-0 bg-black/60 backdrop-blur-sm z-40 lg:hidden"
          onClick={() => setIsSidebarOpen(false)}
        />
      )}

      <Sidebar 
        activeTab={activeTab} 
        setActiveTab={(tab) => {
          setActiveTab(tab);
          setIsSidebarOpen(false);
        }}
        isOpen={isSidebarOpen}
      />

      <main className="flex-1 flex flex-col min-w-0 overflow-hidden lg:pl-64">
        <Header 
          activeTab={activeTab} 
          setActiveTab={setActiveTab}
          toggleSidebar={() => setIsSidebarOpen(true)} 
          onRefresh={handleManualRefresh}
          isRefreshing={isRefreshing}
        />

        <div className="flex-1 overflow-y-auto p-3 md:p-6 space-y-6">
          <div className="max-w-7xl mx-auto w-full pb-10">
            {activeTab === NavigationTab.OVERVIEW && <Dashboard />}
            {activeTab === NavigationTab.ENGINES && <EnginesPanel />}
            {activeTab === NavigationTab.CACHE && <CachePanel />}
            {activeTab === NavigationTab.CONFIG && <ConfigPanel />}
            {activeTab === NavigationTab.LOGS && <LogsPanel />}
          </div>
        </div>
      </main>
    </div>
  );
};

export default App;

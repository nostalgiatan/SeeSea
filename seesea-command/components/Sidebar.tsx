
import React from 'react';
import { 
  LayoutDashboard, 
  Cpu, 
  Database, 
  Settings, 
  Terminal, 
  Waves
} from 'lucide-react';
import { NavigationTab } from '../types';

interface SidebarProps {
  activeTab: NavigationTab;
  setActiveTab: (tab: NavigationTab) => void;
  isOpen: boolean;
}

const Sidebar: React.FC<SidebarProps> = ({ activeTab, setActiveTab, isOpen }) => {
  const navItems = [
    { id: NavigationTab.OVERVIEW, icon: LayoutDashboard, label: '概览仪表盘' },
    { id: NavigationTab.ENGINES, icon: Cpu, label: '引擎管理' },
    { id: NavigationTab.CACHE, icon: Database, label: '缓存操作' },
    { id: NavigationTab.CONFIG, icon: Settings, label: '热加载配置' },
    { id: NavigationTab.LOGS, icon: Terminal, label: '系统日志' },
  ];

  return (
    <aside className={`
      fixed inset-y-0 left-0 z-50 w-64 glass-panel transform transition-transform duration-300 ease-in-out lg:translate-x-0
      ${isOpen ? 'translate-x-0' : '-translate-x-full'}
    `}>
      <div className="flex flex-col h-full">
        {/* Brand */}
        <div className="p-6 border-b border-white/5 flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-gradient-to-tr from-blue-600 to-purple-600 flex items-center justify-center shadow-lg shadow-blue-500/20">
            <Waves className="w-6 h-6 text-white" />
          </div>
          <div>
            <h1 className="text-xl font-bold tracking-tight">SeeSea</h1>
            <p className="text-[10px] text-slate-400 uppercase tracking-widest font-medium">Command Center</p>
          </div>
        </div>

        {/* Navigation */}
        <nav className="flex-1 px-4 py-6 space-y-2">
          {navItems.map((item) => (
            <button
              key={item.id}
              onClick={() => setActiveTab(item.id)}
              className={`
                w-full flex items-center gap-3 px-4 py-3 rounded-xl transition-all duration-200 group
                ${activeTab === item.id 
                  ? 'bg-blue-600/10 text-blue-400 border border-blue-500/20' 
                  : 'text-slate-400 hover:text-white hover:bg-white/5 border border-transparent'}
              `}
            >
              <item.icon className={`w-5 h-5 transition-transform duration-200 ${activeTab === item.id ? 'scale-110' : 'group-hover:scale-110'}`} />
              <span className="font-medium">{item.label}</span>
            </button>
          ))}
        </nav>

        {/* Footer */}
        <div className="p-6 border-t border-white/5">
          <div className="flex items-center gap-3 p-3 rounded-lg bg-slate-900/50">
            <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            <div className="flex flex-col">
              <span className="text-xs font-semibold">Service Online</span>
              <span className="text-[10px] text-slate-500">v2.1.0 Stable</span>
            </div>
          </div>
        </div>
      </div>
    </aside>
  );
};

export default Sidebar;

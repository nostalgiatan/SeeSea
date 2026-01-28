
import React, { useState, useRef, useEffect, useMemo } from 'react';
import { Menu, RefreshCw, Bell, Search, Settings, ShieldAlert, Zap, User, LogOut, X, Command, Activity, Terminal, Cpu, Database, ChevronRight, SearchSlash } from 'lucide-react';
import { NavigationTab } from '../types';

interface SearchAction {
  id: string;
  label: string;
  category: string;
  icon: any;
  shortcut?: string;
  action: () => void;
  keywords: string[];
}

// Fixed: Convert SearchCategory to React.FC to allow 'key' prop during list rendering
const SearchCategory: React.FC<{ title: string; children: React.ReactNode }> = ({ title, children }) => {
  return (
    <div className="space-y-2">
      <h4 className="text-[10px] text-slate-500 font-bold uppercase tracking-[0.2em] px-3">{title}</h4>
      <div className="space-y-1">
        {children}
      </div>
    </div>
  );
};

// Fixed: Convert SearchItem to React.FC to allow 'key' prop during list rendering
const SearchItem: React.FC<{ icon: any; label: string; shortcut?: string; onClick: () => void }> = ({ icon: Icon, label, shortcut, onClick }) => {
  return (
    <button 
      onClick={onClick}
      className="w-full flex items-center justify-between p-3 rounded-2xl hover:bg-white/5 text-slate-300 hover:text-white transition-all text-sm group active:scale-[0.98]"
    >
      <div className="flex items-center gap-3">
        <div className="p-2 rounded-lg bg-white/5 group-hover:bg-blue-600/20 group-hover:text-blue-400 transition-all">
          <Icon className="w-4 h-4" />
        </div>
        <span className="font-medium">{label}</span>
      </div>
      <div className="flex items-center gap-3">
        {shortcut && <span className="text-[9px] font-mono text-slate-600 font-bold hidden sm:inline">{shortcut}</span>}
        <ChevronRight className="w-3.5 h-3.5 text-slate-700 group-hover:text-blue-500 group-hover:translate-x-0.5 transition-all" />
      </div>
    </button>
  );
};

// Fixed: Convert NotifItem to React.FC for consistency and proper typing
const NotifItem: React.FC<any> = ({ icon: Icon, color, title, time }) => {
  return (
    <div className="p-4 hover:bg-white/5 transition-all flex gap-3 items-start border-b border-white/5 cursor-pointer group">
      <div className={`mt-0.5 p-1.5 rounded-lg bg-white/5 ${color}`}>
        <Icon className="w-3.5 h-3.5" />
      </div>
      <div className="flex-1 overflow-hidden">
        <p className="text-xs font-semibold text-slate-200 group-hover:text-blue-400 transition-colors truncate">{title}</p>
        <p className="text-[10px] text-slate-600 mt-1 font-medium tracking-tight">{time}</p>
      </div>
    </div>
  );
};

interface HeaderProps {
  activeTab: NavigationTab;
  setActiveTab: (tab: NavigationTab) => void;
  toggleSidebar: () => void;
  onRefresh: () => void;
  isRefreshing: boolean;
}

const Header: React.FC<HeaderProps> = ({ activeTab, setActiveTab, toggleSidebar, onRefresh, isRefreshing }) => {
  const [showNotifs, setShowNotifs] = useState(false);
  const [showProfile, setShowProfile] = useState(false);
  const [showSearch, setShowSearch] = useState(false);
  const [searchValue, setSearchValue] = useState('');
  
  const notifRef = useRef<HTMLDivElement>(null);
  const profileRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Define functional actions
  const searchActions: SearchAction[] = useMemo(() => [
    {
      id: 'go-overview',
      label: '查看系统概览仪表盘',
      category: '快速跳转',
      icon: Activity,
      shortcut: 'G O',
      keywords: ['概览', '仪表盘', 'overview', 'dashboard', 'status'],
      action: () => setActiveTab(NavigationTab.OVERVIEW)
    },
    {
      id: 'go-engines',
      label: '管理搜索引擎集群',
      category: '快速跳转',
      icon: Cpu,
      shortcut: 'G E',
      keywords: ['引擎', 'engine', 'google', 'bing', 'cluster'],
      action: () => setActiveTab(NavigationTab.ENGINES)
    },
    {
      id: 'go-logs',
      label: '打开实时系统日志',
      category: '快速跳转',
      icon: Terminal,
      shortcut: 'G L',
      keywords: ['日志', 'logs', 'terminal', 'debug'],
      action: () => setActiveTab(NavigationTab.LOGS)
    },
    {
      id: 'go-cache',
      label: '缓存空间与策略管理',
      category: '快速跳转',
      icon: Database,
      shortcut: 'G C',
      keywords: ['缓存', 'cache', 'redis', 'clear'],
      action: () => setActiveTab(NavigationTab.CACHE)
    },
    {
      id: 'go-config',
      label: '修改系统运行配置',
      category: '快速跳转',
      icon: Settings,
      shortcut: 'G S',
      keywords: ['配置', 'config', 'settings', 'hotload'],
      action: () => setActiveTab(NavigationTab.CONFIG)
    },
    {
      id: 'action-refresh',
      label: '执行全站数据同步',
      category: '系统命令',
      icon: RefreshCw,
      keywords: ['刷新', 'refresh', 'sync', 'reload'],
      action: onRefresh
    },
  ], [setActiveTab, onRefresh]);

  const filteredActions = useMemo(() => {
    if (!searchValue.trim()) return searchActions;
    const lower = searchValue.toLowerCase();
    return searchActions.filter(item => 
      item.label.toLowerCase().includes(lower) || 
      item.keywords.some(k => k.includes(lower))
    );
  }, [searchValue, searchActions]);

  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (notifRef.current && !notifRef.current.contains(e.target as Node)) setShowNotifs(false);
      if (profileRef.current && !profileRef.current.contains(e.target as Node)) setShowProfile(false);
    };
    document.addEventListener('mousedown', handleClick);
    
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setShowSearch(true);
      }
      if (e.key === 'Escape') setShowSearch(false);
    };
    document.addEventListener('keydown', handleKeyDown);
    
    return () => {
      document.removeEventListener('mousedown', handleClick);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  useEffect(() => {
    if (showSearch && searchInputRef.current) {
      searchInputRef.current.focus();
    }
  }, [showSearch]);

  const handleActionClick = (action: () => void) => {
    action();
    setShowSearch(false);
    setSearchValue('');
  };

  const getTabTitle = (tab: NavigationTab) => {
    switch (tab) {
      case NavigationTab.OVERVIEW: return '概览';
      case NavigationTab.ENGINES: return '引擎';
      case NavigationTab.CACHE: return '缓存';
      case NavigationTab.CONFIG: return '配置';
      case NavigationTab.LOGS: return '日志';
      default: return '仪表盘';
    }
  };

  return (
    <header className="h-16 glass-panel border-t-0 border-x-0 flex items-center justify-between px-4 md:px-6 sticky top-0 z-30">
      {/* Premium Command Center Search Overlay */}
      {showSearch && (
        <div className="fixed inset-0 z-[100] flex items-start justify-center pt-10 md:pt-24 px-4 sm:px-6">
          <div className="absolute inset-0 bg-slate-950/40 backdrop-blur-2xl animate-in fade-in duration-300" onClick={() => setShowSearch(false)} />
          
          <div className="relative w-full max-w-2xl glass-panel rounded-3xl shadow-[0_32px_64px_-12px_rgba(0,0,0,0.8)] border-white/20 overflow-hidden animate-in zoom-in-95 slide-in-from-top-10 duration-300">
            {/* Command Input Area */}
            <div className="p-4 md:p-6 border-b border-white/10 bg-white/[0.02] relative overflow-hidden">
              <div className="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-transparent via-blue-500/40 to-transparent animate-pulse" />
              <div className="flex items-center gap-4 relative z-10">
                <Search className={`w-6 h-6 transition-colors ${searchValue ? 'text-blue-400' : 'text-slate-600'}`} />
                <input 
                  ref={searchInputRef}
                  value={searchValue}
                  onChange={(e) => setSearchValue(e.target.value)}
                  placeholder="键入指令或功能名称 (例如 'logs', 'engine')..." 
                  className="flex-1 bg-transparent border-none text-xl focus:ring-0 placeholder:text-slate-600 outline-none"
                />
                <button onClick={() => setShowSearch(false)} className="p-2 hover:bg-white/10 rounded-full text-slate-400 transition-colors">
                  <X className="w-5 h-5" />
                </button>
              </div>
            </div>
            
            <div className="max-h-[60vh] overflow-y-auto p-4 md:p-6 space-y-8 scrollbar-hide">
              {filteredActions.length > 0 ? (
                <>
                  {/* Dynamically grouped results */}
                  {['快速跳转', '系统命令'].map(cat => {
                    const items = filteredActions.filter(a => a.category === cat);
                    if (items.length === 0) return null;
                    return (
                      <SearchCategory key={cat} title={cat}>
                        {items.map(item => (
                          <SearchItem 
                            key={item.id} 
                            icon={item.icon} 
                            label={item.label} 
                            shortcut={item.shortcut} 
                            onClick={() => handleActionClick(item.action)}
                          />
                        ))}
                      </SearchCategory>
                    );
                  })}
                </>
              ) : (
                <div className="flex flex-col items-center justify-center py-20 text-center animate-in fade-in duration-500">
                  <div className="p-4 rounded-full bg-slate-900/50 border border-white/5 mb-4">
                    <SearchSlash className="w-8 h-8 text-slate-700" />
                  </div>
                  <h3 className="text-lg font-bold text-slate-400">未找到匹配指令</h3>
                  <p className="text-sm text-slate-600 max-w-xs mt-2">试试搜索 "刷新"、"配置" 或者 "引擎" 以获得更多结果。</p>
                </div>
              )}
            </div>
            
            <div className="p-4 bg-blue-600/5 border-t border-white/5 flex justify-between items-center text-[10px] text-slate-600 font-bold uppercase tracking-widest">
              <div className="flex items-center gap-2">
                <span className="w-1.5 h-1.5 rounded-full bg-blue-500 animate-pulse" />
                <span>Command Hub Alpha</span>
              </div>
              <div className="flex gap-4">
                <span className="flex items-center gap-1">ENTER 确认</span>
                <span className="flex items-center gap-1">ESC 关闭</span>
              </div>
            </div>
          </div>
        </div>
      )}

      <div className="flex items-center gap-3">
        <button 
          onClick={toggleSidebar}
          className="lg:hidden p-2 hover:bg-white/5 rounded-lg text-slate-400 active:scale-95 transition-all"
        >
          <Menu className="w-6 h-6" />
        </button>
        <div className="flex flex-col">
          <div className="flex items-center gap-2 text-[10px] md:text-xs text-slate-500 uppercase tracking-widest font-bold">
            <span className="hidden sm:inline">Admin</span>
            <span className="hidden sm:inline">/</span>
            <span className="text-blue-400">{getTabTitle(activeTab)}</span>
          </div>
        </div>
      </div>

      <div className="flex items-center gap-1.5 md:gap-3">
        {/* Desktop Search Trigger */}
        <button 
          onClick={() => setShowSearch(true)}
          className="hidden md:flex items-center gap-3 bg-slate-900/50 border border-white/5 px-4 py-2 rounded-xl text-slate-500 hover:text-slate-300 hover:border-white/20 transition-all active:scale-95"
        >
          <Search className="w-4 h-4" />
          <span className="text-sm mr-8">搜索系统指标...</span>
          <div className="flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-white/5 text-[9px] font-bold">
            <Command className="w-2.5 h-2.5" /> K
          </div>
        </button>

        {/* Mobile Search Icon */}
        <button 
          onClick={() => setShowSearch(true)}
          className="md:hidden p-2 hover:bg-white/5 rounded-lg text-slate-400 active:scale-95 transition-all"
        >
          <Search className="w-5 h-5" />
        </button>

        <button 
          onClick={onRefresh}
          className={`p-2 hover:bg-white/5 rounded-lg text-slate-400 transition-all ${isRefreshing ? 'animate-spin text-blue-400' : 'active:rotate-180'}`}
          title="刷新数据"
        >
          <RefreshCw className="w-5 h-5" />
        </button>
        
        <div className="relative" ref={notifRef}>
          <button 
            onClick={() => setShowNotifs(!showNotifs)}
            className={`p-2 rounded-lg transition-all relative ${showNotifs ? 'bg-blue-600/20 text-blue-400' : 'text-slate-400 hover:bg-white/5'}`}
          >
            <Bell className="w-5 h-5" />
            <span className="absolute top-2 right-2.5 w-2 h-2 bg-red-500 rounded-full border-2 border-slate-950" />
          </button>
          
          {showNotifs && (
            <>
              <div className="absolute top-full right-0 mt-3 w-72 md:w-80 glass-panel rounded-2xl shadow-2xl overflow-hidden border-white/20 animate-in fade-in zoom-in-95 duration-200 origin-top-right">
                <div className="p-4 border-b border-white/10 bg-white/[0.03] flex justify-between items-center">
                  <span className="font-bold text-sm">系统告警中心</span>
                  <span className="text-[10px] bg-red-500/20 text-red-400 px-2 py-0.5 rounded-full font-bold">3 NEW</span>
                </div>
                <div className="max-h-80 overflow-y-auto">
                  <NotifItem icon={ShieldAlert} color="text-red-400" title="Bing 引擎响应超时" time="2分钟前" />
                  <NotifItem icon={Zap} color="text-amber-400" title="API 负载达到 85%" time="15分钟前" />
                  <NotifItem icon={Settings} color="text-blue-400" title="热配置已全量发布" time="1小时前" />
                </div>
              </div>
              <div className="absolute top-full right-3 md:right-4 w-3 h-3 bg-slate-900 border-l border-t border-white/20 rotate-45 -translate-y-1.5 z-10" />
            </>
          )}
        </div>

        <div className="relative" ref={profileRef}>
          <button 
            onClick={() => setShowProfile(!showProfile)}
            className="w-8 h-8 md:w-9 md:h-9 rounded-xl bg-gradient-to-br from-blue-600 to-indigo-600 border border-white/20 flex items-center justify-center p-0.5 active:scale-95 transition-all shadow-lg shadow-blue-500/10"
          >
            <div className="w-full h-full rounded-[10px] overflow-hidden bg-slate-950">
              <img src="https://api.dicebear.com/7.x/avataaars/svg?seed=Felix" alt="Avatar" />
            </div>
          </button>

          {showProfile && (
            <>
              <div className="absolute top-full right-0 mt-3 w-56 glass-panel rounded-2xl shadow-2xl overflow-hidden border-white/20 animate-in fade-in zoom-in-95 duration-200 origin-top-right">
                <div className="p-4 bg-white/[0.05] border-b border-white/5">
                  <p className="text-[10px] font-bold text-slate-500 uppercase tracking-widest mb-1">管理员</p>
                  <p className="font-bold truncate text-sm">SeeSea_Root_Admin</p>
                </div>
                <div className="p-1.5">
                  <button className="w-full flex items-center gap-3 px-3 py-2.5 text-xs font-medium hover:bg-white/5 rounded-xl transition-all text-slate-300">
                    <User className="w-4 h-4 text-blue-400" /> 个人中心
                  </button>
                  <button className="w-full flex items-center gap-3 px-3 py-2.5 text-xs font-medium hover:bg-white/5 rounded-xl transition-all text-slate-300">
                    <ShieldAlert className="w-4 h-4 text-amber-400" /> 开启维护模式
                  </button>
                  <div className="h-px bg-white/5 my-1 mx-2" />
                  <button className="w-full flex items-center gap-3 px-3 py-2.5 text-xs font-medium hover:bg-red-400/10 rounded-xl transition-all text-red-400">
                    <LogOut className="w-4 h-4" /> 退出登录
                  </button>
                </div>
              </div>
              <div className="absolute top-full right-3 md:right-4 w-3 h-3 bg-slate-900 border-l border-t border-white/20 rotate-45 -translate-y-1.5 z-10" />
            </>
          )}
        </div>
      </div>
    </header>
  );
};

export default Header;

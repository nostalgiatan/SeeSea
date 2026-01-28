
import React from 'react';
import { Save, Globe, Shield, Terminal, Loader2, CheckCircle2, AlertCircle } from 'lucide-react';
import { useConfig } from '../hooks/useConfig';

const ConfigPanel: React.FC = () => {
  const { config, loading, isSaving, showSuccess, error, updateField, save } = useConfig();

  // 1. Loading State
  if (loading) return (
    <div className="flex flex-col items-center justify-center py-32 space-y-4">
      <Loader2 className="w-10 h-10 animate-spin text-blue-500" />
      <p className="text-slate-500 font-medium animate-pulse">正在同步内核配置...</p>
    </div>
  );

  // 2. Safety Check: If config is null after loading, show error instead of crashing
  if (!config) return (
    <div className="glass-panel p-10 rounded-3xl border-red-500/20 text-center space-y-4">
      <AlertCircle className="w-12 h-12 text-red-500 mx-auto" />
      <h3 className="text-xl font-bold text-white">配置加载异常</h3>
      <p className="text-slate-400 max-w-sm mx-auto">无法连接到配置服务器，请检查后端 API 服务是否正常运行。</p>
    </div>
  );

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">内核热配置</h2>
          <p className="text-slate-400 text-sm">修改即刻生效，底层逻辑自动重新挂载</p>
        </div>
        <button 
          onClick={save} 
          disabled={isSaving} 
          className="flex items-center gap-2 bg-blue-600 hover:bg-blue-500 px-6 py-2.5 rounded-xl font-bold transition-all shadow-lg shadow-blue-500/20 disabled:opacity-50"
        >
          {isSaving ? <Loader2 className="w-5 h-5 animate-spin" /> : <Save className="w-5 h-5" />} 部署配置
        </button>
      </div>

      {error && (
        <div className="bg-red-500/10 border border-red-500/20 p-4 rounded-2xl flex items-center gap-3 text-red-400 text-sm font-medium animate-in slide-in-from-top-4">
          <AlertCircle className="w-5 h-5" /> {error}
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <section className="glass-panel p-6 rounded-3xl space-y-6 border-white/5">
          <h3 className="text-lg font-bold flex items-center gap-2 border-b border-white/5 pb-4">
            <Globe className="w-5 h-5 text-blue-400" />
            请求与性能控制
          </h3>
          <div className="space-y-5">
            <ConfigItem label="API 监听端口" description="默认 8080">
              <input 
                type="number" 
                value={config.api_port || ''} 
                onChange={(e) => updateField('api_port', parseInt(e.target.value))} 
                className="bg-slate-900/50 border border-white/10 rounded-xl px-4 py-2.5 w-full focus:ring-2 focus:ring-blue-500/30 outline-none transition-all" 
              />
            </ConfigItem>
            <ConfigItem label="搜索超时阈值 (ms)" description="建议 5000">
              <input 
                type="number" 
                value={config.search_timeout_ms || ''} 
                onChange={(e) => updateField('search_timeout_ms', parseInt(e.target.value))} 
                className="bg-slate-900/50 border border-white/10 rounded-xl px-4 py-2.5 w-full focus:ring-2 focus:ring-blue-500/30 outline-none transition-all" 
              />
            </ConfigItem>
          </div>
        </section>

        <section className="glass-panel p-6 rounded-3xl space-y-6 border-white/5">
          <h3 className="text-lg font-bold flex items-center gap-2 border-b border-white/5 pb-4">
            <Shield className="w-5 h-5 text-purple-400" />
            特性开关
          </h3>
          <div className="space-y-6">
            <div className="flex items-center justify-between p-2 rounded-2xl hover:bg-white/[0.02] transition-all">
              <div>
                <p className="font-semibold text-sm">调试模式</p>
                <p className="text-xs text-slate-500">开启后记录详细 Trace 路径</p>
              </div>
              <Switch checked={config.debug_mode} onChange={(val) => updateField('debug_mode', val)} />
            </div>
          </div>
        </section>
      </div>

      <div className="glass-panel p-6 rounded-3xl border-white/5">
        <div className="flex items-center gap-3 mb-4 text-slate-400">
          <Terminal className="w-4 h-4" />
          <h3 className="text-xs font-black uppercase tracking-widest">Active Descriptor</h3>
        </div>
        <div className="bg-black/50 p-6 rounded-2xl font-mono text-sm text-blue-400/80 border border-white/5 overflow-x-auto">
          <pre>{JSON.stringify(config, null, 2)}</pre>
        </div>
      </div>

      {showSuccess && (
        <div className="fixed bottom-10 right-10 flex items-center gap-3 px-6 py-4 rounded-2xl bg-blue-600 text-white shadow-2xl animate-in slide-in-from-bottom-10">
          <CheckCircle2 className="w-5 h-5" />
          <span className="font-bold">配置已成功热重载</span>
        </div>
      )}
    </div>
  );
};

const ConfigItem: React.FC<{ label: string; description: string; children: React.ReactNode }> = ({ label, description, children }) => (
  <div className="space-y-1.5">
    <div className="flex justify-between items-end px-1">
      <label className="text-xs font-bold text-slate-300">{label}</label>
      <span className="text-[10px] text-slate-600 font-medium italic">{description}</span>
    </div>
    {children}
  </div>
);

const Switch: React.FC<{ checked: boolean; onChange: (val: boolean) => void }> = ({ checked, onChange }) => (
  <button 
    onClick={() => onChange(!checked)} 
    className={`w-11 h-6 rounded-full p-1 transition-all duration-300 ${checked ? 'bg-blue-500 shadow-[0_0_12px_rgba(59,130,246,0.5)]' : 'bg-slate-800'}`}
  >
    <div className={`w-4 h-4 bg-white rounded-full transition-transform duration-300 ${checked ? 'translate-x-5' : 'translate-x-0'}`} />
  </button>
);

export default ConfigPanel;

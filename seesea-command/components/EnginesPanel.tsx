
import React from 'react';
import { Power, RefreshCw, Loader2, Activity, ShieldCheck, Zap } from 'lucide-react';
import { useEngines } from '../hooks/useEngines';

const EnginesPanel: React.FC = () => {
  const { engines, loading, processing, toggle, refresh } = useEngines();

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 relative z-10">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold flex items-center gap-2">
            搜索引擎集群
            {loading && <Loader2 className="w-4 h-4 animate-spin text-blue-500" />}
          </h2>
          <p className="text-slate-400 text-sm">
            管理分布式搜索节点 • {engines.length} 个活跃引擎 
            <span className="mx-2 text-slate-700">|</span>
            可用率: <span className="text-blue-400 font-bold">{engines.length > 0 ? (engines.filter(e => e.enabled).length / engines.length * 100).toFixed(1) : 0}%</span>
          </p>
        </div>
        <button 
          type="button"
          onClick={refresh} 
          disabled={loading}
          className="flex items-center gap-2 bg-slate-900 border border-white/10 hover:border-white/30 hover:bg-slate-800 px-5 py-2.5 rounded-xl text-xs font-bold transition-all active:scale-95 disabled:opacity-50 cursor-pointer shadow-lg"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin text-blue-400' : ''}`} /> 
          强制同步状态
        </button>
      </div>

      <div className="glass-panel rounded-3xl overflow-hidden border-white/5 shadow-2xl">
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="bg-white/[0.03] text-[10px] uppercase tracking-[0.15em] text-slate-500 font-black border-b border-white/5">
                <th className="px-6 py-5">节点标识</th>
                <th className="px-6 py-5">实时状态</th>
                <th className="px-6 py-5">请求总量</th>
                <th className="px-6 py-5">QoS 成功率</th>
                <th className="px-6 py-5">延迟 (Latency)</th>
                <th className="px-6 py-5 text-right">控制单元</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-white/[0.03]">
              {loading && engines.length === 0 ? (
                [1,2,3,4,5].map(i => (
                  <tr key={i} className="animate-pulse">
                    <td colSpan={6} className="px-6 py-6">
                      <div className="h-6 bg-slate-800/40 rounded-lg w-full" />
                    </td>
                  </tr>
                ))
              ) : (
                engines.map((engine) => (
                  <tr key={engine.name} className="hover:bg-white/[0.02] transition-colors group relative">
                    <td className="px-6 py-5">
                      <div className="flex items-center gap-3">
                        <div className={`w-2.5 h-2.5 rounded-full transition-all duration-500 ${engine.enabled ? (engine.temporarily_disabled ? 'bg-amber-500 animate-pulse' : 'bg-green-500 shadow-[0_0_12px_rgba(34,197,94,0.6)]') : 'bg-red-500/50'}`} />
                        <div className="flex flex-col">
                          <span className="font-bold text-slate-200 group-hover:text-white transition-colors">{engine.name}</span>
                          <span className="text-[9px] text-slate-600 font-mono">ID: {engine.name.toUpperCase()}_NODE_01</span>
                        </div>
                      </div>
                    </td>
                    <td className="px-6 py-5">
                      <div className="flex items-center">
                        {engine.enabled ? (
                          engine.temporarily_disabled ? (
                            <div className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-amber-500/10 text-amber-400 border border-amber-500/20">
                              <Zap className="w-3 h-3" />
                              <span className="text-[10px] font-black uppercase tracking-tight">熔断保护</span>
                            </div>
                          ) : (
                            <div className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                              <ShieldCheck className="w-3 h-3" />
                              <span className="text-[10px] font-black uppercase tracking-tight">正常运行</span>
                            </div>
                          )
                        ) : (
                          <div className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-slate-800/50 text-slate-500 border border-white/5">
                            <Power className="w-3 h-3" />
                            <span className="text-[10px] font-black uppercase tracking-tight">已停用</span>
                          </div>
                        )}
                      </div>
                    </td>
                    <td className="px-6 py-5 font-mono text-xs text-slate-400 font-medium">{engine.total_requests.toLocaleString()}</td>
                    <td className="px-6 py-5">
                      <div className="flex flex-col gap-1.5">
                        <div className="flex justify-between items-center w-24">
                          <span className="text-[10px] font-bold text-slate-300">{engine.success_rate.toFixed(1)}%</span>
                        </div>
                        <div className="w-24 h-1 bg-slate-900 rounded-full overflow-hidden">
                          <div 
                            className={`h-full transition-all duration-1000 ${engine.success_rate > 95 ? 'bg-emerald-500' : engine.success_rate > 80 ? 'bg-amber-500' : 'bg-red-500'}`}
                            style={{ width: `${engine.success_rate}%` }}
                          />
                        </div>
                      </div>
                    </td>
                    <td className="px-6 py-5">
                      <div className="flex items-center gap-2">
                        <Activity className="w-3 h-3 text-blue-500/50" />
                        <span className="font-mono text-xs text-blue-400 font-bold">{engine.avg_response_time_ms || '--'}ms</span>
                      </div>
                    </td>
                    <td className="px-6 py-5 text-right">
                      <button 
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          toggle(engine.name, engine.enabled);
                        }}
                        disabled={processing === engine.name}
                        className={`
                          p-2.5 rounded-xl transition-all cursor-pointer relative z-20 active:scale-90
                          ${engine.enabled 
                            ? 'text-red-400 hover:bg-red-400/10 border border-transparent hover:border-red-400/20' 
                            : 'text-emerald-400 hover:bg-emerald-400/10 border border-transparent hover:border-emerald-400/20'}
                          disabled:opacity-30 disabled:cursor-not-allowed
                        `}
                        title={engine.enabled ? "停用引擎" : "启用引擎"}
                      >
                        {processing === engine.name ? (
                          <Loader2 className="w-5 h-5 animate-spin" />
                        ) : (
                          <Power className={`w-5 h-5 ${engine.enabled ? 'fill-red-400/10' : ''}`} />
                        )}
                      </button>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
      
      {/* Legend / Info */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="p-4 rounded-2xl bg-white/[0.02] border border-white/5 flex items-start gap-4">
          <div className="p-2 rounded-lg bg-green-500/10 text-green-500"><ShieldCheck className="w-4 h-4" /></div>
          <div>
            <h4 className="text-xs font-bold text-slate-300">健康巡检</h4>
            <p className="text-[10px] text-slate-500 mt-1 leading-relaxed">系统每 30s 自动对集群节点进行一次深度 QoS 拨测。</p>
          </div>
        </div>
        <div className="p-4 rounded-2xl bg-white/[0.02] border border-white/5 flex items-start gap-4">
          <div className="p-2 rounded-lg bg-amber-500/10 text-amber-500"><Zap className="w-4 h-4" /></div>
          <div>
            <h4 className="text-xs font-bold text-slate-300">自动熔断</h4>
            <p className="text-[10px] text-slate-500 mt-1 leading-relaxed">连续 5 次请求失败将自动触发熔断，静默 120s 后自动重试。</p>
          </div>
        </div>
        <div className="p-4 rounded-2xl bg-white/[0.02] border border-white/5 flex items-start gap-4">
          <div className="p-2 rounded-lg bg-blue-500/10 text-blue-500"><Activity className="w-4 h-4" /></div>
          <div>
            <h4 className="text-xs font-bold text-slate-300">负载均衡</h4>
            <p className="text-[10px] text-slate-500 mt-1 leading-relaxed">动态权重算法，根据响应延迟自动调节节点并发分发比例。</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default EnginesPanel;

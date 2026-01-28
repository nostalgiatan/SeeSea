
import React from 'react';
import { Database, Trash2, ShieldAlert, Clock, BarChart3, Info, CheckCircle2, Loader2 } from 'lucide-react';
import { useCache } from '../hooks/useCache';

const CachePanel: React.FC = () => {
  const { cacheData, loading, isClearing, showToast, handleClear } = useCache();

  if (loading) return (
    <div className="flex flex-col items-center justify-center py-32 space-y-4">
      <Loader2 className="w-10 h-10 animate-spin text-blue-500" />
      <p className="text-slate-500 font-medium">读取缓存索引...</p>
    </div>
  );

  const stats = cacheData || { total_keys: 0, size_bytes: 0, hit_rate: 0 };

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">全局缓存管理</h2>
          <p className="text-slate-400 text-sm">监控和配置 Redis/内存 缓存层的表现</p>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="md:col-span-2 space-y-6">
          <div className="glass-panel p-6 rounded-2xl grid grid-cols-2 sm:grid-cols-3 gap-6">
            <div className="space-y-1">
              <span className="text-[10px] text-slate-500 uppercase font-bold tracking-wider">总缓存键数</span>
              <p className="text-2xl font-bold tabular-nums">{stats.total_keys.toLocaleString()}</p>
            </div>
            <div className="space-y-1">
              <span className="text-[10px] text-slate-500 uppercase font-bold tracking-wider">占用空间</span>
              <p className="text-2xl font-bold tabular-nums">{(stats.size_bytes / (1024 * 1024 * 1024)).toFixed(2)} GB</p>
            </div>
            <div className="space-y-1">
              <span className="text-[10px] text-slate-500 uppercase font-bold tracking-wider">命中率</span>
              <p className="text-2xl font-bold text-blue-400 tabular-nums">{(stats.hit_rate * 100).toFixed(1)}%</p>
            </div>
          </div>

          <div className="glass-panel p-6 rounded-2xl relative overflow-hidden">
            <h3 className="text-lg font-bold mb-6 flex items-center gap-2">
              <BarChart3 className="w-5 h-5 text-blue-400" />
              命中效能 (Live)
            </h3>
            <div className="h-48 flex items-end gap-2 relative z-10">
              {[65, 80, 72, 85, 90, 88, 75, 82, 92, 85, 80, 86].map((h, i) => (
                <div key={i} className="flex-1 flex flex-col items-center gap-2 group">
                  <div 
                    className="w-full bg-blue-500/20 group-hover:bg-blue-500/40 rounded-t-lg transition-all duration-500"
                    style={{ height: `${h}%` }}
                  />
                  <span className="text-[8px] text-slate-600 font-mono">{i*5}m</span>
                </div>
              ))}
            </div>
          </div>
        </div>

        <div className="space-y-6">
          <div className="glass-panel p-6 rounded-2xl bg-gradient-to-br from-red-500/5 to-transparent border-red-500/20">
            <h3 className="text-lg font-bold mb-4 flex items-center gap-2 text-red-400">
              <ShieldAlert className="w-5 h-5" />
              危险操作区
            </h3>
            <p className="text-sm text-slate-400 mb-6 leading-relaxed">清除缓存将立即导致后端负载激增，所有搜索引擎将重新进行全量索引，请谨慎操作。</p>
            <button 
              onClick={() => handleClear()}
              disabled={isClearing}
              className="w-full flex items-center justify-center gap-2 py-3.5 rounded-xl bg-red-600 hover:bg-red-500 text-white font-bold transition-all disabled:opacity-50 active:scale-[0.98] shadow-lg shadow-red-500/10"
            >
              {isClearing ? <Clock className="w-4 h-4 animate-spin" /> : <Trash2 className="w-4 h-4" />}
              全量清除缓存
            </button>
          </div>

          <div className="glass-panel p-6 rounded-2xl">
            <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
              <Info className="w-5 h-5 text-blue-400" />
              当前策略
            </h3>
            <div className="space-y-4 text-sm font-medium">
              <div className="flex justify-between items-center">
                <span className="text-slate-500">逐出模型</span>
                <span className="px-2 py-0.5 rounded bg-slate-900 border border-white/5 text-[10px] font-mono">LRU_VOLATILE</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-slate-500">实时压缩</span>
                <span className="text-green-400">ENABLED (LZ4)</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {showToast && (
        <div className="fixed bottom-6 right-6 flex items-center gap-3 px-6 py-4 rounded-2xl bg-slate-900 border border-emerald-500/30 shadow-2xl animate-in slide-in-from-right-10 duration-500">
          <CheckCircle2 className="w-6 h-6 text-emerald-400" />
          <div>
            <p className="font-bold text-sm">指令执行成功</p>
            <p className="text-[10px] text-slate-500 uppercase tracking-tight">Cache layer has been flushed</p>
          </div>
        </div>
      )}
    </div>
  );
};

export default CachePanel;


import React, { useEffect, useRef } from 'react';
import { Play, Square, Search, RefreshCw, AlertCircle, FileText } from 'lucide-react';
import { useLogs } from '../hooks/useLogs';

const LogsPanel: React.FC = () => {
  const { 
    logs, 
    isLive, 
    setIsLive, 
    filter, 
    setFilter, 
    clearLogs, 
    isLoading, 
    error,
    logFiles,
    currentFile,
    setCurrentFile,
    refreshLogs
  } = useLogs();
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isLive && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs, isLive]);

  const getLevelColor = (level: string) => {
    switch (level) {
      case 'info': return 'text-blue-400';
      case 'warn': return 'text-amber-400';
      case 'error': return 'text-red-400';
      case 'debug': return 'text-slate-500';
      default: return 'text-slate-100';
    }
  };

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const formatDate = (timestamp: number | null) => {
    if (!timestamp) return '-';
    const date = new Date(timestamp * 1000);
    return date.toLocaleString('zh-CN', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  return (
    <div className="h-[calc(100vh-140px)] md:h-[calc(100vh-160px)] flex flex-col space-y-4 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div className="flex items-center justify-between px-1">
        <div>
          <h2 className="text-xl md:text-2xl font-bold">系统日志</h2>
          <p className="text-slate-500 text-xs hidden sm:block">实时流式监控内核事件</p>
        </div>
        <div className="flex items-center gap-2">
          <button 
            onClick={refreshLogs}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-xl transition-all text-[10px] font-bold ${isLoading ? 'animate-spin' : ''}`}
            title="刷新日志"
          >
            <RefreshCw className={`w-3 h-3 ${isLoading ? 'animate-spin' : ''}`} />
          </button>
          <button 
            onClick={() => setIsLive(!isLive)} 
            className={`flex items-center gap-2 px-3 py-1.5 md:px-4 md:py-2 rounded-xl transition-all text-[10px] md:text-xs font-bold ${isLive ? 'bg-blue-600 shadow-lg shadow-blue-500/20' : 'bg-slate-900 border border-white/5 text-slate-400'}`}
          >
            {isLive ? <Play className="w-3 h-3 fill-current" /> : <Square className="w-3 h-3 fill-current" />} {isLive ? 'LIVE' : 'PAUSED'}
          </button>
        </div>
      </div>

      <div className="flex-1 glass-panel rounded-2xl flex flex-col overflow-hidden">
        <div className="p-3 border-b border-white/5 bg-white/[0.01] flex items-center justify-between gap-2">
          <div className="flex gap-1 overflow-x-auto no-scrollbar flex-1">
            {['all', 'info', 'warn', 'error'].map((lvl) => (
              <button 
                key={lvl} 
                onClick={() => setFilter(lvl)} 
                className={`px-3 py-1 rounded-lg text-[9px] font-bold uppercase tracking-widest transition-all border shrink-0 ${filter === lvl ? 'bg-white/10 border-white/20 text-white' : 'border-transparent text-slate-500'}`}
              >
                {lvl}
              </button>
            ))}
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {logFiles.length > 0 && (
              <select
                value={currentFile || ''}
                onChange={(e) => setCurrentFile(e.target.value)}
                className="bg-slate-900 border border-white/10 rounded-lg px-2 py-1 text-[10px] text-slate-300 focus:outline-none focus:border-blue-500"
              >
                {logFiles.map((file) => (
                  <option key={file.name} value={file.name}>
                    {file.name} ({formatFileSize(file.size)})
                  </option>
                ))}
              </select>
            )}
            <Search className="w-4 h-4 text-slate-600 sm:hidden" />
          </div>
        </div>

        <div ref={scrollRef} className="flex-1 p-3 font-mono text-[10px] md:text-xs overflow-y-auto bg-black/30 selection:bg-blue-500/40">
          {error && (
            <div className="flex items-center gap-2 text-red-400 mb-2 p-2 bg-red-500/10 rounded-lg">
              <AlertCircle className="w-4 h-4 shrink-0" />
              <span className="text-xs">{error}</span>
            </div>
          )}
          
          {isLoading && logs.length === 0 ? (
            <div className="flex items-center justify-center h-full text-slate-500">
              <div className="flex items-center gap-2">
                <RefreshCw className="w-4 h-4 animate-spin" />
                <span>加载日志中...</span>
              </div>
            </div>
          ) : logs.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-slate-500">
              <FileText className="w-8 h-8 mb-2 opacity-50" />
              <span className="text-sm">暂无日志</span>
            </div>
          ) : (
            <div className="space-y-2 md:space-y-1">
              {logs.filter(l => filter === 'all' || l.level === filter).map((log) => (
                <div 
                  key={log.id} 
                  className="flex flex-col md:flex-row gap-0.5 md:gap-4 group hover:bg-white/5 rounded px-2 py-1 md:py-0.5 transition-colors items-start"
                >
                  <div className="flex items-center gap-2 shrink-0">
                    <span className="text-slate-600 select-none">[{log.timestamp}]</span>
                    <span className={`font-bold ${getLevelColor(log.level)} min-w-[3.5rem]`}>
                      {log.level.toUpperCase()}
                    </span>
                  </div>
                  <div className="flex items-start gap-3">
                    <span className="text-blue-500/70 shrink-0 font-bold hidden md:inline">{log.service}:</span>
                    <span className="text-slate-300 break-words leading-relaxed">{log.message}</span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default LogsPanel;

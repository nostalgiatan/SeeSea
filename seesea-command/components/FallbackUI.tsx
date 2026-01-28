
import React from 'react';
import { AlertCircle, RefreshCw, WifiOff, ServerCrash } from 'lucide-react';

interface FallbackUIProps {
  type: 'offline' | 'error' | 'loading';
  message?: string;
  onRetry: () => void;
}

const FallbackUI: React.FC<FallbackUIProps> = ({ type, message, onRetry }) => {
  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh] text-center p-6 animate-in fade-in duration-700">
      <div className="relative mb-8">
        <div className="absolute inset-0 bg-blue-500/20 blur-3xl rounded-full" />
        <div className="relative w-24 h-24 rounded-full border border-white/10 flex items-center justify-center bg-slate-900/50 backdrop-blur-xl">
          {type === 'offline' && <WifiOff className="w-10 h-10 text-slate-400" />}
          {type === 'error' && <ServerCrash className="w-10 h-10 text-red-400 animate-pulse" />}
          {type === 'loading' && <RefreshCw className="w-10 h-10 text-blue-400 animate-spin" />}
        </div>
      </div>
      
      <h2 className="text-2xl font-bold mb-2 tracking-tight">
        {type === 'offline' ? '连接丢失' : type === 'error' ? '系统核心异常' : '正在唤醒核心...'}
      </h2>
      <p className="text-slate-400 max-w-md mx-auto mb-8 text-sm leading-relaxed">
        {message || '我们无法连接到 SeeSea 命令中心，可能是服务器正在维护或您的网络连接已断开。'}
      </p>
      
      <button 
        onClick={onRetry}
        className="flex items-center gap-2 bg-white text-black px-8 py-3 rounded-2xl font-bold hover:bg-slate-200 transition-all active:scale-95 shadow-xl shadow-white/10"
      >
        <RefreshCw className="w-4 h-4" />
        重新尝试连接
      </button>

      <div className="mt-12 flex gap-4 text-[10px] text-slate-600 font-mono uppercase tracking-widest">
        <span>Code: {type.toUpperCase()}</span>
        <span>•</span>
        <span>Node: SH-EDGE-01</span>
      </div>
    </div>
  );
};

export default FallbackUI;


import React, { useState } from 'react';
import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { Activity, Zap, HardDrive, Cpu, Network, ArrowUpRight, ArrowDownRight, Database, RefreshCw } from 'lucide-react';
import CustomSelect from './CustomSelect';
import { useSystemStatus } from '../hooks/useSystemStatus';

const StatCard: React.FC<{ title: string; value: string | number; trend?: string; isUp?: boolean; icon: any; color: string; loading?: boolean; }> = ({ title, value, trend, isUp, icon: Icon, color, loading }) => (
  <div className="glass-panel p-3.5 md:p-6 rounded-2xl hover:border-white/20 transition-all duration-300 group relative overflow-hidden">
    {loading ? (
      <div className="animate-pulse space-y-4">
        <div className="flex justify-between items-start"><div className="w-10 h-10 bg-slate-800 rounded-xl" /><div className="w-12 h-4 bg-slate-800 rounded" /></div>
        <div className="space-y-2"><div className="w-16 h-3 bg-slate-800 rounded" /><div className="w-24 h-6 bg-slate-800 rounded" /></div>
      </div>
    ) : (
      <>
        <div className={`absolute top-0 right-0 w-24 h-24 blur-3xl opacity-10 rounded-full -mr-12 -mt-12 transition-all group-hover:opacity-20 ${color}`} />
        <div className="flex justify-between items-start mb-2 md:mb-4 relative z-10">
          <div className={`p-2 md:p-3 rounded-xl ${color} bg-opacity-10 group-hover:bg-opacity-20 transition-all`}><Icon className={`w-4 h-4 md:w-6 md:h-6 ${color.replace('bg-', 'text-')}`} /></div>
          {trend && <div className={`flex items-center gap-0.5 text-[10px] md:text-xs font-bold ${isUp ? 'text-green-400' : 'text-red-400'}`}>{isUp ? <ArrowUpRight className="w-3 h-3" /> : <ArrowDownRight className="w-3 h-3" />}{trend}</div>}
        </div>
        <div className="space-y-0.5 relative z-10">
          <h3 className="text-slate-500 text-[10px] md:text-sm font-bold truncate uppercase tracking-widest">{title}</h3>
          <p className="text-base md:text-2xl font-bold tracking-tight text-white">{value}</p>
        </div>
      </>
    )}
  </div>
);

const ResourceProgress: React.FC<{ icon: any; label: string; value: number; color: string }> = ({ icon: Icon, label, value, color }) => (
  <div className="space-y-2.5">
    <div className="flex justify-between items-center text-[10px]">
      <div className="flex items-center gap-2 text-slate-400 font-bold uppercase tracking-tight"><Icon className="w-3.5 h-3.5" /><span>{label}</span></div>
      <span className={`font-black ${value > 85 ? 'text-red-400 animate-pulse' : 'text-white'}`}>{value}%</span>
    </div>
    <div className="h-2 w-full bg-slate-900/80 rounded-full overflow-hidden p-[2px] border border-white/[0.05]">
      <div className={`h-full rounded-full ${color} transition-all duration-700 ease-out`} style={{ width: `${value}%` }} />
    </div>
  </div>
);

const Dashboard: React.FC = () => {
  const [timeRange, setTimeRange] = useState('24h');
  const [previousStats, setPreviousStats] = useState<ApiStatsResponse | null>(null);
  const { status, loading } = useSystemStatus();

  // 保存前一次的统计数据用于计算趋势
  React.useEffect(() => {
    if (status?.search_stats && !loading) {
      setPreviousStats(prev => {
        // 只在有足够数据时更新（至少 5 秒间隔）
        if (!prev || Math.abs(Date.now() - Date.now()) > 5000) {
          return status.search_stats;
        }
        return prev;
      });
    }
  }, [status?.search_stats, loading]);

  // 计算趋势
  const calculateTrend = (current: number, previous: number): { value: string; isUp: boolean } => {
    if (previous === 0) return { value: "0%", isUp: true };
    const diff = current - previous;
    const percent = Math.abs((diff / previous) * 100).toFixed(1);
    return {
      value: `${diff >= 0 ? '+' : ''}${percent}%`,
      isUp: diff >= 0
    };
  };

  const searchTrend = React.useMemo(() => {
    if (!status?.search_stats || !previousStats) return { value: "0%", isUp: true };
    return calculateTrend(
      status.search_stats.total_searches,
      previousStats.total_searches
    );
  }, [status?.search_stats, previousStats]);

  const cacheTrend = React.useMemo(() => {
    if (!status?.search_stats || !previousStats) return { value: "0%", isUp: true };
    return calculateTrend(
      status.search_stats.cache_hit_rate * 100,
      previousStats.cache_hit_rate * 100
    );
  }, [status?.search_stats, previousStats]);

  // 将搜索历史数据转换为图表数据格式
  const chartData = React.useMemo(() => {
    if (!status?.search_stats.search_history || status.search_stats.search_history.length === 0) {
      return [];
    }

    return status.search_stats.search_history.map(entry => {
      const date = new Date(entry.hour * 3600 * 1000);
      const hours = date.getHours().toString().padStart(2, '0');
      const minutes = date.getMinutes().toString().padStart(2, '0');
      return {
        time: `${hours}:${minutes}`,
        requests: entry.count
      };
    });
  }, [status?.search_stats.search_history]);

  return (
    <div className="space-y-4 md:space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 md:gap-6">
        <StatCard title="查询总量" value={status?.search_stats.total_searches.toLocaleString() || "0"} trend={searchTrend.value} isUp={searchTrend.isUp} icon={Activity} color="bg-blue-500" loading={loading} />
        <StatCard title="缓存命中率" value={`${((status?.search_stats.cache_hit_rate || 0) * 100).toFixed(1)}%`} trend={cacheTrend.value} isUp={cacheTrend.isUp} icon={Database} color="bg-emerald-500" loading={loading} />
        <StatCard title="引擎异常" value={status?.search_stats.engine_failures || 0} trend={status?.search_stats.engine_failures ? "需关注" : "正常"} isUp={false} icon={Zap} color="bg-purple-500" loading={loading} />
        <StatCard title="系统在线" value={status ? `${Math.floor(status.uptime_seconds / 3600)}h` : "0h"} icon={RefreshCw} color="bg-amber-500" loading={loading} />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 glass-panel p-4 md:p-6 rounded-3xl overflow-hidden relative">
          <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-8 gap-4 relative z-10">
            <div><h2 className="text-lg font-bold flex items-center gap-2"><div className="w-1 h-5 bg-blue-500 rounded-full" />搜索吞吐动态</h2><p className="text-sm text-slate-500 ml-3">实时监控全球搜索请求频率</p></div>
            <CustomSelect value={timeRange} onChange={setTimeRange} options={[{label: '最近 24 小时', value: '24h'}, {label: '最近 7 天', value: '7d'}]} />
          </div>
          <div className="h-[240px] md:h-[320px] w-full relative z-10">
            {chartData.length > 0 ? (
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={chartData}>
                  <defs><linearGradient id="colorReq" x1="0" y1="0" x2="0" y2="1"><stop offset="5%" stopColor="#3b82f6" stopOpacity={0.25}/><stop offset="95%" stopColor="#3b82f6" stopOpacity={0}/></linearGradient></defs>
                  <CartesianGrid strokeDasharray="4 4" vertical={false} stroke="rgba(255,255,255,0.02)" />
                  <XAxis dataKey="time" axisLine={false} tickLine={false} tick={{fill: '#475569', fontSize: 10}} dy={10} />
                  <YAxis axisLine={false} tickLine={false} tick={{fill: '#475569', fontSize: 10}} />
                  <Tooltip contentStyle={{ backgroundColor: 'rgba(3, 7, 18, 0.95)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: '20px' }} />
                  <Area type="monotone" dataKey="requests" stroke="#3b82f6" strokeWidth={3} fillOpacity={1} fill="url(#colorReq)" animationDuration={1000} />
                </AreaChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex items-center justify-center h-full text-slate-500">
                暂无搜索数据
              </div>
            )}
          </div>
        </div>

        <div className="glass-panel p-6 rounded-3xl flex flex-col relative overflow-hidden">
          <h2 className="text-lg font-bold mb-6 flex items-center gap-2"><div className="w-1 h-5 bg-purple-500 rounded-full" />内核资源消耗</h2>
          <div className="space-y-6 flex-1 relative z-10">
            <ResourceProgress icon={Cpu} label="CPU 负荷" value={Math.round((status?.resources.cpu_usage || 0) * 100)} color="bg-blue-500" />
            <ResourceProgress icon={HardDrive} label="内存占用" value={Math.round((status?.resources.memory_usage || 0) * 100)} color="bg-purple-500" />
            <ResourceProgress icon={Network} label="网络吞吐" value={Math.round((status?.resources.network_io_usage || 0) * 100)} color="bg-amber-500" />
            <ResourceProgress icon={Database} label="磁盘 I/O" value={Math.round((status?.resources.disk_io_usage || 0) * 100)} color="bg-emerald-500" />
          </div>
        </div>
      </div>
    </div>
  );
};

export default Dashboard;


import { useState, useEffect, useCallback } from 'react';
import { fetchLogsFiles, fetchLogsRead, fetchLogsTail, LogsFilesResponse } from '../services/api';

interface LogEntry {
  id: string;
  timestamp: string;
  level: 'info' | 'warn' | 'error' | 'debug';
  message: string;
  service: string;
}

const INITIAL_LOGS: LogEntry[] = [];

// 解析日志行，提取时间戳、级别、服务和消息
const parseLogLine = (line: string): LogEntry | null => {
  // 示例日志格式：2025-01-28 08:15:32 INFO [GATEWAY] API Gateway initialized
  const timeMatch = line.match(/^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})/);
  const levelMatch = line.match(/\b(INFO|WARN|ERROR|DEBUG)\b/i);
  
  if (!timeMatch || !levelMatch) return null;

  const timestamp = timeMatch[1].split(' ')[1]; // 只保留时间部分
  const level = levelMatch[1].toLowerCase() as 'info' | 'warn' | 'error' | 'debug';
  
  // 提取服务（在方括号中的内容）
  const serviceMatch = line.match(/\[([^\]]+)\]/);
  const service = serviceMatch ? serviceMatch[1] : 'SYSTEM';
  
  // 提取消息（服务后面的内容）
  const message = line.substring(serviceMatch ? serviceMatch.index + serviceMatch[0].length : line.indexOf(levelMatch[0]) + levelMatch[0].length).trim();

  return {
    id: Math.random().toString(),
    timestamp,
    level,
    message,
    service
  };
};

// 将后端日志行转换为 LogEntry 格式
const convertToLogEntries = (lines: string[]): LogEntry[] => {
  return lines
    .map(line => parseLogLine(line))
    .filter((entry): entry is LogEntry => entry !== null);
};

export const useLogs = (maxLogs = 100) => {
  const [logs, setLogs] = useState<LogEntry[]>(INITIAL_LOGS);
  const [isLive, setIsLive] = useState(false);
  const [filter, setFilter] = useState('all');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [logFiles, setLogFiles] = useState<LogsFilesResponse['files']>([]);
  const [currentFile, setCurrentFile] = useState<string | null>(null);

  // 加载日志文件列表
  const loadLogFiles = useCallback(async () => {
    try {
      const files = await fetchLogsFiles('python');
      setLogFiles(files.files);
      if (files.files.length > 0 && !currentFile) {
        setCurrentFile(files.files[0].name);
      }
    } catch (err) {
      setError('Failed to load log files');
    }
  }, [currentFile]);

  // 加载日志内容
  const loadLogs = useCallback(async () => {
    if (!currentFile) return;
    
    setIsLoading(true);
    setError(null);
    try {
      const data = await fetchLogsRead(currentFile, maxLogs, 0);
      const entries = convertToLogEntries(data.logs);
      setLogs(entries.reverse()); // 最新的日志在前面
    } catch (err) {
      setError('Failed to load logs');
      console.error('Error loading logs:', err);
    } finally {
      setIsLoading(false);
    }
  }, [currentFile, maxLogs]);

  // 初始加载
  useEffect(() => {
    loadLogFiles();
  }, [loadLogFiles]);

  // 加载日志内容
  useEffect(() => {
    loadLogs();
  }, [loadLogs]);

  // 实时日志更新
  useEffect(() => {
    if (!isLive) return;

    const interval = setInterval(async () => {
      try {
        const data = await fetchLogsTail(20);
        const entries = convertToLogEntries(data.logs);
        setLogs(prev => {
          const newEntries = entries.filter(
            entry => !prev.some(p => p.message === entry.message && p.timestamp === entry.timestamp)
          );
          return [...newEntries, ...prev].slice(0, maxLogs);
        });
      } catch (err) {
        console.error('Error fetching live logs:', err);
      }
    }, 3000);

    return () => clearInterval(interval);
  }, [isLive, maxLogs]);

  const clearLogs = () => setLogs([]);

  const refreshLogs = () => {
    loadLogs();
    loadLogFiles();
  };

  return { 
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
  };
};

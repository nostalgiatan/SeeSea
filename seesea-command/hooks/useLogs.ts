
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
  // 支持多种日志格式：
  // 格式1: 2026-01-29T12:52:36.323249Z INFO seesea_api::api::handlers::static_files: message
  // 格式2: 2025-01-28 08:15:32 INFO [GATEWAY] message
  
  // 尝试匹配 ISO 8601 格式 (格式1)
  const isoTimeMatch = line.match(/^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})/);
  // 尝试匹配标准格式 (格式2)
  const stdTimeMatch = !isoTimeMatch ? line.match(/^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})/) : null;
  
  const timeMatch = isoTimeMatch || stdTimeMatch;
  
  // 匹配日志级别
  const levelMatch = line.match(/\b(INFO|WARN|ERROR|DEBUG)\b/i);
  
  if (!timeMatch || !levelMatch) return null;

  // 格式化时间戳
  let timestamp: string;
  if (isoTimeMatch) {
    // ISO 8601 格式: 2026-01-29T12:52:36 -> 12:52:36
    const timePart = isoTimeMatch[1].split('T')[1];
    timestamp = timePart;
  } else {
    // 标准格式: 2025-01-28 08:15:32 -> 08:15:32
    timestamp = stdTimeMatch![1].split(' ')[1];
  }
  
  const level = levelMatch[1].toLowerCase() as 'info' | 'warn' | 'error' | 'debug';
  
  // 提取服务名（可能是模块路径或方括号中的内容）
  let service = 'SYSTEM';
  
  // 尝试提取模块路径（格式1）
  const moduleMatch = line.match(/INFO|WARN|ERROR|DEBUG\s+(\S+)::/);
  if (moduleMatch) {
    // 提取模块名的最后一部分
    const parts = moduleMatch[1].split('::');
    service = parts[parts.length - 1].toUpperCase();
  } else {
    // 尝试提取方括号中的内容（格式2）
    const bracketMatch = line.match(/\[([^\]]+)\]/);
    if (bracketMatch) {
      service = bracketMatch[1].toUpperCase();
    }
  }
  
  // 提取消息（第一个冒号后面的内容，或者日志级别后面的内容）
  const firstColonIndex = line.indexOf(':');
  let message: string;
  
  if (firstColonIndex > -1 && firstColonIndex > line.indexOf(levelMatch[0]) + levelMatch[0].length) {
    message = line.substring(firstColonIndex + 1).trim();
  } else {
    // 使用日志级别后的内容
    const levelIndex = line.indexOf(levelMatch[0]);
    message = line.substring(levelIndex + levelMatch[0].length).trim();
  }

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

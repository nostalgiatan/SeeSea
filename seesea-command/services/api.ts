
import {
  SystemStatusResponse,
  ApiHealthResponse,
  InternalEngineStatus,
  ApiStatsResponse,
  CacheStatsResponse,
  InternalResourceStatusResponse,
  LogsDirectoryResponse,
  LogsFilesResponse,
  LogsReadResponse,
  LogsTailResponse,
  LogsErrorsResponse
} from '../types';

// 从后端注入的配置中获取 API 基础 URL
const BASE_URL = (typeof window !== 'undefined' && (window as any).__SEESEA_CONFIG__?.API_BASE_URL) || '';

// Add a helper for fetch with timeout
const fetchWithTimeout = async (url: string, options: any = {}, timeout = 5000) => {
  const controller = new AbortController();
  const id = setTimeout(() => controller.abort(), timeout);
  try {
    const response = await fetch(url, {
      ...options,
      signal: controller.signal
    });
    clearTimeout(id);
    return response;
  } catch (err) {
    clearTimeout(id);
    throw err;
  }
};

const safeJson = async (response: Response) => {
  if (!response.ok) throw new Error(`API_ERROR: ${response.status}`);
  const contentType = response.headers.get("content-type");
  if (contentType && contentType.includes("application/json")) {
    return response.json();
  }
  throw new Error("API_ERROR: Non-JSON response");
};

/**
 * System Health
 */
export async function fetchHealth(): Promise<ApiHealthResponse> {
  const response = await fetchWithTimeout(`${BASE_URL}/health`);
  return await safeJson(response);
}

/**
 * System Status
 */
export async function fetchSystemStatus(): Promise<SystemStatusResponse> {
  const response = await fetchWithTimeout(`${BASE_URL}/internal/system/status`);
  return await safeJson(response);
}

/**
 * Engines
 */
export async function fetchEngines(): Promise<InternalEngineStatus[]> {
  const response = await fetchWithTimeout(`${BASE_URL}/internal/engines/status`);
  return await safeJson(response);
}

export async function toggleEngine(engine_name: string, enabled: boolean): Promise<any> {
  const response = await fetchWithTimeout(`${BASE_URL}/internal/engines/toggle`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ engine_name, enabled })
  });
  return await safeJson(response);
}

/**
 * Cache
 */
export async function clearCache(pattern?: string): Promise<any> {
  const response = await fetchWithTimeout(`${BASE_URL}/internal/cache/clear`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pattern })
  });
  return await safeJson(response);
}

/**
 * Config
 */
export async function fetchConfig(): Promise<any> {
  const response = await fetchWithTimeout(`${BASE_URL}/internal/config/get`);
  return await safeJson(response);
}

export async function updateConfig(key: string, value: any): Promise<any> {
  const response = await fetchWithTimeout(`${BASE_URL}/internal/config/update`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ key, value })
  });
  return await safeJson(response);
}

/**
 * Logs - 日志管理
 */
export async function fetchLogsDirectory(): Promise<LogsDirectoryResponse> {
  const response = await fetchWithTimeout(`${BASE_URL}/internal/logs/directory`);
  return await safeJson(response);
}

export async function fetchLogsFiles(log_type: string = 'all'): Promise<LogsFilesResponse> {
  const response = await fetchWithTimeout(
    `${BASE_URL}/internal/logs/files?log_type=${log_type}`
  );
  return await safeJson(response);
}

export async function fetchLogsRead(
  file: string,
  lines: number = 100,
  offset: number = 0
): Promise<LogsReadResponse> {
  const response = await fetchWithTimeout(
    `${BASE_URL}/internal/logs/read?file=${encodeURIComponent(file)}&lines=${lines}&offset=${offset}`
  );
  return await safeJson(response);
}

export async function fetchLogsTail(lines: number = 50): Promise<LogsTailResponse> {
  const response = await fetchWithTimeout(
    `${BASE_URL}/internal/logs/tail?lines=${lines}`
  );
  return await safeJson(response);
}

export async function fetchLogsErrors(): Promise<LogsErrorsResponse> {
  const response = await fetchWithTimeout(`${BASE_URL}/internal/logs/errors`);
  return await safeJson(response);
}

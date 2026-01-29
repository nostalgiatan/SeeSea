
export interface ApiHealthResponse {
  status: string;
  version: string;
  available_engines: number;
  total_engines: number;
}

export interface InternalResourceStatusResponse {
  cpu_usage: number;
  memory_usage: number;
  disk_io_usage: number;
  network_io_usage: number;
  available_memory: number;
  available_disk: number;
  total_disk: number;
  load_avg_1: number;
  load_avg_5: number;
  load_avg_15: number;
  disk_usage_percent: number;
  controller_running: boolean;
}

export interface InternalEngineStatus {
  name: string;
  enabled: boolean;
  temporarily_disabled: boolean;
  consecutive_failures: number;
  total_requests: number;
  failed_requests: number;
  success_rate: number;
  last_updated: number;
  avg_response_time_ms: number | null;
  disabled_reason: string | null;
}

export interface ApiStatsResponse {
  total_searches: number;
  cache_hits: number;
  cache_misses: number;
  cache_hit_rate: number;
  engine_failures: number;
  timeouts: number;
  search_history: SearchHistoryEntry[];
}

export interface SearchHistoryEntry {
  hour: number;
  count: number;
}

export interface CacheStatsResponse {
  total_entries: number;
  size_bytes: number;
  hits: number;
  misses: number;
  hit_rate: number;
  writes: number;
  deletes: number;
  evictions: number;
  avg_get_latency_ms: number;
  avg_set_latency_ms: number;
}

export interface SystemStatusResponse {
  uptime_seconds: number;
  resources: InternalResourceStatusResponse;
  search_stats: ApiStatsResponse;
  engine_statuses: InternalEngineStatus[];
  cache_stats?: {
    total_keys: number;
    total_size_bytes: number;
    hit_rate: number;
  } | null;
}

export enum NavigationTab {
  OVERVIEW = 'overview',
  ENGINES = 'engines',
  CACHE = 'cache',
  CONFIG = 'config',
  LOGS = 'logs'
}

// 日志相关类型
export interface LogFileMetadata {
  name: string;
  size: number;
  modified: number | null;
}

export interface LogsDirectoryResponse {
  log_dir: string;
  exists: boolean;
  readable: boolean;
}

export interface LogsFilesResponse {
  log_dir: string;
  exists: boolean;
  log_type: string;
  total_count: number;
  files: LogFileMetadata[];
  python_count: number;
  rust_count: number;
}

export interface LogsReadResponse {
  file: string;
  path: string;
  total_lines: number;
  offset: number;
  lines_returned: number;
  requested_lines: number;
  logs: string[];
}

export interface LogsTailResponse {
  logs: string[];
  lines_requested: number;
  lines_returned: number;
  log_dir: string;
  log_file: string | null;
}

export interface LogsErrorsResponse {
  errors: string[];
  total_count: number;
  log_dir: string;
  log_file: string | null;
}

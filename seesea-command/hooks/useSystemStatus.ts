
import { useState, useEffect, useCallback } from 'react';
import { SystemStatusResponse } from '../types';
import { fetchSystemStatus } from '../services/api';

export const useSystemStatus = (pollingInterval = 5000) => {
  const [status, setStatus] = useState<SystemStatusResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    try {
      const res = await fetchSystemStatus();
      setStatus(res);
      setError(null);
    } catch (err) {
      setError('无法同步系统状态');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, pollingInterval);
    return () => clearInterval(interval);
  }, [loadData, pollingInterval]);

  return { status, loading, error, refresh: loadData };
};

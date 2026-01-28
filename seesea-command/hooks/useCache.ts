
import { useState, useEffect, useCallback } from 'react';
import { clearCache, fetchSystemStatus } from '../services/api';

export const useCache = () => {
  const [cacheData, setCacheData] = useState<{
    total_keys: number;
    size_bytes: number;
    hit_rate: number;
  } | null>(null);
  const [loading, setLoading] = useState(true);
  const [isClearing, setIsClearing] = useState(false);
  const [showToast, setShowToast] = useState(false);

  const loadCacheStats = useCallback(async () => {
    try {
      const status = await fetchSystemStatus();
      if (status.cache_stats) {
        setCacheData({
          total_keys: status.cache_stats.total_keys,
          size_bytes: status.cache_stats.total_size_bytes,
          hit_rate: status.cache_stats.hit_rate
        });
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadCacheStats();
  }, [loadCacheStats]);

  const handleClear = async (pattern?: string) => {
    setIsClearing(true);
    try {
      await clearCache(pattern);
      setShowToast(true);
      // Fix: Replace undefined setShowSuccess with setShowToast
      setTimeout(() => setShowToast(false), 3000);
      await loadCacheStats();
    } catch (err) {
      console.error("Failed to clear cache:", err);
    } finally {
      setIsClearing(false);
      setShowToast(true);
      setTimeout(() => setShowToast(false), 3000);
    }
  };

  return { cacheData, loading, isClearing, showToast, handleClear, refresh: loadCacheStats };
};

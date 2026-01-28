
import { useState, useEffect, useCallback, useRef } from 'react';
import { InternalEngineStatus } from '../types';
import { fetchEngines, toggleEngine } from '../services/api';

export const useEngines = (pollingInterval = 10000) => {
  const [engines, setEngines] = useState<InternalEngineStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [processing, setProcessing] = useState<string | null>(null);
  // Fix: Use ReturnType<typeof setInterval> instead of NodeJS.Timeout for frontend environment
  const pollingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const loadEngines = useCallback(async (isSilent = false) => {
    if (!isSilent) setLoading(true);
    try {
      const data = await fetchEngines();
      if (data && Array.isArray(data)) {
        setEngines(data);
      }
    } catch (err) {
      console.error("Failed to load engines:", err);
    } finally {
      if (!isSilent) setLoading(false);
    }
  }, []);

  // Polling setup
  useEffect(() => {
    loadEngines();
    pollingTimerRef.current = setInterval(() => {
      loadEngines(true);
    }, pollingInterval);

    return () => {
      if (pollingTimerRef.current) clearInterval(pollingTimerRef.current);
    };
  }, [loadEngines, pollingInterval]);

  const handleToggle = useCallback(async (name: string, currentEnabled: boolean) => {
    if (processing) return; // Prevent multiple simultaneous toggles
    
    setProcessing(name);
    try {
      // Optimistic Update
      setEngines(prev => prev.map(e => 
        e.name === name ? { ...e, enabled: !currentEnabled } : e
      ));

      const result = await toggleEngine(name, !currentEnabled);
      
      // If the API call fails or returns error, we should ideally revert, 
      // but our safe mock API always returns success.
      // Re-fetching latest state to be sure
      await loadEngines(true);
    } catch (err) {
      console.error(`Failed to toggle engine ${name}:`, err);
      // Revert optimistic update on error
      setEngines(prev => prev.map(e => 
        e.name === name ? { ...e, enabled: currentEnabled } : e
      ));
    } finally {
      setProcessing(null);
    }
  }, [processing, loadEngines]);

  return { 
    engines, 
    loading, 
    processing, 
    toggle: handleToggle, 
    refresh: () => loadEngines(false) 
  };
};

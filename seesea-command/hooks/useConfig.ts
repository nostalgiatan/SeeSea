
import { useState, useEffect, useCallback } from 'react';
import { fetchConfig, updateConfig } from '../services/api';

export const useConfig = () => {
  const [config, setConfig] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [showSuccess, setShowSuccess] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const load = async () => {
      setLoading(true);
      try {
        const data = await fetchConfig();
        setConfig(data || null);
      } catch (err) {
        setError("获取配置失败");
      } finally {
        setLoading(false);
      }
    };
    load();
  }, []);

  const updateField = (key: string, value: any) => {
    setConfig((prev: any) => prev ? ({ ...prev, [key]: value }) : null);
  };

  const save = async () => {
    if (!config) return;
    setIsSaving(true);
    setError(null);
    try {
      await updateConfig('search_timeout_ms', config.search_timeout_ms);
      setShowSuccess(true);
      setTimeout(() => setShowSuccess(false), 3000);
    } catch (err) {
      setError("配置部署失败，请检查网络");
    } finally {
      setIsSaving(false);
    }
  };

  return { config, loading, isSaving, showSuccess, error, updateField, save };
};

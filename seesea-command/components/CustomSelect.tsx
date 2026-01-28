
import React, { useState, useRef, useEffect } from 'react';
import { ChevronDown, Check } from 'lucide-react';

interface Option {
  label: string;
  value: string;
}

interface CustomSelectProps {
  options: Option[];
  value: string;
  onChange: (val: string) => void;
}

const CustomSelect: React.FC<CustomSelectProps> = ({ options, value, onChange }) => {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) setIsOpen(false);
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const selectedLabel = options.find(o => o.value === value)?.label;

  return (
    <div className="relative" ref={containerRef}>
      <button 
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-2 bg-slate-900/80 border border-white/10 hover:border-white/20 px-4 py-2 rounded-xl text-xs font-medium transition-all min-w-[140px] justify-between"
      >
        <span>{selectedLabel}</span>
        <ChevronDown className={`w-3.5 h-3.5 transition-transform duration-300 ${isOpen ? 'rotate-180' : ''}`} />
      </button>

      {isOpen && (
        <div className="absolute top-full right-0 mt-2 w-full min-w-[160px] glass-panel rounded-xl overflow-hidden shadow-2xl z-50 animate-in fade-in zoom-in-95 duration-200">
          <div className="py-1">
            {options.map((opt) => (
              <button
                key={opt.value}
                onClick={() => {
                  onChange(opt.value);
                  setIsOpen(false);
                }}
                className="w-full flex items-center justify-between px-4 py-2.5 text-xs hover:bg-white/5 transition-colors text-left"
              >
                <span className={value === opt.value ? 'text-blue-400 font-bold' : 'text-slate-300'}>
                  {opt.label}
                </span>
                {value === opt.value && <Check className="w-3 h-3 text-blue-400" />}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default CustomSelect;

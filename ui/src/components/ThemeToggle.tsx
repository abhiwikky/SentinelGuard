import { Sun, Moon } from 'lucide-react';
import type { Theme } from '../hooks/useTheme';

interface ThemeToggleProps {
  theme: Theme;
  onToggle: () => void;
}

export default function ThemeToggle({ theme, onToggle }: ThemeToggleProps) {
  const isDark = theme === 'dark';

  return (
    <button
      type="button"
      onClick={onToggle}
      className="theme-toggle-track"
      aria-label={`Switch to ${isDark ? 'light' : 'dark'} mode`}
      title={`Switch to ${isDark ? 'light' : 'dark'} mode`}
    >
      {/* Background icons */}
      <span className="theme-toggle-icon theme-toggle-icon--sun">
        <Sun size={11} />
      </span>
      <span className="theme-toggle-icon theme-toggle-icon--moon">
        <Moon size={11} />
      </span>

      {/* Sliding knob */}
      <span
        className="theme-toggle-knob"
        style={{
          transform: isDark ? 'translateX(0)' : 'translateX(22px)',
        }}
      >
        {isDark ? <Moon size={12} /> : <Sun size={12} />}
      </span>
    </button>
  );
}

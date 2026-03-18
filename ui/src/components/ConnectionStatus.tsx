import type { ConnectionState } from '../types';

interface Props {
  state: ConnectionState;
  error: string | null;
}

export default function ConnectionStatus({ state, error }: Props) {
  const config = {
    connected: {
      bg: 'bg-green-950/50 border-green-800',
      dot: 'bg-green-400',
      text: 'text-green-300',
      label: 'All systems operational',
    },
    degraded: {
      bg: 'bg-yellow-950/50 border-yellow-800',
      dot: 'bg-yellow-400',
      text: 'text-yellow-300',
      label: 'Degraded connectivity',
    },
    disconnected: {
      bg: 'bg-red-950/50 border-red-800',
      dot: 'bg-red-400',
      text: 'text-red-300',
      label: 'Disconnected',
    },
  }[state];

  return (
    <div className={`flex items-center justify-between px-4 py-2 rounded-lg border ${config.bg} animate-fade-in`}>
      <div className="flex items-center gap-2">
        <div className={`live-dot ${config.dot}`} />
        <span className={`text-sm font-medium ${config.text}`}>
          {config.label}
        </span>
      </div>
      {error && (
        <span className="text-xs text-gray-500">{error}</span>
      )}
    </div>
  );
}

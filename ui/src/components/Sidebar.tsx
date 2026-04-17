import type { PageId } from '../App';
import {
  LayoutDashboard,
  Activity,
  Bell,
  ShieldAlert,
  ChevronLeft,
  ChevronRight,
  Shield,
} from 'lucide-react';

interface SidebarProps {
  activePage: PageId;
  onNavigate: (page: PageId) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
  alertCount: number;
  quarantineCount: number;
}

const NAV_ITEMS: { id: PageId; label: string; icon: typeof LayoutDashboard; section?: string }[] = [
  { id: 'dashboard', label: 'Overview', icon: LayoutDashboard, section: 'MONITOR' },
  { id: 'processes', label: 'Process Risk', icon: Activity },
  { id: 'alerts', label: 'Alert Feed', icon: Bell, section: 'RESPONSE' },
  { id: 'quarantine', label: 'Quarantine', icon: ShieldAlert },
];

export default function Sidebar({
  activePage,
  onNavigate,
  collapsed,
  onToggleCollapse,
  alertCount,
  quarantineCount,
}: SidebarProps) {
  const getBadge = (id: PageId) => {
    if (id === 'alerts' && alertCount > 0) return alertCount;
    if (id === 'quarantine' && quarantineCount > 0) return quarantineCount;
    return null;
  };

  return (
    <aside
      className="flex flex-col h-full shrink-0 transition-all duration-300 ease-in-out"
      style={{
        width: collapsed ? '72px' : '240px',
        background: 'var(--bg-surface)',
        borderRight: '1px solid rgba(255,255,255,0.04)',
      }}
    >
      {/* Logo */}
      <div className="flex items-center gap-3 px-4 py-5 shrink-0" style={{ borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
        <div
          className="neu-pressed-sm flex items-center justify-center shrink-0"
          style={{ width: 38, height: 38 }}
        >
          <Shield size={20} style={{ color: 'var(--accent-blue)' }} />
        </div>
        {!collapsed && (
          <div className="animate-fade-in overflow-hidden">
            <div className="text-base font-bold tracking-tight whitespace-nowrap" style={{ color: 'var(--text-primary)' }}>
              SentinelGuard
            </div>
            <div
              className="text-[9px] font-bold uppercase tracking-widest whitespace-nowrap"
              style={{ color: 'var(--text-muted)' }}
            >
              SOC Platform
            </div>
          </div>
        )}
      </div>

      {/* Navigation */}
      <nav className="flex-1 py-4 px-3 overflow-y-auto custom-scroll">
        {NAV_ITEMS.map((item, idx) => {
          const Icon = item.icon;
          const badge = getBadge(item.id);

          return (
            <div key={item.id}>
              {/* Section header */}
              {item.section && (
                <div
                  className="mt-2 mb-2"
                  style={{
                    opacity: collapsed ? 0 : 1,
                    height: collapsed ? 0 : 'auto',
                    transition: 'all 0.2s',
                    overflow: 'hidden',
                  }}
                >
                  <div className="section-label">{item.section}</div>
                </div>
              )}

              {/* Nav item */}
              <button
                type="button"
                onClick={() => onNavigate(item.id)}
                className={`nav-item w-full mb-1 ${activePage === item.id ? 'active' : ''}`}
                title={collapsed ? item.label : undefined}
                style={{ justifyContent: collapsed ? 'center' : 'flex-start', padding: collapsed ? '0.7rem' : undefined }}
              >
                <Icon size={18} style={{ flexShrink: 0 }} />
                {!collapsed && (
                  <>
                    <span className="flex-1 text-left whitespace-nowrap">{item.label}</span>
                    {badge !== null && (
                      <span
                        className="text-[10px] font-bold px-1.5 py-0.5 rounded-full"
                        style={{
                          background: item.id === 'alerts'
                            ? 'rgba(239,68,68,0.15)'
                            : 'rgba(168,85,247,0.15)',
                          color: item.id === 'alerts' ? '#f87171' : '#c084fc',
                          minWidth: '20px',
                          textAlign: 'center',
                        }}
                      >
                        {badge > 99 ? '99+' : badge}
                      </span>
                    )}
                  </>
                )}
              </button>
            </div>
          );
        })}
      </nav>

      {/* Collapse Toggle */}
      <div className="px-3 py-3 shrink-0" style={{ borderTop: '1px solid rgba(255,255,255,0.04)' }}>
        <button
          type="button"
          onClick={onToggleCollapse}
          className="neu-button w-full justify-center"
          style={{ padding: '0.5rem' }}
          aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          {collapsed ? <ChevronRight size={16} /> : <ChevronLeft size={16} />}
        </button>
      </div>
    </aside>
  );
}

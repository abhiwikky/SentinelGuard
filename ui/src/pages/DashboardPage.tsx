import { useAppData } from '../App';
import MetricsPanel from '../components/MetricsPanel';
import AlertFeed from '../components/AlertFeed';
import ProcessRiskSummary from '../components/ProcessRiskSummary';
import QuarantinePanel from '../components/QuarantinePanel';

export default function DashboardPage() {
  const { health, alerts, processes, quarantined, handleRelease } = useAppData();

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Row 1: Metrics */}
      <MetricsPanel health={health} />

      {/* Row 2: Process Risk Summary + Alert Feed */}
      <div className="grid grid-cols-1 xl:grid-cols-12 gap-6">
        <div className="xl:col-span-7">
          <ProcessRiskSummary processes={processes} />
        </div>
        <div className="xl:col-span-5">
          <AlertFeed alerts={alerts} compact />
        </div>
      </div>

      {/* Row 3: Quarantine */}
      <QuarantinePanel processes={quarantined} onRelease={handleRelease} />
    </div>
  );
}

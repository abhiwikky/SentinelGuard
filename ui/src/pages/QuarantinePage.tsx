import { useAppData } from '../App';
import QuarantinePanel from '../components/QuarantinePanel';

export default function QuarantinePage() {
  const { quarantined, handleRelease } = useAppData();

  return (
    <div className="animate-fade-in max-w-5xl">
      <QuarantinePanel processes={quarantined} onRelease={handleRelease} />
    </div>
  );
}

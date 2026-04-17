import { useAppData } from '../App';
import AlertFeed from '../components/AlertFeed';

export default function AlertsPage() {
  const { alerts } = useAppData();

  return (
    <div className="animate-fade-in max-w-5xl">
      <AlertFeed alerts={alerts} />
    </div>
  );
}

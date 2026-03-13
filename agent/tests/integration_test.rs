use std::path::PathBuf;
use std::sync::Arc;

use sentinelguard_agent::config::Config;
use sentinelguard_agent::correlation::CorrelationEngine;
use sentinelguard_agent::database::Database;
use sentinelguard_agent::detectors::{DetectorManager, DetectorScores};
use sentinelguard_agent::events::{EventIngestion, EventType, FileEvent};

fn sample_event(process_id: u32, timestamp: i64) -> FileEvent {
    FileEvent {
        event_type: EventType::FileWrite,
        process_id,
        process_path: "C:\\Temp\\encryptor.exe".to_string(),
        file_path: format!("C:\\data\\{}.locked", process_id),
        bytes_read: 0,
        bytes_written: 32_768,
        timestamp,
        result: 0,
        entropy_preview: vec![0x91; 16],
    }
}

#[tokio::test]
async fn test_event_ingestion_persists_events() {
    let db_path = std::env::temp_dir().join(format!("sentinelguard-test-{}.db", uuid::Uuid::new_v4()));
    let db = Arc::new(Database::new(&db_path).await.unwrap());
    db.initialize_schema().await.unwrap();

    let (_detector_tx, detector_rx) = tokio::sync::mpsc::unbounded_channel();
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    let ingestion = EventIngestion::new(event_rx, _detector_tx.clone(), db.clone());
    let ingestion_task = tokio::spawn(async move { ingestion.start().await.unwrap() });

    event_tx.send(sample_event(1001, 10)).unwrap();
    drop(event_tx);
    ingestion_task.await.unwrap();
    drop(detector_rx);

    let (total_events, _, _, _) = db.get_system_metrics().await.unwrap();
    assert_eq!(total_events, 1);
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_correlation_engine_loads_real_model() {
    let model_path = PathBuf::from("..\\ml\\models\\sentinelguard_model.onnx");
    let engine = CorrelationEngine::new(&model_path).await.unwrap();
    let score = engine
        .infer(&DetectorScores {
            process_id: 42,
            process_path: "C:\\Temp\\encryptor.exe".to_string(),
            entropy_score: 0.9,
            mass_write_score: 0.8,
            mass_rename_delete_score: 0.7,
            ransom_note_score: 0.6,
            shadow_copy_score: 0.2,
            process_behavior_score: 0.7,
            file_extension_score: 0.9,
            event_rate: 12.0,
            avg_entropy_per_sec: 0.8,
            rename_delete_freq: 4.0,
            burst_interval: 1.0,
            num_detectors_firing: 5.0,
            file_diversity: 10.0,
            bytes_written_per_sec: 150_000.0,
            unique_extensions: 2.0,
            timestamp: 1234,
            triggered_detectors: vec!["EntropyDetector".to_string()],
        })
        .await
        .unwrap();

    assert!((0.0..=1.0).contains(&score));
}

#[tokio::test]
async fn test_detector_manager_tracks_real_scores() {
    let manager = Arc::new(DetectorManager::new(Arc::new(Config::default())).await.unwrap());
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let task_manager = manager.clone();
    let task = tokio::spawn(async move { task_manager.process_events(rx).await.unwrap() });

    for index in 0..55 {
        tx.send(sample_event(777, 500 + index as i64 % 10)).unwrap();
    }
    drop(tx);
    task.await.unwrap();

    let scores = manager.get_aggregated_scores().await;
    assert_eq!(scores.process_id, 777);
    assert!(scores.mass_write_score > 0.0);
    assert!(scores.num_detectors_firing >= 1.0);
}

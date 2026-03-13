use std::sync::Arc;

use sentinelguard_agent::config::Config;
use sentinelguard_agent::detectors::DetectorManager;
use sentinelguard_agent::events::{EventType, FileEvent};

fn sample_event(timestamp: i64, event_type: EventType, file_path: &str, bytes_written: u64) -> FileEvent {
    FileEvent {
        event_type,
        process_id: 1337,
        process_path: "C:\\Users\\abhi\\AppData\\Local\\Temp\\encryptor.exe".to_string(),
        file_path: file_path.to_string(),
        bytes_read: 0,
        bytes_written,
        timestamp,
        result: 0,
        entropy_preview: vec![0x91; 16],
    }
}

#[tokio::test]
async fn test_mass_write_detection_uses_real_window() {
    let manager = Arc::new(DetectorManager::new(Arc::new(Config::default())).await.unwrap());
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let manager_for_task = manager.clone();

    let task = tokio::spawn(async move { manager_for_task.process_events(rx).await.unwrap() });

    for index in 0..55 {
        tx.send(sample_event(100 + index as i64 % 10, EventType::FileWrite, &format!("C:\\data\\{}.txt", index), 8192))
            .unwrap();
    }
    drop(tx);
    task.await.unwrap();

    let scores = manager.get_aggregated_scores().await;
    assert!(scores.mass_write_score > 0.0);
}

#[tokio::test]
async fn test_detector_aggregation_records_triggered_detectors() {
    let manager = DetectorManager::new(Arc::new(Config::default())).await.unwrap();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let manager_ref = Arc::new(manager);
    let manager_for_task = manager_ref.clone();

    let task = tokio::spawn(async move { manager_for_task.process_events(rx).await.unwrap() });

    for index in 0..60 {
        tx.send(sample_event(
            200 + (index / 6) as i64,
            EventType::FileWrite,
            &format!("C:\\data\\{}.locked", index),
            16_384,
        ))
        .unwrap();
    }
    tx.send(sample_event(210, EventType::FileRename, "C:\\data\\renamed.locked", 0))
        .unwrap();
    drop(tx);
    task.await.unwrap();

    let scores = manager_ref.get_aggregated_scores().await;
    assert_eq!(scores.process_id, 1337);
    assert!(scores.mass_write_score > 0.0);
    assert!(scores.file_extension_score > 0.0);
    assert!(!scores.triggered_detectors.is_empty());
}

#[test]
fn test_default_model_path_matches_training_output() {
    let config = Config::default();
    assert_eq!(
        config.ml_model_path,
        std::path::PathBuf::from("models\\sentinelguard_model.onnx")
    );
}

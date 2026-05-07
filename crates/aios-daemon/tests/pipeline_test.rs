//! 验证 ProcessingEvent dispatch 逻辑。

use aios_daemon::pipeline::{should_stop_processing, ProcessingEvent};

#[test]
fn window_expiry_does_not_stop_processing_loop() {
    assert!(!should_stop_processing(&ProcessingEvent::WindowExpired));
}

#[test]
fn closed_raw_channel_stops_processing_loop() {
    assert!(should_stop_processing(&ProcessingEvent::RawChannelClosed));
}

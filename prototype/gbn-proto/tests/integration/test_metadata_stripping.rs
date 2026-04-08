//! Metadata stripping tests:
//! Verify that the sanitizer removes ALL identifying metadata from video files.

#[test]
fn test_mp4_metadata_stripped() {
    // TODO: Create MP4 with GPS, camera model, timestamp → sanitize → verify zero fields survive
}

#[test]
fn test_creation_timestamp_zeroed() {
    // TODO: Verify creation_time is epoch 0 after sanitization
}

#[test]
fn test_encoder_string_removed() {
    // TODO: Verify no "iPhone" or "Android" encoder strings survive
}

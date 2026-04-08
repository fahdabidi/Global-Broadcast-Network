//! # MCN Sanitizer
//!
//! Wraps FFmpeg to strip ALL identifying metadata from video files before
//! they enter the chunking and encryption pipeline.
//!
//! ## What Gets Stripped
//!
//! - EXIF data (GPS, camera model, lens info, orientation)
//! - Container-level tags (creation time, encoder software, title, comment)
//! - Encoder version strings ("recorded with iPhone 15 Pro")    
//! - Telemetry tracks (GoPro GPS, DJI flight data)
//! - Thumbnail/cover art embedded in container
//!
//! ## FFmpeg Strategy
//!
//! ```text
//! ffmpeg -i input.mp4 \
//!   -map_metadata -1 \           # strip all global metadata
//!   -map_chapters -1 \           # strip chapter markers
//!   -fflags +bitexact \          # deterministic output
//!   -flags:v +bitexact \         # deterministic video
//!   -flags:a +bitexact \         # deterministic audio
//!   -c copy \                    # no re-encoding (fast, lossless)
//!   -metadata creation_time=0 \  # zero out creation timestamp
//!   output.mp4
//! ```
//!
//! ## Verification
//!
//! After stripping, the sanitizer runs `ffprobe -show_format -show_streams`
//! and parses the output to verify that no identifying tags survive.

// TODO: Implement in Phase 1 execution
// - sanitize_video(input_path, output_path) -> Result<SanitizeReport>
// - verify_sanitization(file_path) -> Result<Vec<LeakedField>>

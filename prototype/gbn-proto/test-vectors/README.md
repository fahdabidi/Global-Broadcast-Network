# Test Vectors

This directory holds user-provided sample video files for testing the MCN pipeline.

## Required Files

| File | Size | Purpose |
|---|---|---|
| Any `.mp4` file | 10–50 MB | Rapid iteration and correctness tests |
| Any `.mp4` file | 500+ MB | Performance benchmarks (B1.1–B1.8) |

## How to Provide Test Videos

Place your video files directly in this directory. The deployment scripts will upload them to the Creator EC2 instance.

**Good test videos contain:**
- GPS/location metadata (for testing sanitizer stripping)
- Camera model and lens info in EXIF
- Encoder strings (e.g., "recorded with iPhone 15 Pro")
- Embedded thumbnails

For example, a video recorded on your phone contains all of the above by default.

## ⚠️ Important

- This directory is `.gitignore`d — video files are **never** committed to the repository.
- Do not use videos containing sensitive content — the prototype has no content encryption at rest during testing.

#!/bin/bash
# Combine recorded frames and audio into video using ffmpeg
# Usage: ./scripts/combine-recording.sh [output_dir] [output_filename]

set -e

OUTPUT_DIR="${1:-recording}"
OUTPUT_FILE="${2:-output.mp4}"

FRAMES_DIR="$OUTPUT_DIR/frames"
AUDIO_FILE="$OUTPUT_DIR/audio.wav"

# Check if recording directory exists
if [ ! -d "$FRAMES_DIR" ]; then
    echo "❌ Error: Frames directory not found: $FRAMES_DIR"
    exit 1
fi

if [ ! -f "$AUDIO_FILE" ]; then
    echo "❌ Error: Audio file not found: $AUDIO_FILE"
    exit 1
fi

# Count frames
FRAME_COUNT=$(find "$FRAMES_DIR" -name "frame_*.png" | wc -l | tr -d ' ')
echo "📊 Found $FRAME_COUNT frames in $FRAMES_DIR"
echo "🎵 Audio: $AUDIO_FILE"

# Check if ffmpeg is available
if ! command -v ffmpeg &> /dev/null; then
    echo "❌ Error: ffmpeg not found. Install with: brew install ffmpeg"
    exit 1
fi

# Combine frames and audio into video
echo "🎬 Combining frames and audio into video..."
ffmpeg -y \
    -framerate 60 \
    -pattern_type glob \
    -i "$FRAMES_DIR/frame_*.png" \
    -i "$AUDIO_FILE" \
    -c:v libx264 \
    -pix_fmt yuv420p \
    -preset medium \
    -crf 23 \
    -c:a aac \
    -b:a 192k \
    -shortest \
    "$OUTPUT_DIR/$OUTPUT_FILE" \
    2>&1 | grep -E "frame=|time=|size=" | tail -10

if [ -f "$OUTPUT_DIR/$OUTPUT_FILE" ]; then
    FILE_SIZE=$(du -h "$OUTPUT_DIR/$OUTPUT_FILE" | cut -f1)
    echo ""
    echo "✅ Video created: $OUTPUT_DIR/$OUTPUT_FILE ($FILE_SIZE)"
    echo ""
    echo "📺 To view:"
    echo "   open $OUTPUT_DIR/$OUTPUT_FILE"
else
    echo "❌ Error: Failed to create video"
    exit 1
fi

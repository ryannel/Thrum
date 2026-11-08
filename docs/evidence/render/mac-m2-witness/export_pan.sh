#!/bin/sh
# Read a completed Game Performance trace without starting any recording.
set -eu
if [ "$#" -ne 2 ]; then
    echo 'usage: export_pan.sh <pan.trace> <fresh output directory>' >&2
    exit 2
fi
trace=$1
dest=$2
mkdir "$dest"
xcrun xctrace export --input "$trace" --toc > "$dest/toc.xml"
for schema in displayed-surfaces-interval device-display-info time-info; do
    query="/trace-toc/run/data/table[@schema=\"$schema\"]"
    xcrun xctrace export --input "$trace" --xpath "$query" > "$dest/$schema.xml"
done
query='/trace-toc/run/data/table[@schema="os-signpost" and @category="PointsOfInterest" and not(@dynamic-tracing-enabled-subsystems)]'
if ! xcrun xctrace export --input "$trace" --xpath "$query" > "$dest/pan-markers.xml" 2> "$dest/pan-markers-error.log"; then
    echo 'Points of Interest export unavailable. Check the clock fallback with the game log.' >&2
fi

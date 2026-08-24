#!/bin/zsh
# iterate APNs configuration example.
# Usage:
#   1. Copy this file to a private location, e.g. ~/.config/iterate/apns-env.sh
#   2. Fill in your real values.
#   3. Run: source ~/.config/iterate/apns-env.sh
#   4. Start iterate from the same shell session.

export APNS_KEY_ID="YOUR_KEY_ID"
export APNS_TEAM_ID="YOUR_TEAM_ID"
export APNS_AUTH_KEY_PATH="/absolute/path/to/AuthKey_XXXXXXXXXX.p8"

# Optional overrides.
# APNS_TOPIC is the app bundle id. Live Activity pushes derive
# "<bundle id>.push-type.liveactivity" automatically.
export APNS_TOPIC="com.iterate.notify"
# Use "sandbox" for debug/dev builds, "production" for TestFlight/App Store or ad-hoc prod certs.
export APNS_ENV="sandbox"

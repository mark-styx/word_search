#!/bin/zsh

set -euo pipefail

output_file=""
prompt=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output-last-message)
      output_file="$2"
      shift 2
      ;;
    *)
      prompt="$1"
      shift
      ;;
  esac
done

if [[ -z "$output_file" ]]; then
  echo "missing --output-last-message" >&2
  exit 1
fi

if [[ "$prompt" == *"Adversarially review proposed additions"* ]]; then
  cat > "$output_file" <<'JSON'
{"approved":true,"can_extend":true,"reason":"The additions stay on-theme and family friendly.","words":["SEALIFE","KELPFOREST","TIDEPOOL","CORALREEF","WHALESONG","SEASTAR","DOLPHIN","DRIFTWOOD","ANCHOR","SHELLS"]}
JSON
  exit 0
fi

if [[ "$prompt" == *"You are extending an existing family-friendly word-search bank."* ]]; then
  cat > "$output_file" <<'JSON'
{"can_extend":true,"reason":"There is still room to grow this friendly ocean theme.","words":["SEALIFE","KELPFOREST","TIDEPOOL","CORALREEF","WHALESONG","SEASTAR","DOLPHIN","DRIFTWOOD","ANCHOR","SHELLS"]}
JSON
  exit 0
fi

cat > "$output_file" <<'JSON'
{"name":"Ocean Pals","can_extend":true,"words":["OCEAN","WAVES","TURTLE","DOLPHIN","SEAHORSE","STARFISH","CORAL","SHELL","BEACH","ISLAND","SAILBOAT","LIGHTHOUSE","ANCHOR","TIDAL","SEAGLASS","SANDCASTLE","BOARDWALK","LAGOON","PEBBLES","SURF","SNORKEL","KAYAK","HARBOR","MARINA","DRIFT","BREEZE","SUNSET","SANDY","COAST","PIER","JETTY","COVE","DUNE","GULL","WHALE","OTTER","PUFFIN","CLAM","KELP","REEF","TROPICS","SPRAY","PADDLE","TIDES","BOUY","CABANA","PALMS","SEABREEZE"]}
JSON

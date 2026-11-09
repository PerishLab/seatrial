case "$1" in workspace|lifecycle|flow|config|entry|history|input|mouse|display|interaction|exit) mode=$1 ;; *) exit 97 ;; esac
docker run --rm \
  --name "${SEATRIAL_LANE}-bagua-${mode}" \
  --network "$SEATRIAL_LANE" \
  --label perish.owner=seatrial \
  --label "perish.lane=$SEATRIAL_LANE" \
  --mount "type=volume,src=$SEATRIAL_VOLUME,dst=/trial" \
  --mount "type=bind,src=$SEATRIAL_BINARY_BAGUA,dst=/opt/bagua,readonly" \
  --mount "type=bind,src=$SEATRIAL_SCENE,dst=/scene,readonly" \
  --entrypoint python3 "$SEATRIAL_IMAGE" -B /scene/driver/main.py "$mode" /trial /opt/bagua

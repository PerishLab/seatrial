docker run --rm \
  --name "${SEATRIAL_LANE}-bagua-observation" \
  --network "$SEATRIAL_LANE" \
  --add-host "locus.${SEATRIAL_LANE}.internal:127.0.0.1" \
  --label perish.owner=seatrial \
  --label "perish.lane=$SEATRIAL_LANE" \
  --mount "type=volume,src=$SEATRIAL_VOLUME,dst=/trial" \
  --mount "type=bind,src=$SEATRIAL_BINARY_BAGUA,dst=/opt/bagua,readonly" \
  --mount "type=bind,src=$SEATRIAL_BINARY_LOCUS,dst=/opt/locus-api,readonly" \
  --mount "type=bind,src=$SEATRIAL_SCENE,dst=/scene,readonly" \
  --entrypoint python3 "$SEATRIAL_IMAGE" -B /scene/observation/main.py \
  /trial /opt/bagua /opt/locus-api "locus.${SEATRIAL_LANE}.internal" "$SEATRIAL_VERSION_BAGUA" "$SEATRIAL_VERSION_LOCUS"

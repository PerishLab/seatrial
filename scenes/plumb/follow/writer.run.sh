docker run --rm \
  --name "${SEATRIAL_LANE}-writer" \
  --network "$SEATRIAL_LANE" \
  --label perish.owner=seatrial \
  --label "perish.lane=$SEATRIAL_LANE" \
  --mount "type=volume,src=$SEATRIAL_VOLUME,dst=/trial" \
  --mount "type=bind,src=$SEATRIAL_BINARY_PLUMB,dst=/opt/plumb,readonly" \
  --mount "type=bind,src=$SEATRIAL_SCENE,dst=/scene,readonly" \
  --env HOME=/trial/home --env PLUMB_HOME=/trial/state \
  --entrypoint sh "$SEATRIAL_IMAGE" -eu /scene/writer.sh

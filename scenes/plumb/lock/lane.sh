case "$1" in joint|required) ;; *) exit 2 ;; esac
docker run --rm \
  --name "${SEATRIAL_LANE}-lock-$1" \
  --network "$SEATRIAL_LANE" \
  --label perish.owner=seatrial \
  --label "perish.lane=$SEATRIAL_LANE" \
  --mount "type=volume,src=$SEATRIAL_VOLUME,dst=/trial" \
  --mount "type=bind,src=$SEATRIAL_BINARY_PLUMB,dst=/opt/tools/plumb,readonly" \
  --mount "type=bind,src=$SEATRIAL_BINARY_ECTROPY,dst=/opt/tools/ectropy,readonly" \
  --mount "type=bind,src=$SEATRIAL_SCENE,dst=/scene,readonly" \
  --env HOME=/trial/home --env PLUMB_HOME=/trial/state \
  --env PLUMB_GUARD_CACHE_RESERVE=0 \
  --entrypoint sh "$SEATRIAL_IMAGE" -eu /scene/container.sh \
  "$1" "$SEATRIAL_VERSION_PLUMB" "$SEATRIAL_VERSION_ECTROPY"

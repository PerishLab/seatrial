case "$1" in context|release|project) mode=$1 ;; *) exit 97 ;; esac
docker run --rm \
  --name "${SEATRIAL_LANE}-concord-${mode}" \
  --network "$SEATRIAL_LANE" \
  --label perish.owner=seatrial \
  --label "perish.lane=$SEATRIAL_LANE" \
  --mount "type=volume,src=$SEATRIAL_VOLUME,dst=/trial" \
  --mount "type=bind,src=$SEATRIAL_BINARY_CONCORD,dst=/opt/concord,readonly" \
  --mount "type=bind,src=$SEATRIAL_SCENE,dst=/scene,readonly" \
  --env HOME=/trial/home \
  --entrypoint python3 "$SEATRIAL_IMAGE" -B "/scene/${mode}.py"

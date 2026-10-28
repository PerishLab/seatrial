test -n "$SEATRIAL_TENCENT_STATE"
test -f "$SEATRIAL_TENCENT_STATE"
case "$SEATRIAL_TENCENT_STATE" in
  /*) ;;
  *) exit 2 ;;
esac
test -n "$SEATRIAL_TENCENT_ACCOUNT"
test -n "$SEATRIAL_TENCENT_PRINCIPAL"
case "$SEATRIAL_TENCENT_ACCOUNT:$SEATRIAL_TENCENT_PRINCIPAL" in
  *[!0-9:]*|:*|*:) exit 2 ;;
esac
docker run --rm \
  --name "$SEATRIAL_LANE-observer" \
  --network "$SEATRIAL_LANE" \
  --label perish.owner=seatrial \
  --label "perish.lane=$SEATRIAL_LANE" \
  --mount "type=volume,src=$SEATRIAL_VOLUME,dst=/trial" \
  --mount "type=bind,src=$SEATRIAL_TENCENT_STATE,dst=/seed/observer.json,readonly" \
  --mount "type=bind,src=$SEATRIAL_BINARY_HARDRIG,dst=/opt/hardrig,readonly" \
  --mount "type=bind,src=$SEATRIAL_BINARY_RUNSEAL,dst=/opt/runseal,readonly" \
  --mount "type=bind,src=$SEATRIAL_SCENE,dst=/scene,readonly" \
  --env "SEATRIAL_TENCENT_ACCOUNT=$SEATRIAL_TENCENT_ACCOUNT" \
  --env "SEATRIAL_TENCENT_PRINCIPAL=$SEATRIAL_TENCENT_PRINCIPAL" \
  --env HOME=/trial/home \
  --entrypoint sh "$SEATRIAL_IMAGE" -eu /scene/tencent.sh

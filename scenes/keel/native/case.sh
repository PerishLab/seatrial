keel() {
  entry=$1
  shift
  docker run --rm \
    --network "$SEATRIAL_LANE" \
    --label perish.owner=seatrial --label "perish.lane=$SEATRIAL_LANE" \
    --mount "type=volume,src=$SEATRIAL_VOLUME,dst=/keel/.local" \
    --mount "type=bind,src=$SEATRIAL_BINARY_KEEL,dst=/opt/keel,readonly" \
    --mount "type=bind,src=$SEATRIAL_SCENE/model.json,dst=/opt/model.json,readonly" \
    --env KEEL_STORE=sqlite:/keel/.local/keel.db \
    --entrypoint "$entry" "$SEATRIAL_IMAGE" "$@"
}
cli() {
  keel /opt/keel "$@"
}
bootstrap() {
  cli bootstrap --model /opt/model.json --custody "/keel/.local/$1"
}
probe() {
  docker run --rm \
    --network "$SEATRIAL_LANE" \
    --label perish.owner=seatrial --label "perish.lane=$SEATRIAL_LANE" \
    "$SEATRIAL_IMAGE_PROBE" --silent --show-error --max-time 5 \
    --output /dev/null --write-out '%{http_code}' "$url"
}
test "$(keel keel-api --version)" = "keel-api $SEATRIAL_VERSION_KEEL"
bootstrap custody
case "$1" in
  again)
    keel test -f /keel/.local/custody
    if bootstrap second; then
      printf '%s\n' 'second bootstrap was accepted' >&2
      exit 1
    fi
    if keel test -f /keel/.local/second; then
      printf '%s\n' 'second bootstrap wrote a custody file' >&2
      exit 1
    fi
    test "$(cli status)" = sealed
    ;;
  health)
    host="keel-health-lite.$SEATRIAL_LANE.internal"
    docker run --detach --name "$host" \
      --network "$SEATRIAL_LANE" \
      --label perish.owner=seatrial --label "perish.lane=$SEATRIAL_LANE" \
      --mount "type=volume,src=$SEATRIAL_VOLUME,dst=/keel/.local" \
      --env KEEL_STORE=sqlite:/keel/.local/keel.db \
      --entrypoint keel-api "$SEATRIAL_IMAGE"
    url="http://$host:8080/health"
    deadline=$(($(date +%s) + 60))
    until test "$(probe || true)" = 200; do
      test "$(date +%s)" -lt "$deadline"
      sleep 0.25
    done
    keel cat /keel/.local/custody > /dev/null
    test "$(probe)" = 200
    ;;
  *) exit 2 ;;
esac

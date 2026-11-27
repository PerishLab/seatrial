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
    custody=$(keel sha256sum /keel/.local/custody)
    retry=$(bootstrap second)
    if test "$retry" != 'already sealed'; then
      printf '%s\n' 'same-model bootstrap did not report already sealed' >&2
      exit 1
    fi
    test "$(keel sha256sum /keel/.local/custody)" = "$custody"
    keel test ! -e /keel/.local/second
    keel test ! -L /keel/.local/second
    if refusal=$(keel sh -eu -c '
      model=$(sed "$1" /opt/model.json)
      test "$model" != "$(cat /opt/model.json)"
      printf "%s\n" "$model" > /keel/.local/changed.json
      exec /opt/keel bootstrap --model /keel/.local/changed.json --custody /keel/.local/custody
    ' sh 's/\("name":"body","kind":"text","only":{"kind":"free"},"serial":null,"need":\)false/\1true/' 2>&1); then
      printf '%s\n' 'different-model bootstrap was accepted' >&2
      exit 1
    fi
    case "$refusal" in
      *'occupied estate refuses bootstrap'*) ;;
      *) printf '%s\n' "$refusal" >&2; exit 1 ;;
    esac
    test "$(keel sha256sum /keel/.local/custody)" = "$custody"
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

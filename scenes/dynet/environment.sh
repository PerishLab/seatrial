set -eu
umask 077
trial_ns=$SEATRIAL_LANE
trial_root=$SEATRIAL_WORK/dynet-control
mkdir -m 700 "$trial_root"
trial_image=$(docker image inspect --format '{{.Id}}' "$SEATRIAL_IMAGE")
trial_secrets=$SEATRIAL_BINARY_SECRETS
trial_api=$(printf '%s' "$SEATRIAL_BINARIES_SECRETS" | jq -er '.["secrets-api"]')
trial_dynet=$SEATRIAL_BINARY_DYNET
trial_keeper=api.$trial_ns.internal
trial_server=$trial_ns-secrets
trial_publisher=$trial_ns-publisher
trial_client=$trial_ns-dynet
trial_resolver=$trial_ns-resolver
trial_agent_state=/device
trial_agent_bootstrap=/device/bootstrap.json
trial_marker=6f1c2a3b-0000-4000-8000-000000000001
fail() { printf '%s\n' "$1" >&2; exit 1; }
volume() {
  docker volume create --label perish.owner=seatrial --label "perish.lane=$trial_ns" "$trial_ns-$1" >/dev/null || return
  printf '%s\n' "$trial_ns-$1"
}
trial_state=$(volume secrets)
trial_auth=$(volume auth)
trial_public=$(volume public)
trial_publish=$(volume publisher)
trial_device=$(volume device)
owned() { docker run --label perish.owner=seatrial --label "perish.lane=$trial_ns" "$@"; }
put() { docker exec -i "$1" sh -eu -c 'umask 077; cat > "$1"' sh "$2"; }
pexec() { docker exec "$trial_publisher" /fixture/secrets "$@"; }
cexec() { docker exec "$trial_client" "$@"; }
connection=/publish/connection.json
declaration=/publish/publication.json
status() { pexec status --connection "$connection" --device device0; }
wait_status() {
  trial_deadline=$(( $(date +%s) + 40 ))
  while :; do
    trial_observed=$(status 2>/dev/null) || trial_observed=null
    if printf '%s' "$trial_observed" | jq -e --arg phase "$2" --argjson generation "$1" '.report.phase == $phase and .report.applied.generation == $generation' >/dev/null; then return; fi
    [ "$(date +%s)" -lt "$trial_deadline" ] || fail "device report did not reach $2 generation $1"
    sleep 1
  done
}
start_api() { docker start "$trial_server" >/dev/null; }
start_agent() {
  cexec sh -eu -c 'umask 077; : > /device/runtime.log'
  docker exec -d "$trial_client" sh -eu -c 'echo $$ > /device/runtime.pid; exec /fixture/dynet --instance dynet9 --state "$1" --port 15353 divert --bootstrap "$2" --seconds 0 >> /device/runtime.log 2>&1' sh "$trial_agent_state" "$trial_agent_bootstrap"
}
stop_agent() {
  trial_pid=$(cexec cat /device/runtime.pid)
  cexec kill -TERM "$trial_pid"
  trial_deadline=$(( $(date +%s) + 8 ))
  while cexec kill -0 "$trial_pid" 2>/dev/null; do
    [ "$(date +%s)" -lt "$trial_deadline" ] || fail 'Dynet stop exceeded its control transport budget'
    sleep 1
  done
}
dns() { printf 'DNS expectation: %s\n' "$1"; cexec /fixture/dns probe "$1"; }
diagnose() {
  trial_result=$?
  trap - EXIT
  if [ "$trial_result" -ne 0 ]; then
    cexec tail -n 60 /device/runtime.log || true
    cexec ip route show table dynet9 || true
    cexec stat -c '%a %u %g %n' /device /device/device.token /device/bootstrap.json || true
    docker logs --tail 20 "$trial_resolver" || true
  fi
  exit "$trial_result"
}
trap diagnose EXIT
wait_dns() {
  trial_deadline=$(( $(date +%s) + 6 ))
  while ! dns open; do [ "$(date +%s)" -lt "$trial_deadline" ] || fail 'confirmed offline DNS did not recover'; done
}
payload() {
  jq -n --arg prefix "$1" --arg marker "$trial_marker" '{version:1,upstream:"127.0.0.2",retain:86400,manifest:{version:1,nodes:[{label:"fixture/front",protocol:"vmess",server:"192.0.2.1",port:443,uuid:$marker,udp:true}],clusters:[{name:"private",nodes:["fixture/front"]}],rules:[{holds:$prefix,cluster:"private"}],fallback:"direct"}}' | put "$trial_publisher" /publish/payload.json
}
publish() { payload "$2"; pexec publish --declaration "$declaration" --generation "$1" --input /publish/payload.json > "$trial_root/published-$1.json"; }
checkpoint() { cexec cat "$trial_agent_state/dynet9.control/checkpoint"; }
wait_checkpoint() {
  trial_deadline=$(( $(date +%s) + 8 ))
  while ! checkpoint | jq -e --argjson received "$1" --argjson applied "$2" '.received.receipt.generation == $received and .applied.receipt.generation == $applied and .validated.generation == $applied' >/dev/null; do
    [ "$(date +%s)" -lt "$trial_deadline" ] || fail 'file checkpoint did not confirm expected generations'
    sleep 1
  done
}

set -eu
umask 077
gw_ns=$SEATRIAL_LANE
gw_root=$SEATRIAL_WORK/dynet-gateway
mkdir -m 700 "$gw_root"
gw_image=$(docker image inspect --format '{{.Id}}' "$SEATRIAL_IMAGE")
gw_router=gateway.$gw_ns.internal
gw_direct=direct.$gw_ns.internal
gw_client=$gw_ns-gateway-selected
gw_control=$gw_ns-gateway-control
gw_publisher=$gw_ns-gateway-publisher
gw_binary=$SEATRIAL_BINARY_DYNET
gw_secrets=$SEATRIAL_BINARY_SECRETS
gw_fixture=$SEATRIAL_WORK/dynet-dns
fail() { printf '%s\n' "$1" >&2; exit 1; }
owned() { docker run --env PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin --label perish.owner=seatrial --label "perish.lane=$gw_ns" "$@"; }
volume() {
  docker volume create --label perish.owner=seatrial --label "perish.lane=$gw_ns" "$gw_ns-gw-$1" >/dev/null
  printf '%s\n' "$gw_ns-gw-$1"
}
put() { docker exec -i "$1" sh -eu -c 'umask 077; cat > "$1"' sh "$2"; }
gexec() { docker exec "$gw_router" "$@"; }
pexec() { docker exec "$gw_publisher" /fixture/secrets "$@"; }
address() { docker inspect --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' "$1"; }
gw_device=$(volume device)
gw_publish=$(volume publish)
owned --detach --init --name "$gw_router" --network "$gw_ns" --network-alias "$gw_router" --cap-add NET_ADMIN --device /dev/net/tun --sysctl net.ipv4.ip_forward=1 --volume "$gw_device:/device" --volume "$gw_binary:/fixture/dynet:ro" --volume "$gw_fixture:/fixture/wire:ro" --entrypoint sleep "$gw_image" infinity >/dev/null
for gw_name in "$gw_direct" "$gw_client" "$gw_control"; do
  owned --detach --init --name "$gw_name" --network "$gw_ns" --cap-add NET_ADMIN --volume "$gw_fixture:/fixture/wire:ro" --entrypoint sleep "$gw_image" infinity >/dev/null
done
owned --detach --init --name "$gw_publisher" --network none --volume "$gw_publish:/publish" --volume "$gw_secrets:/fixture/secrets:ro" --entrypoint sleep "$gw_image" infinity >/dev/null
gw_jobs=
for gw_name in "$gw_router" "$gw_direct" "$gw_client" "$gw_control"; do
  docker exec "$gw_name" sh -eu -c 'apt-get -o Acquire::Retries=0 -o Acquire::http::Timeout=20 update >/dev/null; DEBIAN_FRONTEND=noninteractive apt-get -y --no-install-recommends install iproute2=6.15.0-1 >/dev/null' > "$gw_root/$gw_name.packages" 2>&1 &
  gw_jobs="$gw_jobs $!"
done
gw_failed=0
for gw_job in $gw_jobs; do wait "$gw_job" || gw_failed=1; done
[ "$gw_failed" -eq 0 ] || fail 'gateway fixture iproute2 preparation failed'
gexec sh -eu -c 'DEBIAN_FRONTEND=noninteractive apt-get -y --no-install-recommends install nftables=1.1.3-1 >/dev/null; mkdir -p /etc/iproute2/rt_tables.d; chmod 700 /device'
docker exec "$gw_publisher" chmod 700 /publish
pexec signer --output /publish/signer.key > "$gw_root/publisher.pub"
printf '%064d\n' 0 | put "$gw_publisher" /publish/unused.token
gexec /fixture/dynet identity --output /device/identity.age > "$gw_root/recipient.pub"
gw_recipient=$(cat "$gw_root/recipient.pub")
gw_key=$(cat "$gw_root/publisher.pub")
jq -n --arg recipient "$gw_recipient" '{target:{device:"gateway0",resource:"routing",epoch:"fixture1"},recipient:$recipient,signerRef:"/publish/signer.key",minimum:1,remote:{baseUrl:"http://127.0.0.1:8080",tokenRef:"/publish/unused.token"}}' | put "$gw_publisher" /publish/declaration.json
jq -n '{version:1,upstream:"127.0.0.2",retain:86400,manifest:{version:1,nodes:[{label:"fixture/closed",protocol:"vmess",server:"127.0.0.3",port:2444,uuid:"6f1c2a3b-0000-4000-8000-000000000001",udp:true},{label:"fixture/silent",protocol:"vmess",server:"127.0.0.3",port:2443,uuid:"6f1c2a3b-0000-4000-8000-000000000002",udp:true}],clusters:[{name:"private",nodes:["fixture/closed","fixture/silent"]}],rules:[{suffix:"protected.fixture",cluster:"private"}],fallback:"direct"}}' | put "$gw_publisher" /publish/payload.json
pexec seal --declaration /publish/declaration.json --generation 1 --input /publish/payload.json --output /publish/routing.envelope
docker exec "$gw_publisher" cat /publish/routing.envelope | put "$gw_router" /device/routing.envelope
jq -n --arg publisher "$gw_key" '{version:1,target:{device:"gateway0",resource:"routing",epoch:"fixture1"},identityRef:"/device/identity.age",publisher:$publisher,minimum:1,source:{kind:"file",path:"/device/routing.envelope"}}' | put "$gw_router" /device/bootstrap.json
gw_router_ip=$(address "$gw_router")
gw_client_ip=$(address "$gw_client")
gw_direct_ip=$(address "$gw_direct")
[ -n "$gw_router_ip" ] && [ -n "$gw_client_ip" ] && [ -n "$gw_direct_ip" ] || fail 'gateway fixture addresses unavailable'
docker exec "$gw_direct" ip address add 203.0.113.7/32 dev lo
gexec ip route add 203.0.113.7/32 via "$gw_direct_ip"
for gw_name in "$gw_client" "$gw_control"; do docker exec "$gw_name" ip route add 203.0.113.7/32 via "$gw_router_ip"; done
docker exec -d "$gw_direct" /fixture/wire http 203.0.113.7:8080
docker exec -d "$gw_router" /fixture/wire dns 127.0.0.2:53 203.0.113.7
docker exec -d "$gw_router" /fixture/wire dns "$gw_router_ip:53" 203.0.113.7
docker exec -d "$gw_router" sh -eu -c 'exec /fixture/wire blackhole 127.0.0.3:2443 > /device/silent.log 2>&1'
diagnose() {
  gw_result=$?
  trap - EXIT
  if [ "$gw_result" -ne 0 ]; then
    gexec tail -n 50 /device/runtime.log || true
    gexec ip -4 rule show || true
    gexec nft list ruleset || true
    cat "$gw_root"/*.packages || true
  fi
  exit "$gw_result"
}
trap diagnose EXIT
gexec /fixture/wire query 127.0.0.2:53 before.fixture 203.0.113.7
docker exec "$gw_control" /fixture/wire query "$gw_router_ip:53" protected.fixture 203.0.113.7
docker exec "$gw_client" /fixture/wire fetch 203.0.113.7:8080
gexec nft add table inet fw4
gexec env DYNET_LOCAL=false DYNET_SOURCES="$gw_client_ip/32" /fixture/dynet --instance dynet9 --port 15353 up --claim 0.0.0.0/0
docker exec -d "$gw_router" sh -eu -c 'umask 077; ulimit -c 0; ulimit -n 512; ulimit -v 524288; echo $$ > /device/runtime.pid; exec env DYNET_LOCAL=false DYNET_SOURCES="$1/32" /fixture/dynet --instance dynet9 --state /device --port 15353 divert --bootstrap /device/bootstrap.json --seconds 0 > /device/runtime.log 2>&1' sh "$gw_client_ip"
gw_deadline=$(( $(date +%s) + 10 ))
until docker exec "$gw_client" /fixture/wire query "$gw_router_ip:53" ready.fixture 203.0.113.7; do
  [ "$(date +%s)" -lt "$gw_deadline" ] || fail 'forwarded DNS did not become ready'
  sleep 1
done
docker exec "$gw_client" /fixture/wire pressure "$gw_router_ip:53" > "$gw_root/pressure.log" 2>&1 &
gw_pressure=$!
sleep 1
for gw_round in 1 2 3 4 5; do
  docker exec "$gw_client" /fixture/wire query "$gw_router_ip:53" "neutral-$gw_round.fixture" 203.0.113.7
  docker exec "$gw_client" /fixture/wire fetch 203.0.113.7:8080
  docker exec "$gw_control" /fixture/wire query "$gw_router_ip:53" "other-$gw_round.protected.fixture" 203.0.113.7
  docker exec "$gw_control" /fixture/wire fetch 203.0.113.7:8080
  gexec /fixture/wire query 127.0.0.2:53 "router-$gw_round.protected.fixture" 203.0.113.7
done
wait "$gw_pressure" || { cat "$gw_root/pressure.log"; fail 'a protected query escaped or produced an unknown answer'; }
gexec grep -Fx 'accepted silent-node connection' /device/silent.log
gexec kill -0 "$(gexec cat /device/runtime.pid)"
docker exec "$gw_client" /fixture/wire query "$gw_router_ip:53" after.protected.fixture closed
gw_pid=$(gexec cat /device/runtime.pid)
gexec kill -TERM "$gw_pid"
gw_deadline=$(( $(date +%s) + 12 ))
while gexec kill -0 "$gw_pid" 2>/dev/null; do
  [ "$(date +%s)" -lt "$gw_deadline" ] || fail 'gateway worker did not stop within its test budget'
  sleep 1
done
gexec /fixture/dynet --instance dynet9 down
gexec nft list tables | grep -Fx 'table inet fw4'
printf '%s\n' 'Forwarded selected-client direct DNS/HTTP, unrelated-client and router controls survived closed/silent node pressure; protected replies stayed closed. Native OpenWrt/procd and offload remain unqualified.'

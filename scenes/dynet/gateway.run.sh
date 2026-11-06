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
address() { docker inspect --format '{{json .NetworkSettings.Networks}}' "$1" | jq -er --arg network "$2" '.[$network].IPAddress'; }
retire() {
  for gw_direction in -s -d; do
    gw_ct=$gw_root/retired-$1-$gw_direction.ct
    if gexec conntrack -D "$gw_direction" "$gw_client_ip" > "$gw_ct" 2>&1; then :; else
      gw_result=$?
      [ "$gw_result" -eq 1 ] && [ "$(wc -l < "$gw_ct")" -eq 1 ] && grep -Eq '^conntrack v1\.4\.8 \(conntrack-tools\): 0 flow entries have been deleted\.$' "$gw_ct" || { cat "$gw_ct"; fail 'selected conntrack retirement unknown'; }
    fi
    gexec conntrack -L "$gw_direction" "$gw_client_ip" > "$gw_root/remaining-$1-$gw_direction.ct" 2> "$gw_root/remaining-$1-$gw_direction.stderr"
    if grep -q 'src=' "$gw_root/remaining-$1-$gw_direction.ct"; then fail 'selected conntrack entries remain'; fi
  done
}

gw_device=$(volume device)
gw_publish=$(volume publish)
gw_wan=$gw_ns-gateway-wan
docker network create --label perish.owner=seatrial --label "perish.lane=$gw_ns" "$gw_wan" >/dev/null
owned --detach --init --name "$gw_router" --network "$gw_ns" --network-alias "$gw_router" --cap-add NET_ADMIN --device /dev/net/tun --sysctl net.ipv4.ip_forward=1 --volume "$gw_device:/device" --volume "$gw_binary:/fixture/dynet:ro" --volume "$gw_fixture:/fixture/wire:ro" --entrypoint sleep "$gw_image" infinity >/dev/null
docker network connect "$gw_wan" "$gw_router"
owned --detach --init --name "$gw_direct" --network "$gw_wan" --cap-add NET_ADMIN --volume "$gw_fixture:/fixture/wire:ro" --entrypoint sleep "$gw_image" infinity >/dev/null
for gw_name in "$gw_client" "$gw_control"; do
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
gexec sh -eu -c 'DEBIAN_FRONTEND=noninteractive apt-get -y --no-install-recommends install nftables=1.1.3-1 conntrack=1:1.4.8-2 >/dev/null; mkdir -p /etc/iproute2/rt_tables.d; chmod 700 /device'
docker exec "$gw_publisher" chmod 700 /publish
pexec signer --output /publish/signer.key > "$gw_root/publisher.pub"
printf '%064d\n' 0 | put "$gw_publisher" /publish/unused.token
gexec /fixture/dynet identity --output /device/identity.age > "$gw_root/recipient.pub"
gw_recipient=$(cat "$gw_root/recipient.pub")
gw_key=$(cat "$gw_root/publisher.pub")
jq -n --arg recipient "$gw_recipient" '{target:{device:"gateway0",resource:"routing",epoch:"fixture1"},recipient:$recipient,signerRef:"/publish/signer.key",minimum:1,remote:{baseUrl:"http://127.0.0.1:8080",tokenRef:"/publish/unused.token"}}' | put "$gw_publisher" /publish/declaration.json
jq -n '{version:1,upstream:"127.0.0.2",retain:86400,manifest:{version:1,nodes:[{label:"fixture/closed",protocol:"vmess",server:"127.0.0.3",port:2444,uuid:"6f1c2a3b-0000-4000-8000-000000000001",udp:true},{label:"fixture/silent",protocol:"vmess",server:"127.0.0.3",port:2443,uuid:"6f1c2a3b-0000-4000-8000-000000000002",udp:true}],clusters:[{name:"private",nodes:["fixture/closed","fixture/silent"]}],rules:[{suffix:"protected.fixture",cluster:"private"},{holds:"203.0.113.8/32",cluster:"private"}],fallback:"direct"}}' | put "$gw_publisher" /publish/payload.json
pexec seal --declaration /publish/declaration.json --generation 1 --input /publish/payload.json --output /publish/routing.envelope
docker exec "$gw_publisher" cat /publish/routing.envelope | put "$gw_router" /device/routing.envelope
jq -n --arg publisher "$gw_key" '{version:1,target:{device:"gateway0",resource:"routing",epoch:"fixture1"},identityRef:"/device/identity.age",publisher:$publisher,minimum:1,source:{kind:"file",path:"/device/routing.envelope"}}' | put "$gw_router" /device/bootstrap.json
gw_router_ip=$(address "$gw_router" "$gw_ns")
gw_router_wan=$(address "$gw_router" "$gw_wan")
gw_client_ip=$(address "$gw_client" "$gw_ns")
gw_control_ip=$(address "$gw_control" "$gw_ns")
gw_direct_ip=$(address "$gw_direct" "$gw_wan")
gw_lan=$(docker network inspect "$gw_ns" | jq -er '.[0].IPAM.Config[0].Subnet')
[ -n "$gw_router_ip" ] && [ -n "$gw_client_ip" ] && [ -n "$gw_direct_ip" ] || fail 'gateway fixture addresses unavailable'
docker exec "$gw_direct" ip address add 203.0.113.7/32 dev lo
docker exec "$gw_direct" ip address add 203.0.113.8/32 dev lo
gexec ip route add 203.0.113.7/32 via "$gw_direct_ip"
gexec ip route add 203.0.113.8/32 via "$gw_direct_ip"
docker exec "$gw_direct" ip route add "$gw_lan" via "$gw_router_wan"
for gw_name in "$gw_client" "$gw_control"; do
  for gw_target in 203.0.113.7 203.0.113.8; do docker exec "$gw_name" ip route add "$gw_target/32" via "$gw_router_ip"; done
done
docker exec -d "$gw_direct" /fixture/wire http 0.0.0.0:8080
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
put "$gw_router" /device/fw4.nft <<'NFT'
table inet fw4 {
  flowtable ft { hook ingress priority 0; devices = { "eth0", "eth1" }; counter; }
  chain input {
    type filter hook input priority 0; policy drop;
    iifname "lo" counter accept
    iifname "eth0" counter accept
    ct state established,related counter accept
    iifname "dynet9" meta l4proto tcp counter drop comment "seatrial-dynet-input-denied"
  }
  chain forward {
    type filter hook forward priority 0; policy drop;
    meta l4proto { tcp, udp } flow add @ft
    ct state established,related counter accept
    iifname "eth0" oifname "eth1" counter accept
  }
}
NFT
gexec nft -f /device/fw4.nft
gexec nft -s list table inet fw4 > "$gw_root/fw4.before"
docker exec "$gw_client" /fixture/wire hold 203.0.113.7:8080 > "$gw_root/selected-held.log" 2>&1 &
gw_selected_held=$!
docker exec "$gw_control" /fixture/wire hold 203.0.113.7:8080 > "$gw_root/control-held.log" 2>&1 &
gw_control_held=$!
gw_deadline=$(( $(date +%s) + 8 ))
until gexec conntrack -L -p tcp -s "$gw_client_ip" --dst 203.0.113.7 --dport 8080 -o extended > "$gw_root/selected.ct" 2>/dev/null && grep -q '\[OFFLOAD\]' "$gw_root/selected.ct" && gexec conntrack -L -p tcp -s "$gw_control_ip" --dst 203.0.113.7 --dport 8080 -o extended > "$gw_root/control.ct" 2>/dev/null && grep -q '\[OFFLOAD\]' "$gw_root/control.ct"; do
  [ "$(date +%s)" -lt "$gw_deadline" ] || fail 'actual software-offload prerequisite is unknown'
  sleep 1
done
printf '%s\n' 'Both original client connections are proved OFFLOAD.'
put "$gw_router" /device/fence.nft <<NFT
table inet trial_fence {
  set held { type ipv4_addr; elements = { $gw_client_ip }; }
  chain lan { type filter hook ingress device "eth0" priority -10; policy accept; ip saddr @held counter drop; }
  chain wan { type filter hook ingress device "eth1" priority -10; policy accept; ip daddr @held counter drop; }
}
NFT
gexec nft -f /device/fence.nft
gw_deadline=$(( $(date +%s) + 5 ))
while kill -0 "$gw_selected_held" 2>/dev/null; do
  [ "$(date +%s)" -lt "$gw_deadline" ] || fail 'selected offloaded connection bypassed ingress fencing'
  sleep 1
done
if wait "$gw_selected_held"; then fail 'selected held connection completed through its fence'; fi
kill -0 "$gw_control_held"
gexec nft -j list table inet trial_fence > "$gw_root/fence.observed.json"
jq -e '[.nftables[].rule? | .expr[]? | .counter? | select(.packets > 0)] | length > 0' "$gw_root/fence.observed.json" >/dev/null
docker exec "$gw_control" /fixture/wire fetch 203.0.113.7:8080
retire admission
put "$gw_router" /device/admission.nft <<NFT
insert rule inet fw4 forward ip saddr $gw_client_ip oifname "dynet9" counter accept comment "seatrial-dynet-out"
insert rule inet fw4 forward iifname "dynet9" ip daddr $gw_client_ip counter accept comment "seatrial-dynet-back"
NFT
gexec nft -f /device/admission.nft
gexec env DYNET_SPAN=198.51.100.0/24 DYNET_LOCAL=false DYNET_SOURCES="$gw_client_ip/32" /fixture/dynet --instance dynet9 --port 15353 up --claim 203.0.113.0/24
docker exec -d "$gw_router" sh -eu -c 'umask 077; ulimit -c 0; ulimit -n 512; ulimit -v 524288; echo $$ > /device/runtime.pid; exec env DYNET_SPAN=198.51.100.0/24 DYNET_LOCAL=false DYNET_SOURCES="$1/32" /fixture/dynet --instance dynet9 --state /device --port 15353 divert --bootstrap /device/bootstrap.json --seconds 0 > /device/runtime.log 2>&1' sh "$gw_client_ip"
gw_seat=$(gexec ip -j -4 address show dev dynet9 | jq -er '.[0].addr_info[0].local')
gw_deadline=$(( $(date +%s) + 10 ))
until gexec /fixture/wire query "$gw_seat:15353" ready.fixture 203.0.113.7; do
  [ "$(date +%s)" -lt "$gw_deadline" ] || fail 'forwarded DNS did not become ready'
  sleep 1
done
docker exec -d "$gw_router" sh -eu -c 'echo $$ > /device/competing.pid; exec /fixture/wire http "$1:53"' sh "$gw_seat"
gw_hex=$(printf '%s\n' "$gw_seat" | awk -F. '{printf "%02X%02X%02X%02X", $4, $3, $2, $1}')
put "$gw_router" /device/listener.sh <<'SH'
set -eu
worker=$(cat /device/runtime.pid)
cat "/proc/$worker/net/tcp" > /device/worker.tcp
for fd in /proc/$worker/fd/*; do
  value=$(readlink "$fd" 2>/dev/null || true)
  case "$value" in socket:*) printf '%s %s\n' "$fd" "$value";; esac
done > /device/worker.sockets
awk -v address="$1" 'NR == FNR {gsub(/[^0-9]/,"",$2); owned[$2]=1; next} index($2,address ":") == 1 && $4 == "0A" && ($10 in owned) {print $2, $10}' /device/worker.sockets /device/worker.tcp > /device/listener.bound
test "$(wc -l < /device/listener.bound)" -eq 1
SH
gw_deadline=$(( $(date +%s) + 5 ))
until gexec sh -eu -c 'grep -q "$1:0035 .* 0A " /proc/net/tcp' sh "$gw_hex"; do
  [ "$(date +%s)" -lt "$gw_deadline" ] || fail 'competing TCP listener did not become ready'
  sleep 1
done
gexec sh /device/listener.sh "$gw_hex"
gexec cat /device/worker.tcp > "$gw_root/worker.tcp"
gexec cat /device/worker.sockets > "$gw_root/worker.sockets"
gexec cat /device/listener.bound > "$gw_root/listener.bound"
gw_competing=$(gexec cat /device/competing.pid)
gw_foreign=$(awk -v address="$gw_hex:0035" '$2 == address && $4 == "0A" {print $10}' "$gw_root/worker.tcp")
[ -n "$gw_foreign" ] && [ "$(printf '%s\n' "$gw_foreign" | wc -l)" -eq 1 ] || fail 'competing listener is ambiguous'
gexec sh -eu -c 'for fd in /proc/$1/fd/*; do readlink "$fd" || true; done' sh "$gw_competing" > "$gw_root/competing.sockets"
grep -Fxq "socket:[$gw_foreign]" "$gw_root/competing.sockets" || fail 'competing socket owner is unknown'
if grep -Fq "socket:[$gw_foreign]" "$gw_root/worker.sockets"; then fail 'foreign listener belongs to worker'; fi
gw_endpoint=$(awk '{print $1}' "$gw_root/listener.bound")
gw_port=$((0x${gw_endpoint##*:}))
[ "$gw_port" -gt 0 ] && [ "$gw_port" -ne 53 ] || fail 'worker listener selection chose competing port'
[ "$gw_seat" = 198.51.100.1 ] || fail 'unexpected fixture TUN seat'
gexec nft delete element inet trial_fence held "{ $gw_client_ip }"
docker exec "$gw_client" /fixture/wire query "$gw_router_ip:53" admitted.fixture 203.0.113.7
if docker exec "$gw_client" /fixture/wire fetch 203.0.113.7:8080 > "$gw_root/input-denied-fetch.log" 2>&1; then fail 'forwarding rules bypassed internal INPUT denial'; fi
gexec nft -j list chain inet fw4 input > "$gw_root/input.denied.json"
jq -e '[.nftables[].rule? | select(.comment == "seatrial-dynet-input-denied") | .expr[] | .counter? | select(.packets > 0)] | length == 1' "$gw_root/input.denied.json" >/dev/null
gexec kill -0 "$(gexec cat /device/runtime.pid)"
docker exec "$gw_control" /fixture/wire fetch 203.0.113.7:8080
gexec nft insert rule inet fw4 input iifname dynet9 ip saddr 198.51.100.128 ip daddr "$gw_seat" tcp dport "$gw_port" counter accept comment '"seatrial-dynet-in"'
docker exec "$gw_client" /fixture/wire fetch 203.0.113.7:8080
printf '%s\n' 'Forwarding alone failed; PID-owned TCP INPUT admission excluded the foreign TCP53 listener and restored HTTP.'
docker exec "$gw_client" /fixture/wire fixed "$gw_router_ip:53"
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
gexec grep -Fq 'accepted silent-node connection' /device/silent.log
gexec kill -0 "$(gexec cat /device/runtime.pid)"
docker exec "$gw_client" /fixture/wire query "$gw_router_ip:53" after.protected.fixture closed
docker exec "$gw_control" /fixture/wire fetch 203.0.113.8:8080
if docker exec "$gw_client" /fixture/wire fetch 203.0.113.8:8080 > "$gw_root/protected-fetch.log" 2>&1; then fail 'protected held address escaped to the direct backend'; fi
gexec nft -j list chain inet fw4 forward > "$gw_root/admission.observed.json"
for gw_tag in seatrial-dynet-out seatrial-dynet-back; do
  jq -e --arg tag "$gw_tag" '[.nftables[].rule? | select(.comment == $tag) | .expr[] | .counter? | select(.packets > 0)] | length == 1' "$gw_root/admission.observed.json" >/dev/null
done
gexec nft -j list chain inet fw4 input > "$gw_root/input.admitted.json"
jq -e '[.nftables[].rule? | select(.comment == "seatrial-dynet-in") | .expr[] | .counter? | select(.packets > 0)] | length == 1' "$gw_root/input.admitted.json" >/dev/null
gexec conntrack -L -s "$gw_client_ip" -d 203.0.113.8 -o extended > "$gw_root/protected.ct" 2> "$gw_root/protected.ct.stderr"
if grep -q '\[OFFLOAD\]' "$gw_root/protected.ct"; then fail 'selected capture path became offloaded'; fi
docker exec "$gw_client" /fixture/wire fixed "$gw_router_ip:53"
gexec nft add element inet trial_fence held "{ $gw_client_ip }"
docker exec "$gw_client" /fixture/wire query "$gw_router_ip:53" fenced.fixture closed
if docker exec "$gw_client" /fixture/wire fetch 203.0.113.7:8080 > "$gw_root/fenced-fetch.log" 2>&1; then fail 'selected HTTP escaped withdrawal fence'; fi
docker exec "$gw_control" /fixture/wire fetch 203.0.113.7:8080
kill -0 "$gw_control_held"
gw_pid=$(gexec cat /device/runtime.pid)
gexec kill -TERM "$gw_pid"
gw_deadline=$(( $(date +%s) + 12 ))
while gexec kill -0 "$gw_pid" 2>/dev/null; do
  [ "$(date +%s)" -lt "$gw_deadline" ] || fail 'gateway worker did not stop within its test budget'
  sleep 1
done
gexec /fixture/dynet --instance dynet9 down
for gw_tag in seatrial-dynet-out seatrial-dynet-back; do
  gw_handle=$(gexec nft -j list chain inet fw4 forward | jq -er --arg tag "$gw_tag" '[.nftables[].rule? | select(.comment == $tag)] | select(length == 1) | .[0].handle')
  gexec nft delete rule inet fw4 forward handle "$gw_handle"
done
gw_handle=$(gexec nft -j list chain inet fw4 input | jq -er '[.nftables[].rule? | select(.comment == "seatrial-dynet-in")] | select(length == 1) | .[0].handle')
gexec nft delete rule inet fw4 input handle "$gw_handle"
gexec nft -s list table inet fw4 > "$gw_root/fw4.after"
cmp "$gw_root/fw4.before" "$gw_root/fw4.after"
gexec conntrack -L -p udp -s "$gw_client_ip" --sport 40053 --dport 53 -o extended > "$gw_root/stale-dns.ct" 2> "$gw_root/stale-dns.stderr"
grep -q "src=$gw_seat " "$gw_root/stale-dns.ct" || fail 'retained DNS DNAT prerequisite missing'
retire withdrawal
gexec nft delete table inet trial_fence
docker exec "$gw_client" /fixture/wire fixed "$gw_router_ip:53"
docker exec "$gw_client" /fixture/wire query "$gw_router_ip:53" restored.fixture 203.0.113.7
docker exec "$gw_client" /fixture/wire fetch 203.0.113.7:8080
wait "$gw_control_held" || { cat "$gw_root/control-held.log"; fail 'unrelated held connection was interrupted'; }
printf '%s\n' 'Source-only fw4 admission, real OFFLOAD fencing/retirement and unrelated held connection preservation passed.'
printf '%s\n' 'Forwarded selected-client direct DNS/HTTP, unrelated-client and router controls survived closed/silent node pressure; protected replies stayed closed. Native OpenWrt/PPPoE and hardware offload remain unqualified.'

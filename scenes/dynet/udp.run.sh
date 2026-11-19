set -eu
umask 077

lane=$SEATRIAL_LANE
work=$SEATRIAL_WORK/dynet-udp
image=$(docker image inspect --format '{{.Id}}' "$SEATRIAL_IMAGE")
sing=ghcr.io/sagernet/sing-box:v1.13.14@sha256:c8b67944345dc84a226b648a71f854818606eae0813c4e6a452f192ef821b5b8
wire=$work/wire
router=$lane-udp-router
front=$lane-udp-front
exitnode=$lane-udp-exit
sink=$lane-udp-sink
first=$lane-udp-first
second=$lane-udp-second
wan=$lane-udp-wan
uuid=85054868-dbcb-48d2-b8db-8e898753f9ea
password=c2VhdHJpYWwtdWRwLXRlcw==
mkdir -m 700 "$work"
docker run --rm --network none --user "$(id -u):$(id -g)" \
  --label perish.owner=seatrial --label "perish.lane=$lane" \
  --volume "$SEATRIAL_SCENE:/scene:ro" --volume "$work:/work" \
  --entrypoint rustc "$image" --edition 2024 /scene/wire/main.rs -o /work/wire
sha256sum "$wire"

owned() {
  docker run --env PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin --detach --init --label perish.owner=seatrial --label "perish.lane=$lane" "$@"
}
put() { docker exec -i "$1" sh -eu -c 'umask 077; cat > "$1"' sh "$2"; }
address() { docker inspect --format '{{json .NetworkSettings.Networks}}' "$1" | jq -er --arg network "$2" '.[$network].IPAddress'; }
gateway() { docker exec "$router" "$@"; }
witness() { docker logs "$sink" 2>&1 | awk -v size="$1" 'NF == 3 && $3 == size { count++ } END { print count + 0 }'; }

docker network create --label perish.owner=seatrial --label "perish.lane=$lane" "$wan" >/dev/null
owned --name "$router" --network "$lane" --cap-add NET_ADMIN --device /dev/net/tun \
  --sysctl net.ipv4.ip_forward=1 --volume "$SEATRIAL_BINARY_DYNET:/fixture/dynet:ro" \
  --volume "$SEATRIAL_VOLUME:/fixture/debian" --volume "$wire:/fixture/wire:ro" --entrypoint sleep "$image" infinity >/dev/null
docker network connect "$wan" "$router"
owned --name "$sink" --network "$wan" --cap-add NET_ADMIN \
  --volume "$SEATRIAL_VOLUME:/fixture/debian" --volume "$wire:/fixture/wire:ro" --entrypoint sleep "$image" infinity >/dev/null
for endpoint in "$first" "$second"; do
  owned --name "$endpoint" --network "$lane" --cap-add NET_ADMIN \
    --volume "$SEATRIAL_VOLUME:/fixture/debian" --volume "$wire:/fixture/wire:ro" --entrypoint sleep "$image" infinity >/dev/null
done

lists=/fixture/debian/lists
gateway mkdir -p "$lists"
gateway apt-get -o "Dir::State::Lists=$lists" -o Acquire::Retries=0 -o Acquire::http::Timeout=20 update >/dev/null
jobs=
for endpoint in "$router" "$sink" "$first" "$second"; do
  docker exec "$endpoint" env DEBIAN_FRONTEND=noninteractive apt-get -o "Dir::State::Lists=$lists" \
    -y --no-install-recommends install iproute2=6.15.0-1 > "$work/$endpoint.packages" 2>&1 &
  jobs="$jobs $!"
done
failed=0
for job in $jobs; do wait "$job" || failed=1; done
if [ "$failed" -ne 0 ]; then cat "$work"/*.packages; exit 1; fi
gateway env DEBIAN_FRONTEND=noninteractive apt-get -o "Dir::State::Lists=$lists" -y --no-install-recommends install nftables=1.1.3-1 >/dev/null

routerlan=$(address "$router" "$lane")
routerwan=$(address "$router" "$wan")
sinkip=$(address "$sink" "$wan")
firstip=$(address "$first" "$lane")
secondip=$(address "$second" "$lane")
wanif=$(gateway ip -j route get "$sinkip" | jq -er '.[0].dev')
subnet=$(docker network inspect "$lane" | jq -er '.[0].IPAM.Config[0].Subnet')

jq -n --arg uuid "$uuid" '{inbounds:[{type:"vmess",listen:"0.0.0.0",listen_port:2443,users:[{uuid:$uuid,alterId:0}]}],outbounds:[{type:"direct"}]}' > "$work/front.json"
jq -n --arg key "$password" '{inbounds:[{type:"shadowsocks",listen:"0.0.0.0",listen_port:2444,method:"2022-blake3-aes-128-gcm",password:$key}],outbounds:[{type:"direct"}]}' > "$work/exit.json"
owned --name "$exitnode" --network "$wan" --cap-add NET_ADMIN --volume "$work/exit.json:/etc/sing-box/test.json:ro" "$sing" run -c /etc/sing-box/test.json >/dev/null
exitip=$(address "$exitnode" "$wan")
owned --name "$front" --network "$wan" --add-host "udp-exit:$exitip" --volume "$work/front.json:/etc/sing-box/test.json:ro" "$sing" run -c /etc/sing-box/test.json >/dev/null
frontip=$(address "$front" "$wan")



for target in 203.0.113.7 203.0.113.8; do
  docker exec "$sink" ip address add "$target/32" dev lo
  gateway ip route add "$target/32" via "$sinkip"
  docker exec "$exitnode" ip route add "$target/32" via "$sinkip"
  for endpoint in "$first" "$second"; do docker exec "$endpoint" ip route add "$target/32" via "$routerlan"; done
done
gateway nft add table ip udp_control
gateway nft 'add chain ip udp_control postrouting { type nat hook postrouting priority srcnat; policy accept; }'
gateway nft add rule ip udp_control postrouting oifname "$wanif" ip saddr "$subnet" ip daddr 203.0.113.0/24 snat to "$routerwan"
docker exec -d "$sink" sh -eu -c 'exec /fixture/wire echo 203.0.113.7:3478 >/proc/1/fd/1 2>&1'
docker exec -d "$sink" sh -eu -c 'exec /fixture/wire echo 203.0.113.8:3478 >/proc/1/fd/1 2>&1'
docker exec -d "$sink" sh -eu -c 'exec /fixture/wire echo 203.0.113.7:443 >/proc/1/fd/1 2>&1'
docker exec -d "$sink" /fixture/wire http 0.0.0.0:3478
docker exec "$first" /fixture/wire round 0.0.0.0:24999 203.0.113.7:3478 "$routerwan" 16

gateway sh -eu -c 'mkdir -m 700 /device; mkdir -p /etc/iproute2/rt_tables.d'
put "$router" /device/identity.age < "$SEATRIAL_SCENE/fixture/identity.age"
put "$router" /device/bootstrap.json < "$SEATRIAL_SCENE/fixture/bootstrap.json"
put "$router" /device/routing.envelope < "$SEATRIAL_SCENE/fixture/one.envelope"

sources=$firstip/32,$secondip/32
start() {
  docker exec -d "$router" sh -eu -c 'umask 077; ulimit -n 512; echo $$ > /device/runtime.pid;
    exec env DYNET_SPAN=198.51.100.0/24 DYNET_LOCAL=false DYNET_SOURCES="$1" /fixture/dynet --instance dynet9 --state /device --port 15353 divert --bootstrap /device/bootstrap.json --seconds 0 >>/device/runtime.log 2>&1' sh "$sources"
  deadline=$(( $(date +%s) + 20 ))
  until gateway /fixture/wire query 198.51.100.1:15353 ready.fixture 203.0.113.7; do
    [ "$(date +%s)" -lt "$deadline" ] || return 1
    sleep 1
  done
}
crash() {
  worker=$(gateway cat /device/runtime.pid)
  gateway kill -KILL "$worker"
  limit=$(( $(date +%s) + 5 ))
  while gateway test -e "/proc/$worker"; do
    [ "$(date +%s)" -lt "$limit" ] || return 1
    sleep 1
  done
}
diagnose() {
  result=$?
  trap - EXIT
  if [ "$result" -ne 0 ]; then
    gateway tail -n 70 /device/runtime.log || true
    docker logs "$front" 2>&1 | tail -n 20 || true
    docker logs "$exitnode" 2>&1 | tail -n 20 || true
  fi
  exit "$result"
}
trap diagnose EXIT

docker exec -d "$router" /fixture/wire dns 127.0.0.2:53 203.0.113.7 "$frontip"
gateway env DYNET_SPAN=198.51.100.0/24 DYNET_LOCAL=false DYNET_SOURCES="$sources" /fixture/dynet --instance dynet9 --port 15353 up --claim 203.0.113.0/24
start
before=$(witness 1400)
originalmtu=$(docker exec "$first" ip -j link show dev eth0 | jq -er '.[0].mtu')
docker exec "$first" ip link set dev eth0 mtu 1280
docker exec "$first" /fixture/wire quiet 0.0.0.0:25009 203.0.113.7:3478 1400
docker exec "$first" ip link set dev eth0 mtu "$originalmtu"
test "$(witness 1400)" = "$before"
docker exec "$first" /fixture/wire double 0.0.0.0:25000 203.0.113.7:3478 203.0.113.8:3478 "$exitip" &
a=$!
docker exec "$second" /fixture/wire round 0.0.0.0:25000 203.0.113.7:3478 "$exitip" 1400 &
b=$!
wait "$a"
wait "$b"
docker exec "$first" curl --noproxy "*" --fail --silent --show-error --max-time 5 --local-port 25000 http://203.0.113.7:3478/ >/dev/null
docker exec "$first" /fixture/wire round 0.0.0.0:25000 203.0.113.7:3478 "$exitip" 32
docker exec "$first" /fixture/wire round 0.0.0.0:25000 203.0.113.7:443 "$exitip" 1200
printf '%s\n' 'UDP/443 round trip passed. This fixture sends an opaque datagram.'
docker exec "$first" /fixture/wire fetch 203.0.113.7:3478
deadline=$(( $(date +%s) + 45 ))
until gateway grep -q 'dropped after .* quiet' /device/runtime.log; do
  [ "$(date +%s)" -lt "$deadline" ] || exit 1
  sleep 1
done
docker exec "$first" /fixture/wire round 0.0.0.0:25000 203.0.113.7:3478 "$exitip" 32

docker stop --time 2 "$exitnode" >/dev/null
before=$(witness 41)
docker exec "$first" /fixture/wire quiet 0.0.0.0:25001 203.0.113.7:3478 41
test "$(witness 41)" = "$before"
docker start "$exitnode" >/dev/null
docker exec "$exitnode" ip route replace 203.0.113.7/32 via "$sinkip"
docker exec "$exitnode" ip route replace 203.0.113.8/32 via "$sinkip"
docker exec "$second" /fixture/wire round 0.0.0.0:25002 203.0.113.8:3478 "$exitip" 48

crash
before=$(witness 42)
docker exec "$first" /fixture/wire quiet 0.0.0.0:25003 203.0.113.7:3478 42
test "$(witness 42)" = "$before"
start
docker exec "$first" /fixture/wire round 0.0.0.0:25004 203.0.113.7:3478 "$exitip" 128

crash
start
docker exec "$first" /fixture/wire crowd 203.0.113.7:3478 "$exitip"

put "$router" /device/routing.next < "$SEATRIAL_SCENE/fixture/two.envelope"
gateway mv /device/routing.next /device/routing.envelope
deadline=$(( $(date +%s) + 20 ))
until gateway cat /device/dynet9.control/checkpoint | jq -e '.applied.receipt.generation == 2' >/dev/null; do
  [ "$(date +%s)" -lt "$deadline" ] || exit 1
  sleep 1
done
before=$(witness 11)
docker exec "$first" /fixture/wire refused 0.0.0.0:25011 203.0.113.7:3478
test "$(witness 11)" = "$before"

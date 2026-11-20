set -eu
umask 077
lane=$SEATRIAL_LANE
router=$lane-alias-router
exitnode=$lane-alias-exit
client=$lane-alias-client
lan=$lane-alias-lan
image=$SEATRIAL_IMAGE
work=$SEATRIAL_WORK/alias
mkdir -m 700 "$work"
fixture=$SEATRIAL_SCENE/alias/fixture.py
sing=ghcr.io/sagernet/sing-box:v1.13.14@sha256:c8b67944345dc84a226b648a71f854818606eae0813c4e6a452f192ef821b5b8
uuid=85054868-dbcb-48d2-b8db-8e898753f9ea
owned() { docker run --detach --init --env PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin --label perish.owner=seatrial --label "perish.lane=$lane" "$@"; }
router() { docker exec "$router" "$@"; }
put() { docker exec -i "$router" sh -eu -c 'umask 077; cat > "$1"' sh "$1"; }
ipof() { docker inspect --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' "$1"; }
sixof() { docker inspect --format '{{range .NetworkSettings.Networks}}{{.GlobalIPv6Address}}{{end}}' "$1"; }
probe() { docker exec "$client" python3 /fixture.py "$@"; }
check() { probe check "$1" "$2" "$3" "$routerip"; }
docker network create --ipv6 --label perish.owner=seatrial --label "perish.lane=$lane" "$lan" >/dev/null
owned --volume "$SEATRIAL_VOLUME:/fixture/debian" --name "$router" --network "$lan" --cap-add NET_ADMIN --device /dev/net/tun --sysctl net.ipv4.ip_forward=1 --volume "$SEATRIAL_BINARY_DYNET:/dynet:ro" --volume "$SEATRIAL_BINARY_SECRETS:/secrets:ro" --volume "$fixture:/fixture.py:ro" --entrypoint sleep "$image" infinity >/dev/null
for name in "$client" "$exitnode"; do
  owned --volume "$SEATRIAL_VOLUME:/fixture/debian" --name "$name" --network "$lan" --cap-add NET_ADMIN --volume "$fixture:/fixture.py:ro" --entrypoint sleep "$image" infinity >/dev/null
done
lists=/fixture/debian/lists
docker exec -i "$router" python3 - < "$SEATRIAL_SCENE/alias/packages.py" > "$work/index.log" 2>&1 || { cat "$work/index.log"; exit 1; }
jobs=
for endpoint in "$router" "$client" "$exitnode"; do
  set -- iproute2 python3
  case "$endpoint" in
    "$router") set -- "$@" nftables ;;
    "$client") set -- "$@" systemd-resolved ;;
  esac
  docker exec "$endpoint" env DEBIAN_FRONTEND=noninteractive apt-get -o "Dir::State::Lists=$lists" -o Dir::Etc::sourcelist=/fixture/debian/sources.list -o Dir::Etc::sourceparts=- -y --no-install-recommends install "$@" > "$work/$endpoint.packages" 2>&1 &
  jobs="$jobs $!"
done
failed=0
for job in $jobs; do wait "$job" || failed=1; done
if [ "$failed" -ne 0 ]; then cat "$work"/*.packages; exit 1; fi
routerip=$(ipof "$router")
clientip=$(ipof "$client")
exitip=$(ipof "$exitnode")
exitv6=$(sixof "$exitnode")
clientv6=$(sixof "$client")
test -n "$exitv6" && test -n "$clientv6"
docker exec "$client" ip route add 203.0.113.7/32 via "$routerip"
docker exec "$exitnode" ip addr add 203.0.113.7/32 dev lo
docker exec "$client" sh -ec 'mkdir -p /etc/systemd/resolved.conf.d; install -d -o systemd-resolve -g systemd-resolve /run/systemd/resolve; printf "[Resolve]\nDNS=%s\nFallbackDNS=\nDomains=~.\nLLMNR=no\nMulticastDNS=no\nDNSSEC=no\n" "$1" > /etc/systemd/resolved.conf.d/fixture.conf; echo nameserver 127.0.0.53 > /etc/resolv.conf' sh "$routerip"
docker exec -d "$client" /usr/lib/systemd/systemd-resolved
docker exec -d "$exitnode" python3 /fixture.py echo "$exitv6"
for name in "$router" "$exitnode"; do docker exec -d "$name" python3 /fixture.py dns "$exitv6"; done
jq -n --arg uuid "$uuid" '{inbounds:[{type:"vmess",listen:"0.0.0.0",listen_port:2443,users:[{uuid:$uuid}]}],outbounds:[{type:"direct"}]}' > "$work/front.json"
owned --name "$lane-alias-front" --network "container:$exitnode" --volume "$work/front.json:/front.json:ro" "$sing" run -c /front.json >/dev/null
router sh -eu -c 'mkdir -m 700 /device; mkdir -p /etc/iproute2/rt_tables.d'
recipient=$(router /dynet identity --output /device/identity.age)
publisher=$(router /secrets signer --output /device/signer.key)
jq -n --arg recipient "$recipient" '{target:{device:"gateway0",resource:"routing",epoch:"fixture1"},recipient:$recipient,signerRef:"/device/signer.key",minimum:1,remote:{baseUrl:"http://127.0.0.1:8080",tokenRef:"/device/unused.token"}}' | put /device/declaration.json
printf '%064d\n' 0 | put /device/unused.token
jq -n --arg uuid "$uuid" --arg host "$exitip" '{version:1,upstream:"127.0.0.2",retain:86400,manifest:{version:1,nodes:[{label:"fixture/front",protocol:"vmess",server:$host,port:2443,uuid:$uuid,udp:true}],clusters:[{name:"proxy",nodes:["fixture/front"]},{name:"second",nodes:["fixture/front"]}],rules:[{suffix:"managed.test",cluster:"proxy"},{suffix:"other.test",cluster:"second"}],fallback:"direct"}}' | put /device/payload.json
router /secrets seal --declaration /device/declaration.json --generation 1 --input /device/payload.json --output /device/routing.envelope
jq -n --arg publisher "$publisher" '{version:1,target:{device:"gateway0",resource:"routing",epoch:"fixture1"},identityRef:"/device/identity.age",publisher:$publisher,minimum:1,source:{kind:"file",path:"/device/routing.envelope"}}' | put /device/bootstrap.json
run() { router env DYNET_SPAN=198.51.100.0/24 DYNET_LOCAL=false DYNET_SOURCES="$clientip/32" /dynet --instance dynet9 --port 15353 --state /device "$@"; }
run up --bare
router nft add table ip aliases
router nft 'add chain ip aliases dns { type nat hook prerouting priority dstnat - 2; policy accept; }'
router nft "add rule ip aliases dns ip daddr $routerip meta l4proto { tcp, udp } th dport 53 dnat to 198.51.100.1:15353"
start() {
  docker exec -d "$router" env DYNET_SPAN=198.51.100.0/24 DYNET_LOCAL=false DYNET_SOURCES="$clientip/32" sh -ec 'echo $$ >/device/worker; exec /dynet --instance dynet9 --port 15353 --state /device divert --bootstrap /device/bootstrap.json --seconds 0 >>/device/run.log 2>&1'
  for i in $(seq 1 15); do if check ready.test 1 0 >/dev/null 2>&1; then return; fi; sleep 1; done
  router tail -30 /device/run.log
  return 1
}
start
probe connect partial.managed.test four 203.0.113.7
probe connect normal.managed.test four 203.0.113.7
probe connect normal-second.managed.test four 203.0.113.7
check normal.edge.test 28 5
probe connect independent.direct.test six "$clientv6"
check normal.direct.test 1 5
check normal.managed.test 1 5
check reverse.direct.test 1 0
check reverse.managed.test 1 5
check reverse.direct.test 1 5
check across.managed.test 1 0
check across.other.test 1 5
check across.managed.test 28 5
check short.managed.test 1 0
check short.direct.test 1 5
sleep 3
check short.direct.test 1 0
router sh -ec 'kill -TERM "$(cat /device/worker)"; for i in $(seq 1 30); do if ! kill -0 "$(cat /device/worker)" 2>/dev/null; then exit 0; fi; sleep 0.2; done; exit 1'
start
check normal.direct.test 1 5
check normal.managed.test 1 5
probe connect independent.direct.test six "$clientv6"
printf '%s\n' 'Managed IPv4 and direct IPv6 connected with independent source witnesses; shared targets refused in both orders and after restart.'

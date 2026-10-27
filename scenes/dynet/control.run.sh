set -eu
. "$SEATRIAL_SCENE/environment.sh"
owned --detach --init --name "$trial_keeper" --network "$trial_ns" --network-alias "$trial_keeper" --entrypoint sleep "$trial_image" infinity >/dev/null
owned --detach --init --name "$trial_publisher" --network "container:$trial_keeper" --volume "$trial_publish:/publish" --volume "$trial_auth:/auth:ro" --volume "$trial_secrets:/fixture/secrets:ro" --entrypoint sleep "$trial_image" infinity >/dev/null
owned --detach --init --name "$trial_client" --network "container:$trial_keeper" --cap-add NET_ADMIN --device /dev/net/tun --volume "$trial_device:/device" --volume "$trial_dynet:/fixture/dynet:ro" --volume "$SEATRIAL_WORK/dynet-dns:/fixture/dns:ro" --entrypoint sleep "$trial_image" infinity >/dev/null
docker exec "$trial_publisher" chmod 700 /publish
cexec chmod 700 /device
cexec sh -eu -c 'apt-get -o Acquire::Retries=2 -o Acquire::http::Timeout=30 update >/dev/null; DEBIAN_FRONTEND=noninteractive apt-get -y --no-install-recommends install iproute2=6.15.0-1 >/dev/null'
cexec dpkg-query -W iproute2
cexec sh -eu -c 'mkdir -p /etc/iproute2/rt_tables.d; printf "40101 dynet9\n" > /etc/iproute2/rt_tables.d/seatrial.conf; ip tuntap add dev dynet9 mode tun; ip addr add 198.51.100.1/24 dev dynet9; ip link set dynet9 up'
trial_recipient=$(cexec /fixture/dynet identity --output /device/identity.age)
trial_publisher_key=$(pexec signer --output /publish/signer.key)
printf '%s\n' "$trial_publisher_key" | owned --rm -i --network none --volume "$trial_public:/public" --entrypoint sh "$trial_image" -eu -c 'umask 077; chmod 700 /public; cat > /public/publisher.pub'
owned --rm --network none --volume "$trial_state:/state" --volume "$trial_auth:/boot" --volume "$trial_public:/public:ro" --volume "$trial_api:/fixture/server:ro" --entrypoint sh "$trial_image" -eu -c 'chmod 700 /boot; exec /fixture/server init --state /state/estate --publisher /public/publisher.pub --admin /boot/admin.token'
owned --detach --name "$trial_server" --network "container:$trial_keeper" --volume "$trial_state:/state" --volume "$trial_api:/fixture/server:ro" --entrypoint /fixture/server "$trial_image" serve --state /state/estate --listen 0.0.0.0:8080 >/dev/null
jq -n '{baseUrl:"http://127.0.0.1:8080",tokenRef:"/auth/admin.token"}' | put "$trial_publisher" "$connection"
jq -n --arg recipient "$trial_recipient" '{target:{device:"device0",resource:"routing",epoch:"enrollment1"},recipient:$recipient,signerRef:"/publish/signer.key",minimum:1,remote:{baseUrl:"http://127.0.0.1:8080",tokenRef:"/auth/admin.token"}}' | put "$trial_publisher" "$declaration"
trial_deadline=$(( $(date +%s) + 15 ))
until pexec register --connection "$connection" --declaration "$declaration"; do [ "$(date +%s)" -lt "$trial_deadline" ] || fail 'API did not become ready'; sleep 1; done
pexec access --connection "$connection" --device device0 --output /publish/device.token
docker exec "$trial_publisher" cat /publish/device.token | put "$trial_client" /device/device.token
jq -n --arg publisher "$trial_publisher_key" '{version:1,target:{device:"device0",resource:"routing",epoch:"enrollment1"},identityRef:"/device/identity.age",publisher:$publisher,minimum:1,source:{kind:"url",baseUrl:"http://127.0.0.1:8080",bootstrap:["127.0.0.1"],tokenRef:"/device/device.token"}}' | put "$trial_client" /device/bootstrap.json
owned --detach --name "$trial_resolver" --network "container:$trial_keeper" --volume "$SEATRIAL_WORK/dynet-dns:/fixture/dns:ro" --entrypoint /fixture/dns "$trial_image" serve >/dev/null
publish 1 203.0.113.0/24
start_agent
wait_status 1 applied
dns open
cexec ip route show table dynet9 | grep -q '203.0.113.0/24'
trial_seen=$(status | jq -er .seen)
sleep 12
trial_later=$(status | jq -er .seen)
[ "$trial_later" -gt "$trial_seen" ] || fail 'device heartbeat did not advance'
docker stop --time 20 "$trial_server" >/dev/null
stop_agent
start_agent
wait_dns
checkpoint | jq -e '.applied.receipt.generation == 1 and .barred == null' >/dev/null
start_api
wait_status 1 applied
publish 2 203.0.114.0/24
wait_status 2 applied
dns open
cexec ip route show table dynet9 | grep -q '203.0.114.0/24'
if pexec publish --declaration "$declaration" --generation 1 --input /publish/payload.json > "$trial_root/rollback.out" 2>&1; then fail 'older publication was accepted'; fi
if pexec publish --declaration "$declaration" --generation 2 --input /publish/payload.json > "$trial_root/conflict.out" 2>&1; then fail 'same-generation different ciphertext was accepted'; fi
pexec revoke --declaration "$declaration" --generation 3 > "$trial_root/revoked.json"
wait_status 3 revoked
dns closed
cexec ip route show table dynet9 | grep -q '203.0.114.0/24'
docker stop --time 20 "$trial_server" >/dev/null
stop_agent
start_agent
sleep 2
dns closed
checkpoint | jq -e '.applied.receipt.generation == 3 and .barred.receipt.generation == 3' >/dev/null
start_api
publish 4 203.0.115.0/24
wait_status 4 applied
dns open
checkpoint | jq -e '.applied.receipt.generation == 4 and .barred == null' >/dev/null
pexec deny --connection "$connection" --device device0
sleep 2
dns open
pexec access --connection "$connection" --device device0 --output /publish/rotated.token
docker exec "$trial_publisher" cat /publish/rotated.token | put "$trial_client" /device/device.token
stop_agent
start_agent
wait_dns
wait_status 4 applied
. "$SEATRIAL_SCENE/file.sh"
owned --rm --network none --user 0:0 --volume "$trial_state:/server:ro" --entrypoint sqlite3 "$SEATRIAL_IMAGE_SQLITE" -readonly /server/estate/estate.db .dump > "$trial_root/server.dump"
if rg -q -F "$trial_marker" "$trial_root/server.dump"; then fail 'server store contains node plaintext'; fi
if rg -q -F 'AGE-SECRET-KEY-' "$trial_root/server.dump"; then fail 'server store contains the device identity'; fi
cexec sh -eu -c 'if grep -R -q -F "$1" /device/dynet9.control /device/file/dynet9.control; then exit 1; fi' sh "$trial_marker"
stop_agent
printf '%s\n' 'Dynet/Secrets: encrypted URL and file inputs, confirmed offline recovery, subscription/heartbeat, CAS refusal, durable revocation and higher-generation recovery passed.'

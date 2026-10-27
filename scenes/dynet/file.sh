cexec mkdir -m 700 /device/file
jq -n --arg recipient "$trial_recipient" '{target:{device:"device0",resource:"file-routing",epoch:"enrollment1"},recipient:$recipient,signerRef:"/publish/signer.key",minimum:1}' | put "$trial_publisher" /publish/file-declaration.json
jq -n --arg publisher "$trial_publisher_key" '{version:1,target:{device:"device0",resource:"file-routing",epoch:"enrollment1"},identityRef:"/device/identity.age",publisher:$publisher,minimum:1,source:{kind:"file",path:"/device/file.envelope"}}' | put "$trial_client" /device/file-bootstrap.json
pexec seal --declaration /publish/file-declaration.json --generation 1 --input /publish/payload.json --output /publish/file-one.envelope > /dev/null
docker exec "$trial_publisher" cat /publish/file-one.envelope | put "$trial_client" /device/file.envelope
stop_agent
trial_agent_state=/device/file
trial_agent_bootstrap=/device/file-bootstrap.json
start_agent
wait_checkpoint 1 1
dns open
printf '%s\n' '{"version":999,"secret":"sensitive-invalid-fixture"}' | put "$trial_publisher" /publish/invalid.json
pexec seal --declaration /publish/file-declaration.json --generation 2 --input /publish/invalid.json --output /publish/file-two.envelope > /dev/null
docker exec "$trial_publisher" cat /publish/file-two.envelope | put "$trial_client" /device/file.envelope
wait_checkpoint 2 1
dns open
cexec mkdir -m 700 /device/invalid
if cexec /fixture/dynet --instance dynet8 --state /device/invalid reach --bootstrap /device/file-bootstrap.json --cluster private --count 0 > "$trial_root/invalid.out" 2>&1; then fail 'fresh invalid encrypted business payload was accepted'; fi
if rg -q -F sensitive-invalid-fixture "$trial_root/invalid.out"; then fail 'decrypted parse error leaked input'; fi
pexec seal --declaration /publish/file-declaration.json --generation 1 --input /publish/payload.json --output /publish/file-replay.envelope > /dev/null
docker exec "$trial_publisher" cat /publish/file-replay.envelope | put "$trial_client" /device/file.envelope
sleep 2
wait_checkpoint 2 1
dns open
pexec seal --declaration /publish/file-declaration.json --generation 3 --input /publish/payload.json --output /publish/file-three.envelope > /dev/null
docker exec "$trial_publisher" cat /publish/file-three.envelope | put "$trial_client" /device/file.envelope
wait_checkpoint 3 3
dns open

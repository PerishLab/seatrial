umask 077
account=$SEATRIAL_TENCENT_ACCOUNT
principal=$SEATRIAL_TENCENT_PRINCIPAL
test "$account" != "$principal"
mkdir -p /trial/home /trial/model/providers
cd /trial/model
cat > providers/tencent.trial.toml <<MODEL
schema='hardrig.provider/v1'
id='tencent.trial'
kind='tencent'
account='$account'
principal='$principal'
credential='observer'
products=['cvm','lighthouse']
regions=['ap-hongkong']
MODEL
status=0
/opt/hardrig provider inspect tencent.trial > /trial/missing.json || status=$?
test "$status" -eq 1
jq -e '.coverage == "unknown" and .failures[0].class == "credential"' /trial/missing.json > /dev/null
test ! -e .local
mkdir -p .local/session .local/providers
cp /seed/observer.json .local/session/observer.json
chmod 600 .local/session/observer.json
printf '%s\n' 'state="/trial/model/.local/session/observer.json"' > .local/providers/observer.toml
cat > runseal.observer.toml <<PROFILE
[env.vars]
TENCENTCLOUD_STATE_FILE="local://session/observer.json"
TENCENTCLOUD_ACCOUNT_UIN="$account"
PROFILE
/opt/runseal :observer @tencent identity --json > /trial/identity.json
jq -e --arg account "$account" --arg principal "$principal" '
  .tencent.identity | .AccountId == $account and .PrincipalId == $principal and .Type == "CAMUser" and (.RequestId | length > 0)
' /trial/identity.json > /dev/null
jq -c '{stage:"runseal-caller",evidence:.}' /trial/identity.json
probe() {
  /opt/hardrig provider "$@" > /trial/last.json
  jq -e --arg account "$account" --arg principal "$principal" '
    .schema == "hardrig.provider-observation/v2" and .coverage == "complete"
    and .failures == [] and (.scopes | length > 0)
    and all(.scopes[];
      .coverage == "complete" and .failures == [] and .data.state == "known"
      and .context.snapshot == false
      and .context.identity.account == $account and .context.identity.principal == $principal
      and .context.identity.kind == "CAMUser" and .context.identity.authenticated
      and (.context.identity.request | length > 0)
      and .context.scope.authorization == $account and .context.scope.backend == "tencent"
      and (.context.scope.product | IN("cvm","lighthouse"))
      and (.context.scope.region | IN(null,"ap-hongkong"))
      and (.context.receipts | length > 0)
      and all(.context.receipts[];
        (.request | length > 0)
        and (.action | IN("DescribeRegions","DescribeZones","DescribeInstances",
          "DescribeZoneInstanceConfigInfos","DescribeImages","DescribeBlueprints","DescribeBundles"))
      )
    )
  ' /trial/last.json > /dev/null
  jq -c '{stage:"hardrig-tencent-readonly",evidence:.}' /trial/last.json
}
probe inspect tencent.trial
jq -e '[.scopes[].context.scope.product] == ["cvm","lighthouse"] and all(.scopes[]; .data.value.kind=="instances")' /trial/last.json > /dev/null
probe catalog tencent.trial --product cvm --region ap-hongkong
jq -e '[.scopes[].data.value.kind] == ["regions","zones","shapes","images"]' /trial/last.json > /dev/null
probe catalog tencent.trial --product lighthouse --region ap-hongkong
jq -e '[.scopes[].data.value.kind] == ["regions","zones","images","bundles"]' /trial/last.json > /dev/null
blueprint=$(jq -r '
  [.scopes[] | select(.data.value.kind=="images") | .data.value.items[]] |
  ((map(select(([.name,.platform,.system] | map(. // "") | join(" ")) | test("linux|ubuntu|debian";"i")))[0] // .[0]).id // empty)
' /trial/last.json)
if test -n "$blueprint"; then
  probe catalog tencent.trial --product lighthouse --region ap-hongkong --image "$blueprint"
  jq -e --arg image "$blueprint" '.scopes[] | select(.data.value.kind=="bundles") | .context.image==$image' /trial/last.json > /dev/null
fi
for path in hosts .local/ssh .local/writes .local/id_ed25519; do
  test ! -e "$path"
done
jq -cn --arg account "$account" --arg principal "$principal" --arg blueprint "$blueprint" '
  {stage:"procurement",account:$account,principal:$principal,region:"ap-hongkong",
   compatibilityBlueprint:$blueprint,price:"unknown",quota:"unknown",stock:"unknown",created:false}
'

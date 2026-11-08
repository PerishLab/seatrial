# Concord

The portable cases drive a formally resolved Concord executable. No product
crate, implementation file, test mode, live GitHub credentials or GitHub write
is used. The supplied provider fixture admits only the required read protocol.

`context` creates a private Git repository and local bare remote, then registers
the subject while foreign Git repository, index, namespace and configuration
values are inherited by Concord. Registration must identify the subject and
retain its refs and clean worktree. The case selects an isolated explicit config
and data home, checks the stable `concord.space/v1` identity and private
`workspace` controls, and confirms that neither the trial root nor the source
checkout acquires legacy Concord management entries. Identity survives registration
and audit. These assertions consume Concord's data-home contract (#451); no old
Space-selector fallback is carried by the Scene. This is the published consumer of #158.

`release` supplies an adversarial release review for the intentionally absent
`v0.0.0` distribution URI under the real Concord HTTPS authority. That URI is
fixture data, not a consumed or pinned product release. Two successful absence
observations must remain unknown. A bounded CONNECT proxy then fails the first,
second or both reads. Only the exact public authority and port are accepted;
successful connections tunnel TLS unchanged, without a certificate or trust
override. Each failed combination must retain its safe classification, never
claim a changed record or disclose the fixture's diagnostic text. No review or
closure is sent to GitHub. The case qualifies #404's negative observation path,
not release inclusion or a successful closure. The provider-only case must leave
the selected data home's `workspace` absent; explicit config/home selection does
not grant local execution state authority.

Every case owns its volume, container and proxy. No host port is published.
Concord subprocesses have a deadline, the proxy is joined, and the runner's
normal lane teardown must independently report successful cleanup. Network
failure or unavailable evidence cannot be converted into a pass.

The runner accepts the existing product map and scene format; the product is
followed through its declared channel. Runtime image and downloaded archive
identities belong in the native report. Linux qualification establishes no
Windows/macOS behavior or candidate/stable installation-content equivalence.

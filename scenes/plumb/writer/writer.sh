
mkdir -p /trial/home /trial/source
git --version
/opt/plumb --version
git init -q --bare -b main /trial/origin.git
cd /trial/source
git init -q -b main
git config user.name Seatrial
git config user.email seatrial@example.invalid
printf '%s\n' retained > retained
git add retained
git commit -q -m base
git remote add origin git@github.com:Example/probe.git
git push -q /trial/origin.git main
cat > /trial/transport <<'TRANSPORT'
#!/bin/sh
exec git-upload-pack /trial/origin.git
TRANSPORT
chmod +x /trial/transport
git config core.sshCommand /trial/transport
git config ssh.variant simple
git fetch -q origin
git --git-dir=/trial/origin.git show-ref > /trial/refs.before
git show-ref > /trial/local.before
git status --porcelain --untracked-files=all > /trial/source.before
git worktree list --porcelain > /trial/trees.before
mkdir -m 700 /trial/trust
(
  umask 077
  cd /trial/trust
  printf '%s\n' seatrial-writer-fixture > token
  openssl req -x509 -newkey rsa:2048 -noenc -days 1 -subj /CN=Seatrial-Writer-CA \
    -addext basicConstraints=critical,CA:TRUE -addext keyUsage=critical,keyCertSign \
    -keyout ca.key -out ca.pem 2> /trial/cert.log
  openssl req -new -newkey rsa:2048 -noenc -subj /CN=api.github.com \
    -keyout server.key -out server.csr 2>> /trial/cert.log
  printf '%s\n' 'subjectAltName=DNS:api.github.com' 'extendedKeyUsage=serverAuth' > server.ext
  openssl x509 -req -days 1 -CA ca.pem -CAkey ca.key -CAcreateserial \
    -in server.csr -extfile server.ext -out server.pem 2>> /trial/cert.log
) || { cat /trial/cert.log >&2; exit 1; }
python3 /scene/provider.py
cat /trial/err
grep -F 'operation writer fixture-writer[bot] (id 1), but this follow runs as foreign-operator (id 2)' /trial/err
git --git-dir=/trial/origin.git show-ref > /trial/refs.after
git show-ref > /trial/local.after
git status --porcelain --untracked-files=all > /trial/source.after
git worktree list --porcelain > /trial/trees.after
cmp /trial/refs.before /trial/refs.after
cmp /trial/local.before /trial/local.after
cmp /trial/source.before /trial/source.after
cmp /trial/trees.before /trial/trees.after
test ! -e /trial/source/auto
printf '%s\n' 'writer refused; only allowed provider reads occurred; remote refs, source and worktrees unchanged'

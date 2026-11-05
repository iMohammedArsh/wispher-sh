#!/usr/bin/env bash
# One-time: create a self-signed code-signing identity "Hams Local Signing" in
# your login keychain. Signing every build with the same identity lets macOS
# remember the Accessibility and Microphone permissions between rebuilds.
# macOS will ask for your password to trust the certificate.
set -euo pipefail
NAME="Hams Local Signing"
if security find-identity -v -p codesigning | grep -q "$NAME"; then
  echo "\"$NAME\" already exists."; exit 0
fi
work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
cat > "$work/cert.conf" <<CONF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $NAME
[ext]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
CONF
openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -config "$work/cert.conf" \
  -keyout "$work/key.pem" -out "$work/cert.pem" 2>/dev/null
pass="hams-$RANDOM"
openssl pkcs12 -export -legacy -inkey "$work/key.pem" -in "$work/cert.pem" -out "$work/id.p12" -passout "pass:$pass" 2>/dev/null \
  || openssl pkcs12 -export -inkey "$work/key.pem" -in "$work/cert.pem" -out "$work/id.p12" -passout "pass:$pass"
keychain="$HOME/Library/Keychains/login.keychain-db"
security import "$work/id.p12" -k "$keychain" -P "$pass" -T /usr/bin/codesign
security add-trusted-cert -r trustRoot -p codeSign -k "$keychain" "$work/cert.pem"
security find-identity -v -p codesigning | grep "$NAME"
echo "Done. scripts/install-macos.sh will now sign Hams with \"$NAME\"."

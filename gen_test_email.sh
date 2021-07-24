#!/usr/bin/env bash

check_bin() {
  if [[ -z "$(type -p "$1")" ]] ; then
    echo "error: $2 is required."
    exit 1
  fi
}

check_bin curl "'curl'"
check_bin mail "a BSD-like 'mail'"

curl -s https://loripsum.net/api/plaintext | \
  fold -s | \
  /usr/bin/mail -s "test mail $(date '+%d/%m %H:%M:%S')" ${USER}

echo Done.

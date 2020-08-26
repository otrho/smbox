#!/usr/bin/env bash

curl -s https://loripsum.net/api/plaintext | fold -s | /usr/bin/mail -s "test mail $(date '+%d/%m %H:%M:%S')" toby
echo Done.

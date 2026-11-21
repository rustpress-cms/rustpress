#!/usr/bin/env bash
# Runs PHP read from stdin inside the reference WordPress (WP-CLI eval), e.g.
#   echo 'echo get_option("blogname");' | dev/wp-eval.sh
exec "$(dirname "$0")/wp.sh" eval "$(cat)"
